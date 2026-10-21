# Instrucciones: integrar el cliente de escritorio con Auth Service

Eres el encargado de la siguiente etapa del cliente de escritorio de Sonimbus. Trabaja de forma autónoma; si algo es ambiguo, toma la opción más razonable, regístrala en `clients/desktop/docs/fase-auth-notas.md` y sigue. Lee este documento completo antes de empezar.

## 1. Objetivo de esta etapa

Conectar el cliente de escritorio **únicamente con lo que ya existe en el backend: Auth Service** (registro, verificación de correo y reenvío), dejando una base sólida (capa de API, navegación con parámetros, manejo de errores, hilos, configuración y pruebas) sobre la que después se conectarán Core, Files y Reports.

**Fuera de alcance (no lo hagas):**

- Nada de repositorios falsos ni datos de ejemplo para grupos, chat, proyectos, etc. Core, Files y Reports aún no existen; esas pantallas se quedan como están.
- No modifiques nada fuera de `clients/desktop/` (en particular `services/auth/`, que es módulo de @tuzc0, ni `contracts/`). Las propuestas para otros módulos van en documentos (sección 7).
- No implementes inicio de sesión real, recuperación de contraseña ni Google: Auth todavía no tiene esos endpoints (ver 4.4).
- No hagas commits ni push. El usuario revisará con `git diff`.

## 2. Contexto del proyecto

- Repositorio: monorepo `team-collaboration-platform`. Cliente en `clients/desktop/` (Maven multimódulo: `app/` y `livekit-sdk/`, este último aún vacío).
- Stack: Java 21, JavaFX 21.0.8, FXML, Maven, **sin `module-info.java`** (classpath). El pom padre fuerza Java 21 con `maven-enforcer-plugin`.
- Paquete base: `mx.uv.sonimbus.desktop`. Capas ya previstas: `ui/`, `data/api`, `data/local`, `data/realtime`, `domain/session`, `domain/outbox`, `domain/voice`.
- Los 16 FXML y sus controladores esqueleto ya existen (`ui/**`). El tema está en `resources/.../css/theme.css` (paleta con variables `-sn-*`, ver comentarios del archivo).
- Documentación que debes leer antes de programar:
  - `docs/STANDARD_DEVELOPMENT.md` (secciones 8, 9, 13, 14, 17, 19, 21, 27).
  - `docs/requirements/casos-de-uso.md` (CU-01, CU-02, CU-03, CU-05) y `docs/requirements/requisitos.md` (RF-AUTH-*, RNF-USA-01/02, RNF-SEG-04, RNF-CON-03, RNF-REN-01, RNF-MNT-02).
  - `docs/decisions/README.md` (D-06, D-07, D-11, D-12 y la tabla de decisiones pendientes).
  - `services/auth/REGISTRATION.md` y, para confirmar detalles, `services/auth/src/handlers/auth.rs`, `src/error.rs`, `src/dto/auth.rs`, `src/validators/auth_validator.rs`, `src/routes/mod.rs`. **Léelos, no los modifiques.**
  - Issues de GitHub `#14` a `#36` del repo `sonimbus-uv/team-collaboration-platform` (si `gh` está disponible: `gh issue view N`; si no, usa la tabla de la sección 7).
  - `clients/desktop/docs/vistas-javafx.md` y `vistas-notas.md` (convenciones de las vistas).

## 3. Primer paso: revisar el trabajo en curso

La rama actual (`feature/desktop-navigation`) ya tiene trabajo sin terminar del usuario. Antes de escribir código:

1. Ejecuta `git status` y `git diff main...HEAD -- clients/desktop` y lee **completos** estos archivos: `App.java`, `ui/navigation/{View,ViewManager,ContentHost,Navigable}.java`, `data/api/*.java`, las pruebas en `src/test/java/.../data/`, ambos `pom.xml` y los controladores modificados.
2. Escribe en `fase-auth-notas.md` un resumen de lo que encontraste: qué está bien, qué falta, qué cambiarías y por qué.
3. **Construye sobre ese trabajo**; no lo reescribas desde cero. Si cambias un diseño existente, justifícalo en las notas.

## 4. Qué hay que construir

### 4.1 Configuración

