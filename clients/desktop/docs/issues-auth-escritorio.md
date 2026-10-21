# Issues de Auth que afectan al cliente de escritorio: propuestas

Propuestas del cliente de escritorio para los issues abiertos de Auth Service. **No modifican `services/auth/` ni `contracts/`**: son insumos para que @tuzc0 decida y para el PR de contrato que corresponda.

- **Fuente:** issues #14 a #36 de `sonimbus-uv/team-collaboration-platform`, leídos el 2026-10-15, y el código de `services/auth/src` en `feature/desktop-navigation`.
- **Estado del cliente:** el que deja esta fase (ver [`fase-auth-notas.md`](fase-auth-notas.md)). Las referencias a clases son de `clients/desktop/app/src/main/java/mx/uv/sonimbus/desktop/`.

## Resumen

| Issue | Tema | Qué propone el escritorio | Cambio en el cliente si se acepta |
|---|---|---|---|
| #14 | Contrato de registro y verificación | Enlace (alternativa B) para el MVP; `display_name` obligatorio; un `type` por caso; el prefijo `/api/v1` lo pone el gateway | Mostrar el nombre; clasificar por `type`; `AuthPaths.PREFIX = "/api/v1"` |
| #16 | Enumeración de cuentas | Mantener el 409 del registro con límites por IP; respuestas genéricas en reenvío y recuperación | Si se elige la respuesta genérica, el registro siempre lleva a verificación |
| #17 | Cuenta no verificada que bloquea el correo | Re-registro sobre cuentas no verificadas (A), con purga a 7 días (B) como mínimo. **Riesgo nuevo detectado:** el reenvío actual permite que el dueño verifique la cuenta del atacante | Texto y flujo nuevos tras el 409 |
| #20 | Política NIST y Unicode | De acuerdo; el cliente no normaliza y cuenta la longitud igual que el servidor | `RegistrationValidator`, `AuthMessages.PASSWORD_RULE`, 3 FXML y sus pruebas |
| #15, #32 | Rate limiter y 429 por saturación | Siempre `Retry-After` en segundos, un solo `type` y límite por IP | Ninguno: el cliente ya respeta `Retry-After` |
| #23 | Idioma del correo | Encabezado `Accept-Language` | Una línea en `AuthApi.post` |
| #26, #27 | Validación del reenvío y DTO de verify-email | `AccountResponse` propio para verify-email | Separar `Account` y el resultado del registro |
| #35 | Tokens EdDSA y sesión | Access token de 15 min solo en memoria; refresh token de 30 días rotativo en el llavero del sistema | Capa de sesión nueva (`domain/session`) |
| #36 | Proveedor de correo | Sin impacto directo | Ninguno |
| (nuevo) | `EMAIL_VERIFICATION_URL` en desarrollo | Aceptar `http` para direcciones privadas cuando `EMAIL_PROVIDER=mailpit` | Ninguno; simplifica la prueba con la VM |

---

## #14 — Contrato de registro y verificación entre Auth y escritorio

**Resumen.** El issue enumera cuatro diferencias entre Auth y las pantallas del escritorio (verificación, nombre, contraseña y errores), más el prefijo `/api/v1` y la confirmación de que verificar no inicia sesión. Pide `contracts/openapi/auth.yaml` y un `type` estable por error.

**Impacto en el escritorio.** Era el bloqueo para conectar registro y verificación. Esta fase lo resolvió **del lado del cliente**, adaptándose a lo que Auth hace hoy, para no esperar al contrato. Cada punto queda listo para cambiar cuando se decida.

### Punto 1 — Verificación: código o enlace

La situación cambió desde que se abrió el issue. La alternativa B decía «hace falta una página web que hoy no existe», pero **Auth ya la tiene**: `GET /auth/verify-email?token=…` confirma y devuelve una página HTML (`REGISTRATION.md`, «La página invita a volver a Sonimbus»).

**Propuesta: alternativa B (solo enlace) para el MVP**, que es lo que el cliente ya implementa.

