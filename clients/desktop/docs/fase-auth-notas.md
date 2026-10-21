# Notas de la fase: escritorio ↔ Auth Service

Registro de trabajo de la etapa descrita en [`fase-auth-escritorio.md`](fase-auth-escritorio.md).

## 1. Revisión del trabajo en curso

Revisado el 2026-10-15 sobre `feature/desktop-navigation` (`a75aeb1`). No existe una rama `main` local; la comparación se hizo contra `origin/main` (`2ee43a9`). El trabajo del usuario está en el commit `32eff7d` (`wip(desktop): navegación entre vistas y cliente de AuthApi`).

Estado de partida comprobado:

- `mvn -q -f clients/desktop/pom.xml verify` con Java 21: **pasa** (pruebas `ApiConfigTest`, `AuthApiTest` y `ViewTest` en verde).
- `python3 clients/desktop/docs/check_fxml.py`: **16 archivos, 0 errores**.

### 1.1 Qué está bien (se conserva)

- **`ViewManager`** cubre los tres modos que pide la fase: `show` (ventana completa), `showContent`/`closeContent` (dentro del shell, a través de `ContentHost`) y `showDialog` (modal con dueño, `WINDOW_MODAL` y tema aplicado). Es un diseño pequeño y claro, y es la base que se va a extender.
- **`ContentHost`** desacopla el `ViewManager` del `contentPane` concreto de `MainController`, y **`MainController`** ya maneja encabezado de sección y elemento seleccionado (`showSection`, `selectNavItem`).
- Los diálogos (`CreateGroup`, `JoinGroup`, `TaskDetail`) se cierran con `getScene().getWindow().hide()` sin depender del `ViewManager`, y `GroupSettings` vuelve al grupo con `closeContent()`.
- **`ApiConfig`** resuelve propiedad → variable de entorno → valor por defecto, con `resolve(...)` separado para probarlo sin tocar el entorno.
- **`AuthApi`** usa `java.net.http.HttpClient` y Jackson con `SNAKE_CASE` y `FAIL_ON_UNKNOWN_PROPERTIES = false`, normaliza la barra final de la URL base y tiene un constructor de paquete para las pruebas.
- **`Account`** refleja exactamente `RegisterResponse` de Auth (`dto/auth.rs`).
- **`AuthApiTest`** ya usa `com.sun.net.httpserver.HttpServer` y cubre 201, 202 sin cuerpo, 409, 429 con `Retry-After`, 503, 500 sin Problem Details, cuerpo malformado y servidor caído. También comprueba el JSON enviado.
- **`jackson-databind`** ya está en `dependencyManagement` con versión fijada (2.22.2).

### 1.2 Qué falta o no cumple la fase