- URL base de Auth configurable, en este orden de prioridad: propiedad de sistema `sonimbus.auth.url` → variable de entorno `SONIMBUS_AUTH_URL` → valor por defecto `http://192.168.33.30:3001` (Auth corre en la VM de Vagrant, `SERVER_ADDR=0.0.0.0:3001`).
- Las rutas (`/auth/register`, etc.) en un solo lugar, para poder anteponer `/api/v1` cuando exista el gateway (issue #14).
- Tiempo de conexión 5 s y de petición 10 s (RNF-REN-01).
- Nada de secretos en el código.

### 4.2 Capa de API (`data/api`)

Contrato real del servicio (confírmalo leyendo el código de Auth):

| Endpoint | Cuerpo | Respuestas relevantes |
|---|---|---|
| `POST /auth/register` | `{"email","password"}` | `201` `{user_id, email, email_verified, status, verification_email_sent}` · `400` validación · `409` correo ya registrado · `429` saturado (`Retry-After`) · `500/503` |
| `POST /auth/resend-verification` | `{"email"}` | `202` siempre (exista o no la cuenta) · `429` · `500` si falla el envío |
| `POST /auth/verify-email` | `{"token"}` | `200` · `400` token inválido/vencido/usado (el escritorio **no** lo usa en esta etapa: la verificación es por enlace) |

Errores: Problem Details con `type: "about:blank"` y `detail` **en inglés**. Por eso:

- Clasifica los errores por **código HTTP + operación**, nunca por el texto de `detail`, y **nunca muestres `detail` al usuario**.
- Excepción tipada (p. ej. `ApiException` con un enum `ApiErrorKind`: `VALIDATION`, `EMAIL_TAKEN`, `RATE_LIMITED` con segundos de espera, `SERVER`, `UNAVAILABLE`, `NETWORK`, `TIMEOUT`, `UNEXPECTED`).
- Usa `java.net.http.HttpClient` (JDK) y **Jackson** para JSON (`FAIL_ON_UNKNOWN_PROPERTIES = false`, para tolerar campos nuevos).
- Las llamadas no se hacen en el hilo de JavaFX: usa `CompletableFuture`/`HttpClient.sendAsync` o `javafx.concurrent.Task`, y regresa al hilo de la UI con `Platform.runLater`.
- Nunca registres en logs ni en excepciones contraseñas ni tokens (RNF-SEG-04).

### 4.3 Navegación

- Termina `ViewManager` / `ContentHost` / `Navigable` del trabajo en curso para soportar:
  - vistas de ventana completa (auth y shell),
  - vistas dentro de `contentPane` del shell,
  - diálogos modales en un `Stage` aparte (`CREATE_GROUP`, `JOIN_GROUP`, `TASK_DETAIL`),
  - **paso de parámetros** a la vista destino (por ejemplo, el correo registrado hacia `VERIFY_ACCOUNT`), sin variables estáticas globales.
- Los controladores obtienen sus dependencias (cliente de API, navegación) por **inyección simple**: `FXMLLoader.setControllerFactory` o un pequeño contenedor de la aplicación. No uses singletons estáticos.
- Conserva la navegación de demostración de `App.java` (`-Djavafx.args=siguiente`, número o nombre de vista); el README la documenta.

### 4.4 Pantallas de autenticación

**Registro (CU-01, `RegisterController` + `register.fxml`):**

- Validación en el cliente **idéntica a la del servidor** (`auth_validator.rs`): correo con formato válido; contraseña de 8 a 128 caracteres con al menos una mayúscula, una minúscula, un dígito y un carácter no alfanumérico; confirmación igual. Pon esta lógica en una clase sin JavaFX (p. ej. `domain/session/RegistrationValidator` o `ui/auth/...Validator`) con pruebas unitarias.
- Actualiza el texto de ayuda de la contraseña en el FXML y en `accessibleHelp` para que diga la regla real.
- **Campo "Nombre completo":** Auth no lo recibe. Ocúltalo (`visible="false" managed="false"`) con un comentario `<!-- Oculto hasta resolver #14 (Auth no recibe el nombre) -->`. Su campo `@FXML` se conserva.
- Mientras la petición está en curso: deshabilita el botón, muestra un indicador y evita dobles envíos.
- Éxito (`201`): navega a `VERIFY_ACCOUNT` pasando el correo y el valor de `verification_email_sent`.
- Errores, en español y sin detalles internos:
  - `409`: "Ya existe una cuenta con este correo." más un enlace "¿No recibiste el correo de verificación?" que lleva a la pantalla de verificación con ese correo (para reenviar).
  - `400`: "Revisa el correo y la contraseña." (la validación local debería evitarlo).
  - `429`: "Demasiadas solicitudes. Intenta de nuevo en N segundos." usando `Retry-After`.
  - red, tiempo de espera o `5xx`: "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo." (RNF-CON-03).
- Reutiliza `errorBox`, `field-invalid`, `confirmPasswordErrorBox` y las clases de alerta existentes.

**Verificación (CU-02, `VerifyAccountController` + `verify-account.fxml`), adaptada al enlace:**

- El servicio envía un **enlace**, no un código. Rediseña la pantalla manteniendo el estilo actual:
  - Texto: "Enviamos un enlace de verificación a **{correo}**. Ábrelo desde tu correo para activar tu cuenta."
  - Botón "Reenviar correo" → `POST /auth/resend-verification`. Tras pulsarlo, se deshabilita **60 s** con cuenta regresiva en el texto ("Reenviar en 42 s"), igual al cooldown del servidor. Muestra `infoBox` ("Si la cuenta existe y no está verificada, te enviamos un nuevo enlace.").
  - Botón principal "Ya verifiqué mi cuenta" → `LOGIN`.
  - Enlace "Volver a iniciar sesión".
  - Si llegó `verification_email_sent=false`: aviso "No pudimos enviar el correo de verificación. Usa Reenviar correo." y el botón de reenvío habilitado desde el inicio.
- Elimina el campo de código de 6 dígitos y su `codeField` del FXML y del controlador (deja nota en `fase-auth-notas.md`: se recuperaría si #14 adopta la alternativa A).

**Inicio de sesión, recuperación y Google (CU-03, CU-05, CU-04):** Auth no tiene esos endpoints.

- Login: valida que los campos no estén vacíos y muestra en `errorBox` (con estilo informativo, no de error) "El inicio de sesión todavía no está disponible en el servidor." Deja la interfaz `AuthApi` preparada solo con lo que existe; **no inventes** firmas de login.
- Recuperar contraseña y Continuar con Google: mismo tratamiento con su propio mensaje.
- Deja `// TODO CU-03 (#35)`, `// TODO CU-05` y `// TODO CU-04` donde corresponda.

### 4.5 Accesibilidad y estilo

- Respeta las convenciones de `vistas-javafx.md`: `labelFor`, `accessibleText`, `defaultButton`, sin estilos en línea, clases nuevas al final de `theme.css` en su propia sección y usando variables `-sn-*`.
- Los mensajes de estado deben distinguirse por texto e icono, no solo por color.

## 5. Dependencias permitidas

Agrega en el pom padre (`dependencyManagement`, con versión fijada) y úsalas en `app/`:

- `com.fasterxml.jackson.core:jackson-databind`.
- `org.xerial:sqlite-jdbc`: **solo agrégala si la usas** en esta etapa (por ejemplo, para preferencias locales como el último correo usado). Si no, no la agregues y anótala como siguiente paso para la cola local (RNF-CON-01).
- Pruebas: `org.mockito:mockito-junit-jupiter`, `org.testfx:testfx-junit5` y `org.testfx:openjfx-monocle` (versión compatible con JavaFX 21) para correr pruebas de UI sin pantalla. Si TestFX no funciona en modo headless en este entorno, quédate con pruebas de lógica y anótalo.

Usa versiones estables y recientes; anota cada versión elegida en las notas. No agregues otras dependencias.

## 6. Pruebas (RNF-MNT-02)

- Unitarias de la validación de registro (casos límite: 7 y 8 caracteres, 128 y 129, sin mayúscula, sin símbolo, correo inválido, confirmación distinta).
- De la capa de API contra un servidor HTTP local de prueba (`com.sun.net.httpserver.HttpServer` del JDK): `201`, `202`, `400`, `409`, `429` con `Retry-After`, `500`, `503`, cuerpo malformado, servidor caído y tiempo agotado. Comprueba que el JSON enviado es el esperado y que la contraseña no aparece en `toString()` ni en mensajes de excepción.
- Del mapeo de errores a mensajes en español.
- Opcional, con TestFX: registro con éxito navega a verificación; error `409` muestra el mensaje.
- Mantén `ViewTest`.

## 7. Issues de Auth que afectan al escritorio: propuestas

Crea `clients/desktop/docs/issues-auth-escritorio.md`. Para cada issue: resumen, impacto en el escritorio, **propuesta concreta** (con alternativas cuando aplique) y qué cambió o cambiaría en el cliente. No modifiques `services/auth/`.

| Issue | Tema | Por qué afecta al escritorio |
|---|---|---|
| #14 | Contrato de registro y verificación entre Auth y escritorio | Decide código vs. enlace, campo nombre, política de contraseña, `type` de error estables, prefijo `/api/v1`. Propón una decisión por punto y un borrador del fragmento OpenAPI (en el `.md`, no en `contracts/`). |
| #16 | Política frente a la enumeración de cuentas | El `409` del registro revela si el correo existe; define qué mensaje muestra el cliente en cada alternativa. |
| #17 | Cuenta no verificada bloquea el correo del dueño | Qué ve el usuario y qué flujo le ofrece el cliente cuando su correo ya está "ocupado". |
| #20 | Política de contraseñas NIST y Unicode | Cambiaría la validación y la ayuda del cliente; indica qué habría que tocar. |
| #15 y #32 | Rate limiter global / 429 por saturación de hash | Cómo debe reaccionar el cliente a `429` y `Retry-After`. |
| #23 | Correo en el idioma del usuario | Propón que el cliente envíe `Accept-Language` o un campo `locale`. |
| #26 y #27 | Validación de reenvío / DTO de respuesta de verify-email | Cambian lo que el cliente recibe; propón el esquema. |
| #35 | Tokens EdDSA y sesión | Qué necesita el escritorio para el login: vigencias, renovación y **almacenamiento seguro del token** en JavaFX (decisión pendiente en `docs/decisions/`). Propón opciones. |
| #36 | Proveedor de correo definitivo | Solo anota si afecta al cliente (probablemente no). |

Al final del documento, una sección "Para discutir con @tuzc0" con las preguntas abiertas en orden de prioridad.

## 8. Documentación

- Actualiza `clients/desktop/README.md`:
  - **Estado:** qué está conectado a Auth y qué no.
  - **Ejecución:** cómo configurar la URL de Auth y cómo probar el registro de punta a punta con la VM (Auth con `EMAIL_PROVIDER=mailpit`, enlace en Mailpit `http://192.168.33.30:8025`).
  - **Navegación de demostración.**
  - **Pruebas:** cómo correrlas.
  - **Estructura de paquetes:** qué va en cada capa.
- **No edites** `README.md` de la raíz ni `docs/PLAN.md` (son de todo el equipo y van en otro PR). En `fase-auth-notas.md` agrega una sección "Cambios propuestos a README y PLAN" con el texto exacto sugerido (estado del escritorio en la Fase 5, línea de "Estado" del README raíz).

## 9. Verificación antes de terminar

1. `mvn -q -f clients/desktop/pom.xml verify` sin errores (Java 21).
2. `python3 clients/desktop/docs/check_fxml.py` con 0 errores.
3. Prueba de humo de FXML: copia `clients/desktop/docs/FxmlSmokeCheck.java` a `app/src/main/java/mx/uv/sonimbus/desktop/dev/`, ejecútala (adáptala si ahora los controladores necesitan la fábrica de controladores), corrige lo que falle y **bórrala de `src/`** al terminar.
4. Revisa que no queden contraseñas, tokens ni URLs con credenciales en el código o en los logs.

## 10. Entregables finales

- Código y pruebas en `clients/desktop/`.
- `clients/desktop/docs/fase-auth-notas.md` con:
  - revisión del trabajo en curso;
  - decisiones tomadas;
  - versiones de dependencias;
  - lo que no se pudo hacer;
  - cambios propuestos a README y PLAN;
  - pasos de prueba manual;
  - resumen final de archivos creados y modificados.
- `clients/desktop/docs/issues-auth-escritorio.md`.
- `clients/desktop/README.md` actualizado.
- Un borrador del mensaje de commit (Conventional Commits, alcance `desktop`) y del cuerpo del PR siguiendo `.github/PULL_REQUEST_TEMPLATE.md`, al final de las notas.