- Sin cambios en el servicio ni riesgo de fuerza bruta.
- La pantalla de verificación del escritorio conserva una función: dice a dónde se envió el enlace, permite reenviarlo con la espera de 60 s del servidor y lleva al inicio de sesión.
- Para una fase posterior: un *deep link* `sonimbus://email-verified` (ya previsto en `REGISTRATION.md`) para que la página devuelva al usuario a la aplicación.

Si el equipo prefiere la **alternativa A** (código de 6 dígitos además del enlace), el escritorio necesitaría:

- vigencia de 15 minutos y como máximo 5 intentos por código, después de los cuales el código se invalida;
- límite por cuenta y por IP;
- la respuesta de `POST /auth/verify-email` con `{ "email", "code" }` y su propio `type` de error (`/problems/invalid-verification-code`).

En el cliente se recuperaría el `codeField` eliminado en esta fase (queda nota en el FXML y en las notas).

### Punto 2 — Nombre completo

RF-AUTH-01 pide nombre, correo y contraseña, y D-06 dice que el perfil (nombre visible) vive en Auth.

**Propuesta:** `POST /auth/register` acepta `display_name`, obligatorio, de 1 a 80 caracteres después de recortar espacios, sin caracteres de control. Si se prefiere no romper clientes, puede ser opcional mientras dure la transición, con el correo antes de `@` como valor por defecto.

**Cambio en el cliente:** quitar `visible="false" managed="false"` de `nameField` y su etiqueta en `register.fxml`, validar en `RegistrationValidator` y enviar `display_name` en `AuthApi.RegisterRequest`.

### Punto 3 — Política de contraseña

**Propuesta:** conservar las reglas actuales hasta resolver #20. El cliente ya las replica exactamente (`domain/session/RegistrationValidator`) y la ayuda dice la regla real.

### Punto 4 — Errores con `type` estable

El cliente hoy clasifica **solo por código HTTP y operación** (`AuthApi.classify`), porque `type` es siempre `about:blank`. Funciona, pero no distingue, por ejemplo, un 400 de correo inválido de uno de contraseña inválida.

**Propuesta:** formato `/problems/<caso>` de la sección 14 de `STANDARD_DEVELOPMENT.md`, con un `detail` en inglés que no sea contrato.