| # | Hallazgo | Dónde | Qué haré |
|---|---|---|---|
| 1 | La URL por defecto es `http://localhost:3001`; la fase pide `http://192.168.33.30:3001` (Auth en la VM de Vagrant). | `ApiConfig` | Cambiar el valor por defecto y su prueba. |
| 2 | Las rutas (`/auth/register`, etc.) están repetidas como literales dentro de `AuthApi`. | `AuthApi` | Moverlas a un solo lugar (`AuthPaths`/`ApiConfig`) con un prefijo configurable para `/api/v1` (#14). |
| 3 | El tiempo de petición es de 15 s; RNF-REN-01 pide 10 s (la conexión ya está en 5 s). | `AuthApi` | Pasar a 10 s y moverlo a la configuración. |
| 4 | `classify()` distingue los 400 **por el texto de `detail`** y `ApiException` usa `detail` como mensaje. La fase prohíbe ambas cosas: hay que clasificar por código HTTP + operación y no exponer `detail`. | `AuthApi`, `ApiException` | Clasificar solo por estado HTTP y operación, y quitar `detail` del mensaje. El comentario de `classify()` sobre `type` estables pasa a la propuesta de #14. |
| 5 | El enum `ApiError` no coincide con los casos que pide la fase. `INVALID_EMAIL`, `INVALID_PASSWORD` e `INVALID_TOKEN` dependen del texto, y faltan `SERVER`, `TIMEOUT` y `RATE_LIMITED` con segundos. Un 500 cae en `UNEXPECTED` en vez de `SERVER`. | `ApiError` | Sustituirlo por `ApiErrorKind` (`VALIDATION`, `EMAIL_TAKEN`, `RATE_LIMITED`, `SERVER`, `UNAVAILABLE`, `NETWORK`, `TIMEOUT`, `UNEXPECTED`). El cambio de nombre evita confundirlo con el `ApiError` de Auth y sigue el texto de la fase. |
| 6 | No se distingue el tiempo agotado de la falta de red: `HttpTimeoutException` es un `IOException` y acaba en `NETWORK`. | `AuthApi.post` | Capturar `HttpTimeoutException`/`HttpConnectTimeoutException` antes y mapearlas a `TIMEOUT`. |
| 7 | La API es bloqueante (documentado en la clase) y ningún controlador la usa todavía. Falta el mecanismo para llamarla fuera del hilo de JavaFX y regresar con `Platform.runLater`. | — | Mantener la API síncrona, que es fácil de probar, y agregar un pequeño ejecutor de UI (`javafx.concurrent.Task` sobre un `Executor` con hilos daemon) que inyectan los controladores. |
| 8 | No hay paso de parámetros entre vistas. `VerifyAccountController` lo dice en un TODO («ViewManager no pasa parámetros»). | `ViewManager` | Agregar `show(View, Object params)` y su equivalente para contenido y diálogos. El controlador destino recibe los parámetros con una interfaz tipada antes de mostrarse, sin estáticos. |
| 9 | No hay inyección de dependencias. `Navigable.setViewManager` se llama **después** de `initialize()`, así que `initialize()` no puede usar ni la navegación ni la API. No hay `setControllerFactory`. | `ViewManager`, controladores | Crear un contenedor pequeño (`AppContext`: configuración, `AuthApi`, ejecutor y `ViewManager`) y una fábrica de controladores que use un constructor con `AppContext` si existe, o el constructor vacío si no. Así los esqueletos de Core/Files/Reports no cambian. `Navigable` se conserva por compatibilidad mientras se migra. |
| 10 | **Se perdió la navegación de demostración** (`-Djavafx.args=siguiente`, número o nombre de vista). El WIP la sustituyó por un atajo que solo acepta `MAIN`, pero el README la sigue documentando. | `App.java` | Restaurarla sobre el `ViewManager` nuevo: ventana completa, dentro del shell o diálogo según la vista. El atajo `MAIN` queda cubierto por el nombre de vista. |
| 11 | Los controladores de auth siguen siendo esqueletos (todo es TODO). `VerifyAccountController` y `verify-account.fxml` todavía tienen el campo de código de 6 dígitos (`codeField`), pero Auth envía un enlace. | `ui/auth/*` | Implementar según la sección 4.4 de la fase. |
| 12 | No hay validación de registro en el cliente. | — | Crear `domain/session/RegistrationValidator` sin JavaFX, equivalente a `auth_validator.rs` (ver 1.3). |
| 13 | Faltan pruebas de tiempo agotado, de 400, de que la contraseña no aparezca en `toString()` ni en excepciones, y del mapeo a mensajes en español. | `AuthApiTest` | Agregarlas. |
| 14 | Faltan las dependencias de prueba (Mockito, TestFX, Monocle). | poms | Agregarlas con versión fijada (sección 5 de la fase). |
| 15 | `clients/desktop/README.md` está cortado: el bloque de código de la navegación de demostración no se cierra y falta lo que seguía. Ya estaba así en `origin/main` (#13), así que no viene de la fusión. | README | Rehacer las secciones de la sección 8 de la fase. **Hecho** (ver 2.4). |

### 1.3 Detalles del contrato real de Auth que afectan al diseño

Confirmados leyendo `services/auth/src` (sin modificarlo):

- `POST /auth/verify-email` responde `200` con el mismo cuerpo que el registro (`RegisterResponse`). `AuthApi.verifyEmail` es correcto, aunque el escritorio no lo usará en esta etapa. Se conserva porque el endpoint existe.
- `POST /auth/resend-verification` también responde **`400`** si el correo tiene formato inválido. La tabla de la fase no lo menciona; se mapeará a `VALIDATION`.
- **Dos fuentes de `429`.** Una es el limitador global (120 peticiones/min para todo `/auth/*`, #15) y la otra la saturación del hash (`RegisterError::Busy`, #32). En ambos casos Auth envía `Retry-After: 60` fijo, en segundos.
- **`500` incluye la falla de envío** del reenvío (`Delivery`). `503` solo aparece con errores de conexión a la base de datos.
- **Validación del servidor**, para replicarla en el cliente:
  - Correo: recortar y pasar a minúsculas; como máximo 254 bytes; sin espacios; exactamente una `@`. La parte local tiene de 1 a 64 bytes, sin `.` al inicio, al final ni repetido, y solo ASCII alfanumérico o `.!#$%&'*+-/=?^_\`{|}~`. El dominio lleva al menos un `.`, etiquetas de 1 a 63 caracteres alfanuméricos o `-` que no empiezan ni terminan en `-`.
  - Contraseña: de 8 a 128 **caracteres Unicode** (no bytes ni unidades UTF-16). Al menos una mayúscula y una minúscula (Unicode), un dígito **ASCII** y un carácter no alfanumérico (Unicode).
  - Equivalencias en Java: `codePointCount` para la longitud; `Character.isUpperCase/isLowerCase(int)`; dígito `'0'..'9'`. Para «no alfanumérico» no basta `Character.isLetterOrDigit`: Rust `is_alphanumeric` también considera numéricos los `Nl` y `No` (por ejemplo `²`, `Ⅻ`). Se replicará con `Character.isAlphabetic(cp) || tipo ∈ {DECIMAL_DIGIT_NUMBER, LETTER_NUMBER, OTHER_NUMBER}`.

### 1.4 Cambios de diseño respecto al WIP (justificación)

- **`ApiError` → `ApiErrorKind`, y clasificación sin `detail`.** Lo exige la fase: `detail` está en inglés y no es un contrato estable. Si se pierde la distinción entre correo inválido y contraseña inválida en un 400, no importa: la validación local evita esos 400, y el mensaje para el usuario es uno solo («Revisa el correo y la contraseña.»).
- **La API sigue síncrona.** No se reescribe a `sendAsync`, porque las pruebas existentes la usan así y la asincronía se resuelve mejor en un solo punto, el ejecutor de UI. Es el cambio mínimo sobre el trabajo existente.
- **Dependencias por fábrica de controladores, sin eliminar `Navigable`.** Se mantiene el diseño actual para las vistas que no necesitan dependencias y se agrega el constructor inyectado solo donde hace falta (auth y shell).

## 2. Decisiones tomadas

### 2.1 Configuración (sección 4.1)

- **`ApiConfig` pasa de clase estática a `record`** (`authBaseUrl`, `connectTimeout`, `requestTimeout`). Así se puede inyectar en `AuthApi` y en el futuro contenedor de la aplicación, y las pruebas pueden usar tiempos de espera cortos (lo necesitará la prueba de tiempo agotado). `ApiConfig.fromEnvironment()` conserva el orden de prioridad del WIP: propiedad `sonimbus.auth.url` → variable de entorno `SONIMBUS_AUTH_URL` → valor por defecto, que ahora es `http://192.168.33.30:3001`.
- **Tiempos de espera:** `CONNECT_TIMEOUT = 5 s` y `REQUEST_TIMEOUT = 10 s` (RNF-REN-01) están definidos en `ApiConfig` y se quitaron de `AuthApi`.
- **Rutas en `AuthPaths`** (paquete `data/api`, visibilidad de paquete), con una constante `PREFIX` vacía. Para pasar al gateway (#14) basta cambiarla a `"/api/v1"`. Es una constante y no una propiedad porque la ruta forma parte del contrato que se compila con el cliente, no de la configuración de cada equipo.
- **La URL base se valida al arrancar:** solo `http`/`https`, con host y **sin credenciales** (`user:pass@`). Los mensajes de error no repiten la URL, para no exponer credenciales por accidente (RNF-SEG-04, punto 9.4 de la fase). Las barras finales se quitan.
- `AuthApi(URI)` cambia a `AuthApi(ApiConfig)`. Como ningún controlador lo usaba todavía, solo se ajustó `AuthApiTest`.

### 2.2 Capa de API (sección 4.2)

- **`ApiErrorKind` reemplaza a `ApiError`**, con los ocho casos de la fase. Se eliminaron `ApiError.java` y `ProblemDetails.java`: al no leer `detail`, el cuerpo de error ya no se interpreta.
- **Clasificación** (`AuthApi.classify`, solo código HTTP + operación):

  | Código | `register` | `resend-verification` | `verify-email` |
  |---|---|---|---|
  | 400 | `VALIDATION` | `VALIDATION` (formato de correo) | `VALIDATION` (token inválido) |
  | 409 | `EMAIL_TAKEN` | `UNEXPECTED` | `UNEXPECTED` |
  | 429 | `RATE_LIMITED` | `RATE_LIMITED` | `RATE_LIMITED` |
  | 500 | `SERVER` | `SERVER` (falla de envío) | `SERVER` |
  | 502/503/504 | `UNAVAILABLE` | `UNAVAILABLE` | `UNAVAILABLE` |
  | otro (422, 415, 404, 3xx…) | `UNEXPECTED` | `UNEXPECTED` | `UNEXPECTED` |
  | sin respuesta | `NETWORK` / `TIMEOUT` | ídem | ídem |

  El 422 se considera `UNEXPECTED` y no `VALIDATION`: Axum lo produce cuando el JSON enviado está mal formado, que es un error del cliente. El WIP lo trataba como `INVALID_REQUEST`, y eso habría mostrado al usuario «Revisa el correo y la contraseña» por un defecto del programa.
- **`TIMEOUT`** se detecta con `HttpTimeoutException`, que incluye `HttpConnectTimeoutException`, antes del `IOException` genérico.
- **`Retry-After`** acepta segundos, como lo envía Auth, y también fecha HTTP (RFC 9110). Los valores negativos o pasados se tratan como 0, y un valor ilegible se ignora.
- **`ApiException`:**
  - el mensaje es fijo: `operación: HTTP código (CASO)` u `operación: CASO`;
  - nunca lleva `detail` ni datos enviados;
  - los constructores son públicos para poder crearla en las pruebas de la capa de UI.
- **Sin secretos en textos (RNF-SEG-04):**
  - los cuerpos se envían como `record` (`RegisterRequest`, `VerifyEmailRequest`, `ResendVerificationRequest`) cuyo `toString()` oculta la contraseña y el token;
  - si falla la serialización, la excepción se lanza **sin causa**, porque el mensaje de Jackson podría citar el cuerpo;
  - hay pruebas que revisan el mensaje, la cadena de causas y la traza completa con 400, cuerpo malformado y servidor caído.
- **Hilos:** `AuthApi` sigue siendo síncrona y se agregó `ui/common/BackgroundTasks`.
  - Ejecuta el trabajo en un pool de hilos daemon (`CompletableFuture.supplyAsync`) y entrega el resultado o la excepción original con `Platform.runLater`.
  - Los ejecutores son inyectables, así que se prueba sin iniciar JavaFX.
  - Es `AutoCloseable` para cerrarla en `App.stop()`.
- **Mensajes en español** en `ui/auth/AuthMessages` (antes `AuthErrorMessages`; se renombró al agregarle los textos de validación y de «no disponible»), una clase sin JavaFX que trabaja sobre `ApiErrorKind`.
  - Registro: los textos de la sección 4.4.
  - Reenvío: `VALIDATION` muestra «Revisa el correo.» y `SERVER` muestra «No pudimos enviar el correo de verificación…», porque en ese endpoint un 500 casi siempre es la falla de envío (`Delivery`).
  - `RATE_LIMITED` sin espera utilizable muestra «Intenta de nuevo en unos momentos», y respeta el singular «1 segundo».
  - `UNEXPECTED` muestra «Ocurrió un error inesperado. Intenta de nuevo.». La fase no lo define.

### 2.3 Navegación (sección 4.3)

- **Paso de parámetros:** `ViewManager.show`, `showContent` y `showDialog` tienen ahora una variante con `Object parameters`.
  - El controlador destino implementa `Parameterized<P>` (`parametersType()` + `setParameters(P)`). Se llama después de `initialize()` y antes de mostrar la vista.
  - Si el tipo no coincide, o la vista no recibe parámetros, falla con un mensaje claro en vez de un `ClassCastException`.
  - Sin parámetros no se llama a `setParameters`: la vista debe verse bien sin ellos, por ejemplo en la demostración.
  - Primer uso: `VerifyAccountParams(email, delivery)`. Es un `record` porque los parámetros deben ser inmutables y no viajar en variables estáticas. `delivery` tiene tres estados (ver 2.5).
- **Inyección de dependencias:** `ControllerFactory` (en `FXMLLoader.setControllerFactory`) usa el único constructor público del controlador y le pasa los servicios registrados según su tipo exacto. Si falta un servicio, el error lo nombra.
  - **`AppContext`** crea los servicios una vez (`ApiConfig`, `AuthApi`, `BackgroundTasks`, `ViewManager`) y se cierra en `App.stop()`.
  - Los 16 esqueletos siguen funcionando sin cambios, porque su constructor implícito no tiene parámetros.
  - `Navigable` se conserva para los controladores que ya lo usan. Los de auth pasarán a inyección por constructor al implementar las pantallas.
- **`ContentHost.setContent(View, Parent)`:** ahora también recibe la vista. `MainController` decide el título, el subtítulo y el elemento seleccionado en un solo `switch`, en vez de repetirlo en cada botón. Así el encabezado sale bien venga la vista de un botón del shell o de la demostración.
  - Los canales muestran «Canal de texto» y «Canal de voz» en lugar del antiguo «# general», para no meter datos de ejemplo. Queda un TODO para CU-16 y CU-22.
- **Navegación de demostración restaurada** en `DemoNavigation`, con lógica pura y probada.
  - Acepta `siguiente` (contador en `Preferences`, como antes), un número del 1 al 16 o el nombre de la vista. El nombre no distingue mayúsculas y acepta guiones.
  - Las vistas de ventana completa se muestran solas, las de contenido dentro del shell con su encabezado, y los diálogos como **modal sobre el shell**. Antes los diálogos se mostraban solos en la ventana; ahora se ven igual que en la aplicación real.
  - El título de la ventana sigue siendo «Sonimbus · N de 16 · VISTA». Un argumento desconocido imprime las opciones y abre LOGIN.
  - El atajo `MAIN` del WIP queda cubierto por el nombre de vista.
- **Verificación con JavaFX real:** una clase temporal, ya borrada, montó las 16 vistas como lo hace la demostración, sin mostrar ventanas.
  - Todas cargaron con la fábrica de controladores, y las de contenido tomaron su título.
  - El correo pasado a `VERIFY_ACCOUNT` apareció en `sentToEmailText`.
  - Nota para TestFX: las vistas de auth tienen un `ScrollPane` como raíz, y `lookup` no encuentra su contenido hasta llamar a `applyCss()`.

### 2.4 README de `clients/desktop`

- Se reescribió desde «Estado», que es donde estaba cortado. Lo anterior (responsabilidad, stack, módulos, contratos, referencias) se conserva sin cambios.
- **Estado:** una tabla por área con lo que está listo y lo que falta. Se actualizará al conectar las pantallas de registro y verificación.
- **Ejecución:**
  - Se documenta `mvn -pl app javafx:run` **sin `-am`**: con `-am`, Maven ejecuta `javafx:run` también en el pom padre y falla porque falta `mainClass` (comprobado).
  - Para cambiar la URL de Auth con Maven se indica la variable de entorno. `javafx:run` lanza la aplicación en otro proceso y no le pasa las propiedades `-D` de Maven; la propiedad se documenta para las **VM options** del IDE.
- **Prueba de punta a punta con la VM:** `EMAIL_VERIFICATION_URL` debe quedarse en `http://localhost:3001/...`. Auth solo acepta `http` para `localhost` o `127.0.0.1` (`providers/mailpit.rs:23`, `providers/resend.rs:15`), y con la IP de la VM no arranca. **Corrección:** una primera versión de este README recomendaba la IP de la VM. Para que el enlace funcione desde el navegador del anfitrión, el README indica un túnel `vagrant ssh -- -N -L 3001:localhost:3001`. Mailpit sí se ve en `http://192.168.33.30:8025` porque Compose publica su puerto en todas las interfaces. Se propone a @tuzc0 aceptar `http` para direcciones privadas en desarrollo (ver `issues-auth-escritorio.md`).
- **Otras secciones:** navegación de demostración con la tabla de las 16 vistas y su presentación, pruebas, estructura de paquetes y cómo recibe dependencias y parámetros un controlador.

### 2.5 Pantallas de autenticación (sección 4.4)

- **Validación local:** `domain/session/RegistrationValidator`, sin JavaFX, replica `auth_validator.rs` punto por punto (ver 1.3).
  - Recorte y minúsculas como en Rust. El recorte usa la propiedad Unicode White_Space y no `String.strip()`, que no quita el U+00A0.
  - Límites en bytes, contraseña contada en puntos de código y la definición de «alfanumérico» de Rust.
  - A diferencia de Auth, que solo informa el primer problema, el cliente informa **todos los requisitos que faltan** de la contraseña («La contraseña necesita al menos 8 caracteres, una mayúscula y un símbolo»), para que la ayuda sea concreta.
  - Riesgo menor: las tablas Unicode de Rust y de Java 21 pueden diferir en caracteres muy recientes.
- **Registro:**
  - El nombre está oculto con el comentario pedido y su campo `@FXML` se conserva.
  - La regla real de contraseña aparece en `passwordHintLabel` y `accessibleHelp`, y también en el paso 2 de recuperación, que tenía el mismo texto viejo. En `profile.fxml` (cambiar contraseña) **no se cambió** por estar fuera de esta fase; queda anotado en la sección 4.
  - Mientras la petición está en curso, el botón se deshabilita con indicador y el texto «Creando cuenta…», los campos dejan de ser editables y un indicador evita dobles envíos.
  - Si el usuario sale de la vista antes de la respuesta, esta se ignora, para que no navegue sola.
  - Con éxito navega a `VERIFY_ACCOUNT` con el correo que devolvió Auth, ya normalizado.
  - Los errores de los campos se muestran en `errorBox` (correo y contraseña) y en `confirmPasswordErrorBox` (confirmación), con `field-invalid` y el foco en el primer campo con error.
- **409 y el enlace de reenvío:** el enlace «¿No recibiste el correo de verificación?» va dentro de `errorBox`.
  - Dentro de un aviso de error usa el color `-sn-error`: el naranja `-sn-primary` quedaba en 4.5:1, justo en el límite.
  - **Corrección de diseño:** primero pasaba `verificationEmailSent=false` desde el 409, y la verificación habría dicho «No pudimos enviar el correo», lo cual es falso. `VerifyAccountParams` tiene ahora `Delivery { SENT, FAILED, UNKNOWN }`, con las fábricas `afterRegistration` y `forExistingAccount`.
- **Verificación:**
  - El campo de código y `codeField` se eliminaron del FXML y del controlador. Se recuperarían si #14 adopta la alternativa A (código de 6 dígitos).
  - `infoBox` pasó de `alert-success` a **`alert-info`**, porque el mensaje es neutro: no confirma que se envió nada.
  - El reenvío espera según el estado de entrega:
    - **`SENT`:** el botón empieza deshabilitado 60 s, porque el cooldown de Auth ya corre desde el registro y un reenvío inmediato no haría nada.
    - **`FAILED`:** aviso de error y reenvío habilitado de inmediato.
    - **`UNKNOWN`:** reenvío habilitado, sin aviso.
  - Tras un reenvío exitoso, el botón espera 60 s. Tras un 429, espera lo que diga `Retry-After`, o 60 s si no lo envía. Con otros errores se puede reintentar enseguida.
  - La cuenta regresiva también actualiza el `accessibleText`.
  - Sin parámetros (demostración) muestra «tu correo» y el reenvío queda deshabilitado.
- **Login, recuperación y Google:** el login valida que los campos no estén vacíos y luego muestra «El inicio de sesión todavía no está disponible en el servidor.» con estilo **informativo**.
  - `AlertBox` cambia caja, icono (`!` / `i` / `✓`), color y `accessibleText` a la vez, para que el estado no se distinga solo por el color.
  - Google (en login y registro) y la recuperación tienen su propio aviso. «Ya tengo un código» también muestra el aviso de recuperación.
  - Quedaron los TODO de CU-03 (#35), CU-04 y CU-05. `AuthApi` no tiene firma de login.
- **Inyección de dependencias:** los cuatro controladores de auth pasaron a constructor (`AuthApi`, `BackgroundTasks`, `ViewManager`) y dejaron de implementar `Navigable`. «Mostrar/Ocultar» contraseña funciona en registro y login (`PasswordVisibility`).
- **CSS:** se agregó la sección «Autenticación: conexión con Auth Service» al final de `theme.css`, con `button-spinner` y el enlace dentro de `alert-error`. Todas las clases usan variables `-sn-*` y tienen el contraste anotado.
- **Pruebas de interfaz:** `AuthScreensTest` tiene 14 pruebas con TestFX sobre Monocle sin pantalla y `AuthApi` simulado con Mockito. Cubren:
  - registro exitoso que navega con el correo;
  - 409 con su enlace hacia la verificación;
  - validación local que no llama al servidor;
  - tiempo agotado;
  - Google;
  - verificación con envío fallido y reenvío;
  - 429 con `Retry-After`;
  - verificación sin parámetros;
  - «Ya verifiqué»;
  - login vacío y login no disponible;
  - mostrar contraseña;
  - recuperación.
- **Prueba de humo de FXML (punto 9.3):** `docs/FxmlSmokeCheck.java` se **actualizó en `docs/`** para cargar con `ControllerFactory`, porque los controladores de auth ya no tienen constructor vacío. Se copió a `dev/`, se ejecutó (16/16 OK, sin avisos de CSS) y se borró de `src/`.

## 3. Versiones de dependencias

Consultadas en Maven Central el 2026-10-15; todas estables.

| Dependencia | Versión | Alcance | Nota |
|---|---|---|---|
| `com.fasterxml.jackson.core:jackson-databind` | 2.22.2 | compile | Ya estaba en el WIP. |
| `org.mockito:mockito-junit-jupiter` | 5.24.0 | test | Última versión (23/09/2026). |
| `org.testfx:testfx-junit5` | 4.0.18 | test | Última versión. |
| `org.testfx:openjfx-monocle` | 21.0.2 | test | Monocle se publica por versión mayor de JavaFX; 21.0.2 es la de JavaFX 21 y funciona con 21.0.8. |
| `org.xerial:sqlite-jdbc` | — | — | **No se agregó:** esta etapa no guarda nada localmente. Queda como siguiente paso para la cola local (RNF-CON-01) y, si se decide, el último correo usado. |

TestFX funciona sin pantalla en este entorno. Las propiedades de Monocle están en el `maven-surefire-plugin` de `app/pom.xml`.

## 4. Lo que no se pudo hacer

- **Agente de Mockito:** Mockito avisa que se adjunta solo («self-attaching») y que eso dejará de funcionar en JDK futuros. La solución recomendada es pasarlo como `-javaagent` en el `argLine` de surefire, con `maven-dependency-plugin:properties`. No se hizo para no agregar un plugin más en esta etapa. En Java 21 funciona.
- **`profile.fxml`** conserva la regla de contraseña vieja («Mínimo 8 caracteres, con al menos una letra y un número.»). Cambiar contraseña depende de Auth y de CU-07, que están fuera de esta fase; conviene alinearla cuando se implemente.
- **Prueba de punta a punta con la VM (2026-10-17): incompleta.** La VM se levantó con libvirt (rama `tech/vagrant-libvirt`), PostgreSQL, Redis y Mailpit quedaron sanos y Auth compiló, pero no arrancó por la diferencia de credenciales entre `infrastructure/.env` y `services/auth/.env` (T-02). Tras ajustar `DATABASE_URL` falta repetir la prueba y recorrer los pasos 8 a 12 de la sección 6. Obstáculos que se encontraron en el camino y que se documentaron:
  - VirtualBox 6.1.28 o posterior rechaza la IP `192.168.33.30` si no se autoriza en `/etc/vbox/networks.conf` (README raíz, rama `tech/vagrant-libvirt`).
  - La VM no traía Rust (rama `tech/provision-rust`).
  - La box de libvirt crea el usuario `vagrant` con `/bin/sh` y el `.env` de infraestructura quedaba de root (corregido en `tech/vagrant-libvirt`).

## 5. Cambios propuestos a README y PLAN

No se editaron: son de todo el equipo y van en otro PR. Texto exacto sugerido.

### `README.md` (raíz), sección «Estado»

Reemplazar la línea actual:

```markdown
🚧 Inicio de la codificación: infraestructura base y esqueleto de Auth listos. Ver [`docs/PLAN.md`](docs/PLAN.md).
```

por:

```markdown
🚧 Fase 5 en curso: infraestructura base; Auth con registro, verificación de correo y reenvío; cliente de escritorio conectado a ese flujo (registro y verificación por enlace). Ver [`docs/PLAN.md`](docs/PLAN.md).
```

### `docs/PLAN.md`, Fase 5

Reemplazar la línea del escritorio:

```markdown
- ⬜ **Escritorio** (@Maple-M136279841): inicio de sesión, lista de grupos, chat en tiempo real, cola local en SQLite, reconexión automática.
```

por:

```markdown
- 🟡 **Escritorio** (@Maple-M136279841): ✅ navegación entre vistas con parámetros e inyección de dependencias, capa de API con errores tipados, registro (CU-01) y verificación por enlace con reenvío (CU-02) conectados a Auth; ⬜ inicio de sesión (espera #35), lista de grupos, chat en tiempo real, cola local en SQLite, reconexión automática.
```

Y, si @tuzc0 está de acuerdo, actualizar la línea de Auth, que ya tiene más de lo que dice (no es parte de este PR):

```markdown
- 🟡 **Auth** (@tuzc0): ✅ esqueleto, migración inicial, *health checks*, registro con Argon2id, verificación por correo (enlace) y reenvío; ⬜ inicio y cierre de sesión, renovación de tokens EdDSA, *endpoint* de clave pública, límite de intentos fallidos.
```

## 6. Pasos de prueba manual

Desde `clients/desktop/`, con `JAVA_HOME` en Java 21.

**A. Sin backend** (la URL por defecto no responde: sirve para probar errores):

1. `mvn -pl app javafx:run` → se abre el inicio de sesión.
2. Pulsa «Iniciar sesión» con los campos vacíos → aviso rojo «Escribe tu correo y tu contraseña.» y campos marcados. Escribe correo y contraseña y vuelve a pulsar → aviso informativo (icono «i») «El inicio de sesión todavía no está disponible en el servidor.». «Continuar con Google» → su propio aviso.
3. «Mostrar» en la contraseña → se ve el texto y el botón dice «Ocultar».
4. «¿Olvidaste tu contraseña?» → «Enviar código» vacío marca el campo; con un correo → «La recuperación de contraseña todavía no está disponible en el servidor.».
5. «Crear cuenta» → no aparece «Nombre completo»; la ayuda dice «De 8 a 128 caracteres, con al menos una mayúscula…».
6. Correo `ana@`, contraseña `corta`, confirmación `otra` → «El correo no tiene un formato válido.» y «La contraseña necesita al menos 8 caracteres, una mayúscula, un número y un símbolo…»; bajo la confirmación «Las contraseñas no coinciden.». Al editar un campo, su marca desaparece.
7. Datos válidos (`ana@uv.mx`, `ValidPass123!` dos veces) → el botón muestra el indicador y «Creando cuenta…»; a los 5 s aproximadamente (sin la VM, se agota el tiempo de conexión) → «No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.» y el botón vuelve a estar disponible.

**B. De punta a punta con la VM:** seguir «Probar el registro de punta a punta con la VM» del README. Además:

8. Registro correcto → pantalla de verificación con el correo en negritas y «Reenviar en 60 s» contando hacia atrás. El correo llega a Mailpit (`http://192.168.33.30:8025`); con el túnel abierto, el enlace muestra la página de Auth y la cuenta queda verificada.
9. Al terminar la cuenta regresiva, «Reenviar correo» → aviso informativo «Si la cuenta existe y no está verificada, te enviamos un nuevo enlace.» y otra cuenta regresiva de 60 s.
10. Registrar otra vez el mismo correo → «Ya existe una cuenta con este correo.» con el enlace «¿No recibiste el correo de verificación?» → pantalla de verificación sin aviso de error y con el reenvío disponible.
11. «Ya verifiqué mi cuenta» → inicio de sesión.
12. Detener Auth con la app abierta y reenviar → mensaje de conexión, sin cerrar la aplicación (RNF-CON-03).

**C. Navegación de demostración:** `mvn -pl app javafx:run -Djavafx.args=siguiente` varias veces (o `=9` para un diálogo sobre el shell, `=12` para una sección con su título).

## 7. Archivos creados y modificados

Todo dentro de `clients/desktop/`. Rutas Java relativas a `app/src/main/java/mx/uv/sonimbus/desktop/` (y `app/src/test/java/...` para pruebas).

**Creados**

| Archivo | Para qué |
|---|---|
| `AppContext.java` | Crea los servicios y los registra en la fábrica de controladores |
| `DemoNavigation.java` | Navegación de demostración (restaurada, ahora probada) |
| `data/api/ApiErrorKind.java` | Casos de error que distingue la interfaz |
| `data/api/AuthPaths.java` | Rutas de Auth con prefijo común |
| `domain/session/RegistrationValidator.java` | Validación del registro idéntica a Auth |
| `ui/auth/AuthMessages.java` | Textos en español (errores, validación, avisos) |
| `ui/auth/VerifyAccountParams.java` | Parámetros de la verificación (`Delivery`) |
| `ui/common/AlertBox.java`, `BusyButton.java`, `PasswordVisibility.java` | Avisos, botón ocupado y mostrar contraseña |
| `ui/common/BackgroundTasks.java` | Trabajo fuera del hilo de JavaFX |
| `ui/navigation/ControllerFactory.java`, `Parameterized.java` | Inyección por constructor y parámetros entre vistas |
| Pruebas: `DemoNavigationTest`, `domain/session/RegistrationValidatorTest`, `ui/auth/AuthMessagesTest`, `ui/auth/AuthScreensTest`, `ui/common/BackgroundTasksTest`, `ui/navigation/ControllerFactoryTest`, `ui/navigation/ParameterDeliveryTest` | |
| `docs/fase-auth-notas.md`, `docs/issues-auth-escritorio.md` | Estas notas y las propuestas para Auth |

**Modificados**

| Archivo | Cambio |
|---|---|
| `pom.xml`, `app/pom.xml` | Mockito, TestFX y Monocle; surefire sin pantalla |
| `App.java` | Usa `AppContext` y `DemoNavigation`; cierra recursos en `stop()` |
| `data/api/ApiConfig.java` | `record` con URL de la VM, tiempos 5/10 s y validación de la URL |
| `data/api/ApiException.java`, `AuthApi.java` | Clasificación sin `detail`, `TIMEOUT`, `Retry-After`, cuerpos sin secretos |
| `ui/navigation/ViewManager.java`, `ContentHost.java` | Fábrica de controladores, parámetros, `setContent(View, Parent)` |
| `ui/shell/MainController.java` | Encabezado de sección según la vista |
| `ui/auth/LoginController.java`, `RegisterController.java`, `VerifyAccountController.java`, `RecoverPasswordController.java` | Pantallas de la sección 4.4; inyección por constructor |
| `ui/auth/{login,register,verify-account,recover-password}.fxml` | Ver 2.5 (nombre oculto, regla real, enlace del 409, verificación por enlace, `fx:id` de iconos) |
| `css/theme.css` | Sección «Autenticación: conexión con Auth Service» al final |
| Pruebas `ApiConfigTest`, `AuthApiTest` | Nuevos casos |
| `README.md` | Estado, ejecución, prueba con la VM, demostración, pruebas, paquetes |
| `docs/FxmlSmokeCheck.java` | Carga con `ControllerFactory` |

**Eliminados:** `data/api/ApiError.java` y `data/api/ProblemDetails.java` (sustituidos por `ApiErrorKind`; ver 2.2).

**Verificación final:** `mvn -q -f clients/desktop/pom.xml verify` con Java 21 pasa (131 pruebas, 0 fallos); `python3 clients/desktop/docs/check_fxml.py` da 0 errores; `FxmlSmokeCheck` carga las 16 vistas (copiado a `dev/`, ejecutado y borrado); sin contraseñas, tokens ni URLs con credenciales en el código ni en los logs.

## 8. Borrador de commit y PR

### Mensaje de commit

```text
feat(desktop): conectar registro y verificación con Auth Service

- Capa de API de Auth con configuración (sonimbus.auth.url /
  SONIMBUS_AUTH_URL), rutas centralizadas, tiempos de 5/10 s y errores
  tipados (ApiErrorKind) clasificados por código HTTP y operación.
- Navegación con parámetros entre vistas, diálogos modales e inyección
  de dependencias por constructor (ControllerFactory, AppContext).
- Registro (CU-01) con validación idéntica a auth_validator.rs y
  mensajes en español; verificación por enlace (CU-02) con reenvío y
  espera de 60 s.
- Login, recuperación y Google avisan que aún no están disponibles.
- Restaura la navegación de demostración (-Djavafx.args).
- Pruebas: API contra HttpServer local, validación, mensajes y
  pantallas con TestFX sin pantalla (Monocle).
- Documentación: README del escritorio, notas de la fase y propuestas
  para los issues de Auth (#14, #16, #17, #20, #35...).

Refs #14
```

(Agregar al final las líneas de atribución que correspondan al hacer el commit.)

### Cuerpo del PR (según `.github/PULL_REQUEST_TEMPLATE.md`)

````markdown
# Pull Request

## Descripción

Conecta el cliente de escritorio con lo que Auth Service ofrece hoy: registro, verificación de correo por enlace y reenvío. Deja la base (capa de API, navegación con parámetros, inyección de dependencias, hilos y pruebas) para conectar después Core, Files y Reports. Login, recuperación y Google muestran que aún no están disponibles.

---

## Tipo de cambio

- [x] Nueva funcionalidad
- [x] Pruebas
- [x] Documentación

---

## Módulo afectado

- [x] Autenticación
- [x] Cliente escritorio

---

## Issue relacionado

```text
Refs #14
```

Requisito relacionado:

```text
RF-AUTH-01, RF-AUTH-07
RNF-USA-01, RNF-USA-02, RNF-SEG-04, RNF-CON-03, RNF-REN-01, RNF-MNT-02
```

---

## Cambios realizados

- `data/api`: `ApiConfig` (URL configurable, 5/10 s), `AuthPaths` (prefijo listo para `/api/v1`), `AuthApi` con errores tipados por código HTTP y operación (nunca por `detail`), `Retry-After` y sin secretos en mensajes.
- `ui/navigation`: parámetros entre vistas (`Parameterized`), `ControllerFactory` con inyección por constructor, `AppContext`; navegación de demostración restaurada.
- Registro (CU-01): validación idéntica a `auth_validator.rs`, nombre oculto hasta #14, indicador y sin dobles envíos, mensajes para 400/409/429/red/5xx.
- Verificación (CU-02): rediseñada para el enlace; reenvío con espera de 60 s, aviso si el correo no salió.
- Login, recuperación y Google: validan campos y avisan que no están disponibles.
- Pruebas (131) y documentación (`README`, `docs/fase-auth-notas.md`, `docs/issues-auth-escritorio.md`).

---

## Criterios de aceptación

- [x] CA-01: Un registro válido llega a Auth y lleva a la pantalla de verificación con el correo.
- [x] CA-02: Los errores se muestran en español y sin detalles internos (409, 400, 429 con espera, red, tiempo agotado, 5xx).
- [x] CA-03: El reenvío funciona y respeta la espera de 60 s del servidor.

---

## Pruebas realizadas

### Unitarias
- [x] Ejecutadas correctamente

### Integración
- [x] Ejecutadas correctamente (capa de API contra un `HttpServer` local; pantallas con TestFX + Monocle)

### Manuales
- [ ] Ejecutadas correctamente

Pasos principales:

```text
1. Levantar Auth en la VM con EMAIL_PROVIDER=mailpit (ver README del escritorio) y el túnel SSH al puerto 3001.
2. mvn -pl app javafx:run → Crear cuenta → verificar el correo en Mailpit (http://192.168.33.30:8025).
3. Probar reenvío, correo repetido (409) y Auth detenido (mensaje de conexión).
```

---

## Evidencia

`mvn verify`: 131 pruebas, 0 fallos. `check_fxml.py`: 16 archivos, 0 errores. `FxmlSmokeCheck`: 16/16 vistas cargan.

---

## Cambios en API

- [x] No existen cambios.
- [x] Los clientes afectados fueron revisados.

---

## Base de datos

- [x] No existen cambios.

---

## Seguridad

- [x] No se introdujeron credenciales o secrets.
- [x] Las entradas del usuario son validadas.
- [x] No se exponen datos sensibles en logs.
- [x] No se exponen detalles internos innecesarios.
- [x] Se revisaron nuevas dependencias.

---

## Docker e infraestructura

- [x] No aplica.

---

## Compatibilidad

### Escritorio
- [x] JavaFX

### Móvil
- [x] No aplica

### Backend
- [x] No aplica

---

## Impacto

### ¿El cambio rompe compatibilidad?

- [x] No

### ¿Requiere cambios en otro servicio?

- [x] No

---

## Riesgos o limitaciones conocidas

```text
- El escritorio se adaptó a Auth tal como está hoy; las decisiones de #14 (código o enlace, nombre, type de errores, /api/v1) pueden requerir ajustes menores, ya preparados.
- #17: el reenvío tras un 409 puede llevar al dueño real a verificar una cuenta creada por otra persona; ver docs/issues-auth-escritorio.md.
- Mockito se adjunta solo (aviso del JDK); funciona en Java 21.
- profile.fxml conserva la regla de contraseña anterior (fuera de esta fase).
```

---

## Definition of Done

- [x] Funcionalidad implementada.
- [x] Criterios de aceptación cumplidos.
- [x] Código compila/construye.
- [x] Pruebas exitosas.
- [x] Seguridad revisada.
- [x] API/documentación actualizada.
- [ ] CI exitoso.
- [ ] Code Review completado.

---

## Notas para el revisor

```text
Empezar por docs/fase-auth-notas.md (revisión del WIP y decisiones) y docs/issues-auth-escritorio.md (preguntas para @tuzc0). Los cambios propuestos al README raíz y al PLAN están en la sección 5 de las notas y van en otro PR.
```
````