| `type` | Estado | Cuándo | Extensiones |
|---|---|---|---|
| `/problems/invalid-email` | 400 | Formato de correo inválido (registro, reenvío) | — |
| `/problems/invalid-password` | 400 | La contraseña no cumple la política | `violations`: lista de `too_short`, `too_long`, `missing_uppercase`, `missing_lowercase`, `missing_digit`, `missing_symbol` |
| `/problems/invalid-display-name` | 400 | Si se adopta el punto 2 | — |
| `/problems/invalid-verification-token` | 400 | Token inválido, vencido, usado o invalidado | — |
| `/problems/email-already-registered` | 409 | Registro con un correo existente (según #16) | — |
| `/problems/too-many-requests` | 429 | Límite global o por IP (#15) y saturación del hash (#32) | Encabezado `Retry-After` en segundos |
| `/problems/malformed-request` | 400 | JSON mal formado o campos faltantes. Hoy Axum responde **422 con texto plano**, no Problem Details | — |
| `/problems/service-unavailable` | 503 | Base de datos o dependencia caída | `Retry-After` opcional |
| `/problems/internal-error` | 500 | Cualquier otro fallo | — |

**Cambio en el cliente:** `AuthApi.classify` pasaría a leer `type` primero y a usar el código HTTP solo como respaldo para respuestas sin `type` conocido. Así un servicio viejo o un proxy siguen funcionando. Con `violations`, `AuthMessages.forPassword` podría mostrar exactamente lo que dijo el servidor.

### Punto 5 — Prefijo `/api/v1`

**Propuesta:** el servicio expone `/auth/...` y **el gateway** publica `/api/v1/auth/...` (`location /api/v1/auth/ { proxy_pass http://auth:3001/auth/; }`). El contrato describe las rutas públicas, con `/api/v1`, como dice `contracts/README.md`.

**Cambio en el cliente:** `AuthPaths.PREFIX = "/api/v1"` y la URL base apuntando al gateway. Es una línea, preparada en esta fase.

### Punto 6 — `verify-email` no devuelve sesión

**Confirmado desde el cliente:** está bien. Tras verificar, «Ya verifiqué mi cuenta» lleva a `LOGIN`. El escritorio no usa `POST /auth/verify-email` porque la verificación es por enlace.

### Borrador del fragmento OpenAPI

Solo como propuesta; el archivo real va en `contracts/openapi/auth.yaml` y en su propio PR.

```yaml
openapi: 3.1.0
info:
  title: Sonimbus Auth Service
  version: 0.1.0
servers:
  - url: https://{host}/api/v1
    variables:
      host: { default: localhost }
paths:
  /auth/register:
    post:
      operationId: register
      summary: Registra una cuenta con correo y contraseña (CU-01)
      parameters:
        - $ref: '#/components/parameters/AcceptLanguage'
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/RegisterRequest' }
      responses:
        '201':
          description: Cuenta creada sin verificar. Si verification_email_sent es false, el cliente ofrece el reenvío.
          content:
            application/json:
              schema: { $ref: '#/components/schemas/RegisterResponse' }
        '400': { $ref: '#/components/responses/ValidationProblem' }
        '409': { $ref: '#/components/responses/EmailAlreadyRegistered' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '500': { $ref: '#/components/responses/InternalError' }
        '503': { $ref: '#/components/responses/ServiceUnavailable' }
  /auth/resend-verification:
    post:
      operationId: resendVerification
      summary: Reenvía el enlace de verificación (CU-02)
      description: >
        Responde 202 exista o no la cuenta, ya esté verificada o en cooldown (60 s entre envíos).
        Según #16, también responde 202 si falla el envío.
      parameters:
        - $ref: '#/components/parameters/AcceptLanguage'
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/ResendVerificationRequest' }
      responses:
        '202': { description: Solicitud aceptada. No revela si la cuenta existe. }
        '400': { $ref: '#/components/responses/ValidationProblem' }
        '429': { $ref: '#/components/responses/TooManyRequests' }
        '503': { $ref: '#/components/responses/ServiceUnavailable' }
  /auth/verify-email:
    post:
      operationId: verifyEmail
      summary: Confirma el correo con el token del enlace (CU-02)
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/VerifyEmailRequest' }
      responses:
        '200':
          description: Correo verificado. No inicia sesión.
          content:
            application/json:
              schema: { $ref: '#/components/schemas/AccountResponse' }
        '400':
          description: Token inválido, vencido, usado o invalidado.
          content:
            application/problem+json:
              schema: { $ref: '#/components/schemas/Problem' }
              example:
                type: /problems/invalid-verification-token
                title: Invalid verification token
                status: 400
        '429': { $ref: '#/components/responses/TooManyRequests' }
components:
  parameters:
    AcceptLanguage:
      name: Accept-Language
      in: header
      required: false
      description: Idioma del correo (#23). Cualquier en-* usa inglés; el valor por defecto es es-MX.
      schema: { type: string, example: es-MX }
  schemas:
    RegisterRequest:
      type: object
      required: [email, password]
      properties:
        email: { type: string, format: email, maxLength: 254 }
        password:
          type: string
          minLength: 8
          maxLength: 128
          writeOnly: true
          description: Longitud en puntos de código Unicode. Política actual hasta resolver #20.
        display_name: { type: string, minLength: 1, maxLength: 80, description: Propuesta del punto 2 }
    ResendVerificationRequest:
      type: object
      required: [email]
      properties:
        email: { type: string, format: email, maxLength: 254 }
    VerifyEmailRequest:
      type: object
      required: [token]
      properties:
        token: { type: string, pattern: '^[0-9a-f]{64}$', writeOnly: true }
    AccountResponse:
      type: object
      required: [user_id, email, email_verified, status]
      properties:
        user_id: { type: string, format: uuid }
        email: { type: string, format: email }
        email_verified: { type: boolean }
        status: { type: string, enum: [active] }
    RegisterResponse:
      allOf:
        - $ref: '#/components/schemas/AccountResponse'
        - type: object
          required: [verification_email_sent]
          properties:
            verification_email_sent: { type: boolean }
    Problem:
      type: object
      required: [type, title, status]
      properties:
        type: { type: string, example: /problems/invalid-email }
        title: { type: string }
        status: { type: integer }
        detail: { type: string, description: En inglés; no es contrato ni se muestra al usuario. }
        violations:
          type: array
          items:
            type: string
            enum: [too_short, too_long, missing_uppercase, missing_lowercase, missing_digit, missing_symbol]
  responses:
    ValidationProblem:
      description: invalid-email, invalid-password, invalid-display-name o malformed-request.
      content:
        application/problem+json:
          schema: { $ref: '#/components/schemas/Problem' }
    EmailAlreadyRegistered:
      description: Solo si #16 conserva la respuesta explícita.
      content:
        application/problem+json:
          schema: { $ref: '#/components/schemas/Problem' }
    TooManyRequests:
      description: Límite de peticiones o saturación del hash.
      headers:
        Retry-After:
          required: true
          schema: { type: integer, minimum: 1, description: Segundos }
      content:
        application/problem+json:
          schema: { $ref: '#/components/schemas/Problem' }
    InternalError:
      description: Fallo interno sin detalles.
      content:
        application/problem+json:
          schema: { $ref: '#/components/schemas/Problem' }
    ServiceUnavailable:
      description: Base de datos u otra dependencia no disponible.
      content:
        application/problem+json:
          schema: { $ref: '#/components/schemas/Problem' }
```

---

## #16 — Política frente a la enumeración de cuentas

**Resumen.** Hoy se puede saber si un correo tiene cuenta por el 409 del registro, por el 500 del reenvío cuando la cuenta existe y falla el envío, y por la diferencia de tiempos. La propuesta del issue es respuesta genérica en el reenvío y en la recuperación, y decidir en equipo para el registro.

**Impacto en el escritorio.** El registro muestra «Ya existe una cuenta con este correo.», con un enlace para reenviar la verificación. En el reenvío, un 500 se muestra como «No pudimos enviar el correo de verificación…».

**Propuesta y mensajes del cliente en cada alternativa:**

| | Alternativa A: respuesta genérica en el registro | Alternativa B: conservar el 409, mitigado (**propuesta**) |
|---|---|---|
| Servidor | El registro responde siempre 202. Si el correo ya tenía cuenta, envía un correo de «ya tienes una cuenta» | 409 explícito. Límite por IP y por correo en el gateway y en Auth (#15) |
| Cliente tras registrar | Siempre a `VERIFY_ACCOUNT` con `Delivery.UNKNOWN`: «Si el correo no tenía cuenta, te enviamos un enlace de verificación. Revisa tu correo.» | 201 → verificación. 409 → «Ya existe una cuenta con este correo.» y el enlace de reenvío (actual) |
| Experiencia | Peor: el usuario no sabe si se equivocó de correo | Clara |
| Requisito | Outbox de correos (#18) para igualar los tiempos | Rate limiting por IP (#15) |

**Propuesta del escritorio: B para el registro y A para el reenvío y la futura recuperación.** Es lo que pide RF-AUTH-01 («si el correo ya existe, el registro se rechaza»), da una experiencia comprensible (RNF-USA-01) y el riesgo baja con límites por IP.

**Qué cambia en el cliente:**

- Con **B**: nada en el registro. En el reenvío, si Auth deja de devolver 500 por falla de envío (CA-02 del issue), el mensaje `AuthMessages.EMAIL_DELIVERY` dejará de aparecer y puede quitarse. El cliente ya trata 202 como éxito neutro.
- Con **A**: `RegisterController.onRegistered` iría siempre a verificación con `VerifyAccountParams.Delivery.UNKNOWN`, se eliminarían el caso `EMAIL_TAKEN` y el enlace del aviso, y el texto de verificación sería neutro.

---

## #17 — Una cuenta no verificada bloquea el correo de su dueño

**Resumen.** Cualquiera puede registrar el correo de otra persona sin verificarlo, y el dueño real recibe 409 para siempre.

**Impacto en el escritorio.** Al recibir un 409, el cliente ofrece «¿No recibiste el correo de verificación?», que lleva al reenvío.

**Riesgo nuevo detectado al conectar el escritorio.** Ese flujo (y cualquier reenvío) manda el enlace al buzón del **dueño real**. Si el dueño lo abre, **verifica una cuenta cuya contraseña eligió el atacante**, y el atacante podrá iniciar sesión con un correo verificado ajeno en cuanto exista el login. El servicio actual no lo impide: verificar no cambia la contraseña.

**Propuesta.**

1. **Alternativa A (re-registro):** si la cuenta existente **no está verificada**, el registro sustituye la contraseña (y el `display_name`), invalida los tokens pendientes y emite uno nuevo, con bloqueo de fila. Con eso, el 409 solo aparece para cuentas verificadas y el flujo anterior deja de ser necesario.
2. **Alternativa B como mínimo inmediato:** purgar las cuentas no verificadas a los 7 días (job de #24). 24 horas coincide con la vigencia del token, pero dejaría poco margen a usuarios reales.
3. Mientras no exista A, **no verificar una cuenta cuyo último registro sea muy antiguo** sin pedir contraseña nueva. O bien, cuando exista CU-05 (alternativa C), que la recuperación marque el correo como verificado.

**Qué ve el usuario y qué cambia en el cliente:**

- Con **A**: un 201 normal para el dueño real, sin pantallas nuevas. El enlace del 409 se quita porque el 409 ya implicaría una cuenta verificada, y el texto pasa a «Ya existe una cuenta verificada con este correo. Inicia sesión o recupera tu contraseña.», con enlaces a `LOGIN` y `RECOVER_PASSWORD`.
- Solo con **B**: el texto del 409 añade «Si no creaste esta cuenta, podrás registrarte de nuevo en unos días», y conviene quitar el enlace de reenvío para no llevar al dueño a verificar la cuenta del atacante.

---

## #20 — Política de contraseñas NIST SP 800-63B y Unicode

**Resumen.** Quitar las reglas de composición, mantener la longitud (8 o 12 a 128), normalizar en NFKC antes de validar y de calcular el hash, y opcionalmente consultar Have I Been Pwned.

**Impacto en el escritorio.** La validación local y la ayuda reflejan la política actual. Si Auth cambia y el cliente no, el cliente rechazaría contraseñas válidas.

**Propuesta.** De acuerdo con el issue, con dos precisiones para los clientes:

- **El cliente no normaliza:** envía la contraseña tal cual y Auth aplica NFKC. Así hay una sola fuente de verdad y el hash coincide entre escritorio y móvil.
- **La longitud mínima se cuenta después de NFKC** en ambos lados. El cliente puede normalizar solo para contar (`java.text.Normalizer`, NFKC), sin cambiar lo que envía. Conviene decidir 8 o 12 antes de publicar el contrato.
- Si se agrega Have I Been Pwned, su rechazo necesita su propio `type` (`/problems/breached-password`) y un mensaje del cliente («Esta contraseña apareció en filtraciones conocidas. Elige otra.»).

**Qué habría que tocar en el cliente:**

| Archivo | Cambio |
|---|---|
| `domain/session/RegistrationValidator.java` | Quitar los requisitos de mayúscula, minúscula, dígito y símbolo de `passwordProblems`; contar la longitud después de NFKC; ajustar `PASSWORD_MIN_LENGTH` si pasa a 12 |
| `ui/auth/AuthMessages.java` | `PASSWORD_RULE` («De 8 a 128 caracteres.») y `forPassword` |
| `register.fxml`, `recover-password.fxml`, `profile.fxml` | `passwordHintLabel`/textos y `accessibleHelp`. `profile.fxml` todavía tiene la regla anterior a esta fase |
| `RegistrationValidatorTest`, `AuthMessagesTest`, `AuthScreensTest` | Casos de composición → longitud y NFKC |

---

## #15 y #32 — Rate limiter global y 429 por saturación del hash

**Resumen.** #15: un solo contador de 120 peticiones por minuto para todo `/auth/*` y todos los clientes; cualquiera puede bloquear a todos. #32: el registro responde 429 en cuanto no hay espacio para calcular el hash, aunque se libere a los pocos milisegundos.

**Impacto en el escritorio.** Un usuario legítimo puede recibir 429 sin haber hecho nada (#15) o en una ráfaga corta (#32).

**Cómo reacciona hoy el cliente:**

- Lee `Retry-After` en segundos (también acepta fecha HTTP) y lo convierte en `ApiErrorKind.RATE_LIMITED` con la espera.
- **Registro:** «Demasiadas solicitudes. Intenta de nuevo en N segundos.» El botón vuelve a estar disponible y no hay reintento automático, para no sumar carga.
- **Reenvío:** el mismo mensaje, y el botón queda deshabilitado con cuenta regresiva durante `Retry-After`, o 60 s si no llega.

**Propuesta para el servicio:**

1. **Siempre enviar `Retry-After` en segundos** con el 429. Ya se hace, con un valor fijo de 60; con #32, el valor puede ser corto (1–2 s) y el cliente lo respetará.
2. Un solo `type` (`/problems/too-many-requests`) para ambos casos. El cliente no necesita distinguirlos.
3. Límite **por IP** en Nginx (`limit_req_zone $binary_remote_addr`) más límites por cuenta en Redis para login y reenvío, como propone #15. El cliente no cambia.
4. Con #32, una espera de unos 2 s antes del 429 cabe en el tiempo de espera del cliente (10 s por petición, RNF-REN-01).

**Cambio en el cliente:** ninguno necesario. Opcional: un reintento automático único cuando `Retry-After ≤ 3 s`, mostrando el indicador mientras tanto.

---

## #23 — Correo en el idioma del usuario

**Resumen.** El idioma del correo está fijo en `es-MX`. Se propone tomarlo de `Accept-Language` o de un campo `locale` y conservarlo en el reenvío.

**Impacto en el escritorio.** El cliente está solo en español por ahora (los textos están en el código y en los FXML; `resources/.../i18n/` está vacío).

**Propuesta:** usar **`Accept-Language`**, el encabezado estándar de HTTP, en `register` y `resend-verification`. El campo `locale` en el cuerpo duplicaría información.

- El escritorio enviaría el idioma de la interfaz, hoy siempre `es-MX`, y no `Locale.getDefault()`: el correo debe coincidir con el idioma de la aplicación, no con el del sistema operativo.
- Que Auth guarde el idioma con la cuenta para reenvíos posteriores, como pide CA-03, y para la recuperación de contraseña.

**Cambio en el cliente:** una línea en `AuthApi.post` (`.header("Accept-Language", uiLocale)`). Cuando haya internacionalización, el idioma vendrá de la configuración de la aplicación.

---

## #26 y #27 — Validación del reenvío y DTO de respuesta de verify-email

**#26 — Resumen.** Mover la normalización y la validación del correo del reenvío al servicio. **Impacto:** ninguno visible si se conserva el 400 con formato inválido; el cliente ya lo trata como `VALIDATION` («Revisa el correo.»). **Propuesta:** que el 400 use `/problems/invalid-email`, igual que el registro (ver #14).

**#27 — Resumen.** `POST /auth/verify-email` devuelve `RegisterResponse`, con un `verification_email_sent: false` que no tiene sentido ahí. **Impacto:** el escritorio no llama a verify-email en esta fase, pero `AuthApi.verifyEmail` lo deserializa a `Account`, que incluye `verificationEmailSent`.

**Propuesta de esquema:** el de `AccountResponse` y `RegisterResponse` (`allOf`) del borrador OpenAPI de #14.

**Cambio en el cliente:** separar `data/api/Account` (cuenta) de un `RegisterResult(Account account, boolean verificationEmailSent)`. Mientras tanto no hay riesgo: Jackson tolera campos nuevos y faltantes (`FAIL_ON_UNKNOWN_PROPERTIES = false`; un booleano faltante queda en `false`).

---

## #35 — Tokens EdDSA y sesión

**Resumen.** Diseñar formato, claims, vigencias, rotación de claves y JWKS antes del login (CU-03). La tabla de decisiones pendientes (`docs/decisions/README.md`, «Tokens») incluye además el **almacenamiento seguro en JavaFX**, a cargo de @tuzc0.

**Impacto en el escritorio.** El login, la reconexión (RNF-CON-03) y todas las llamadas a Core y Files dependen de esto. El cliente dejó `LoginController` con `// TODO CU-03 (#35)` y **no inventó** la firma de login en `AuthApi`.

**Qué necesita el escritorio del contrato:**

1. **Login:** `POST /auth/login` con `{ email, password, client_type: "desktop", device_name }`. `client_type` es necesario para D-12 (una sesión por tipo de cliente) y `device_name` para RNF-SEG-05.
   - Respuesta con `access_token`, `access_expires_in`, `refresh_token` y `refresh_expires_in`.
   - Error genérico `/problems/invalid-credentials` (401), que no revela qué dato falló (RF-AUTH-02).
   - Casos propios para cuenta sin verificar y para demasiados intentos.
2. **Renovación:** `POST /auth/refresh` con rotación en cada uso y detección de reutilización (ya prevista en `auth.refresh_tokens`). Si se reutiliza o la sesión fue reemplazada (D-12: «el último inicio de sesión gana»), un `type` propio (`/problems/session-revoked`) para que el cliente lleve al login con «Tu sesión se cerró porque iniciaste sesión en otro equipo.».
3. **Cierre de sesión:** `POST /auth/logout` (RF-AUTH-03), que invalida el refresh token.
4. **Vigencias propuestas:**
   - Access token de **15 minutos**: si se filtra, sirve poco tiempo.
   - Refresh token de **30 días** con caducidad deslizante: evita pedir la contraseña cada semana en la expo y en el uso diario.
   - El cliente renueva al recibir un 401 o un minuto antes de que expire el access token.
5. **Claims útiles para el cliente:** `sub` y `sid` para la caché local por usuario (SQLite, RNF-CON-01) y para saber si la sesión cambió. El cliente **no** valida la firma; eso lo hacen los servicios (D-02).

**Almacenamiento seguro del token en JavaFX (opciones):**

| Opción | Cómo | A favor | En contra |
|---|---|---|---|
| 1. Llavero del sistema (**propuesta**) | `java-keyring` (Secret Service/libsecret en Linux, Keychain en macOS, Credential Manager en Windows) | El sistema operativo cifra y protege por usuario | Una dependencia nueva; en Linux sin sesión gráfica (VM Vagrant, CI) puede no haber Secret Service |
| 2. Solo memoria | El refresh token vive mientras la app está abierta | Nada que robar del disco; cero dependencias | Hay que iniciar sesión en cada arranque |
| 3. Archivo cifrado | AES-GCM en el directorio del usuario | Sin dependencias del sistema | La clave tiene que vivir en algún lado (disco o código): protección débil |
| 4. SQLite o `Preferences` en claro | — | Simple | Inaceptable (RNF-SEG-04) |

**Propuesta:**

- Access token **solo en memoria**.
- Refresh token en el **llavero del sistema** (opción 1).
- **Respaldo automático a la opción 2** si el llavero no está disponible, con el aviso «No se pudo guardar la sesión en este equipo; tendrás que iniciar sesión al abrir Sonimbus.».
- Nunca en logs, en `toString()` ni en mensajes de excepción, con el mismo patrón que `AuthApi.RegisterRequest`.

Va en `domain/session` (por ejemplo `SessionStore` con implementaciones para el llavero y para memoria) y se registra en `AppContext`.

---

## #36 — Proveedor de correo definitivo

**Resumen.** Elegir entre Resend, SMTP genérico u otro proveedor HTTP. **Impacto en el escritorio: ninguno directo.** El cliente no habla con el proveedor.

Dos notas indirectas:

- **El proveedor no cambia el enlace:** cualquiera debe enviar la misma `EMAIL_VERIFICATION_URL`, con HTTPS en producción y sin redirecciones de seguimiento que alteren el token. Con Resend conviene desactivar el *click tracking* para el enlace de verificación.
- **Entregabilidad:** si el correo cae en spam, la pantalla de verificación ya sugiere «Revisa tu carpeta de spam».

---

## Hallazgo adicional: `EMAIL_VERIFICATION_URL` en desarrollo con la VM

No tiene issue. Lo encontré al documentar la prueba de punta a punta.

**Resumen.** Los proveedores de Auth aceptan `http` en `EMAIL_VERIFICATION_URL` solo para `localhost` o `127.0.0.1` (`providers/mailpit.rs:23`, `providers/resend.rs:15`). En la VM de Vagrant, el enlace con `localhost` que se abre desde el navegador del anfitrión no llega a Auth, y con `http://192.168.33.30:3001` Auth no arranca.

**Impacto en el escritorio.** La prueba manual necesita un túnel SSH (`vagrant ssh -- -N -L 3001:localhost:3001`, documentado en el README del escritorio).

**Propuesta.** Con `EMAIL_PROVIDER=mailpit`, que ya es solo de desarrollo (#21), aceptar también `http` para direcciones privadas (RFC 1918, como `192.168.33.30`). Con Resend se seguiría exigiendo HTTPS.

**Cambio en el cliente:** ninguno; solo se simplificaría el README.

---

## Para discutir con @tuzc0

En orden de prioridad:

1. **#17, riesgo de seguridad:** hoy el reenvío permite que el dueño real verifique, sin saberlo, una cuenta con la contraseña de un atacante. ¿Adoptamos el re-registro sobre cuentas no verificadas (A) antes del login? Mientras tanto, ¿quito del escritorio el enlace de reenvío que aparece tras el 409?
2. **#14, punto 1:** con la página de `GET /auth/verify-email` ya implementada, ¿cerramos la verificación con **solo enlace (B)** para el MVP? El escritorio ya está hecho así; la alternativa A obligaría a rehacer la pantalla y a limitar intentos.
3. **#14, puntos 2 y 4:** ¿`display_name` obligatorio en el registro (RF-AUTH-01, D-06)? ¿Aceptas la lista de `type` y la extensión `violations`? ¿Quién escribe el PR de `contracts/openapi/auth.yaml`? El escritorio puede preparar el borrador a partir de este documento.
4. **#16:** ¿409 explícito en el registro con límites por IP (B), y 202 siempre en el reenvío, incluido cuando falla el envío? Define qué mensajes deja el escritorio.
5. **#35:** vigencias (15 min y 30 días), `client_type` y `device_name` en el login, `type` para sesión revocada, y si aceptas el llavero del sistema con respaldo en memoria para el escritorio (decisión pendiente «Tokens»).
6. **#15:** ¿límite por IP en Nginx o en Auth con Redis? El cliente solo necesita `Retry-After` en segundos.
7. **#20:** ¿cuándo entra la política nueva y con mínimo de 8 o de 12? Así se cambian juntos el escritorio y el móvil.
8. **#14, punto 5:** confirmar que el gateway agrega `/api/v1` y que el servicio se queda en `/auth`.
9. **#23:** ¿`Accept-Language` en lugar de un campo `locale`?
10. **Desarrollo:** ¿aceptar `http` con direcciones privadas en `EMAIL_VERIFICATION_URL` cuando el proveedor es Mailpit?
