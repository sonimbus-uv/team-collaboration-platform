# Auth de escritorio

El módulo integra registro, verificación, login, logout, recuperación y cambio de
contraseña, autenticación con Google y vinculación de Google desde el perfil.

## Arquitectura

La dependencia sigue `ui → domain ← data`. `AppContext` compone los servicios y
`ControllerFactory` entrega sus dependencias por constructor.

| Capa | Ubicación | Responsabilidad |
|---|---|---|
| Presentación | `ui/auth/{registration,login,recovery,common}`, `ui/profile` | Formularios, mensajes y navegación. |
| Dominio | `domain/{registration,session,password,google}` | Casos de uso, validaciones, sesión y gateways. |
| Datos | `data/api/auth/{registration,session,password,google}` | Adaptadores HTTP y DTO por funcionalidad. |

Los servicios dependen de interfaces Gateway; los adaptadores HTTP las implementan.
`AuthApi` centraliza el transporte. Los controladores consumen servicios de dominio,
sin llamadas directas a `AuthApi`. `BackgroundTasks` ejecuta las operaciones fuera
del hilo de JavaFX. La autorización definitiva pertenece al backend.

## Comportamiento fundamental

| Servicio | Responsabilidad |
|---|---|
| `RegistrationService` | Registro, verificación por código y reenvío. |
| `SessionService` | Login, tokens en memoria, renovación y logout. |
| `PasswordService` | Recuperación, restablecimiento y cambio autenticado de contraseña. |
| `GoogleService` | Autenticación y vinculación explícita con PKCE y retorno del navegador. |

El registro devuelve un intento pendiente: `RegistrationAttempt` es el DTO y
`PendingRegistration` el modelo de dominio. La verificación usa ocho dígitos ASCII.
El reenvío reemplaza el código sin ampliar el vencimiento y respeta `Retry-After`.
La contraseña no se traslada entre vistas; verificar la cuenta no inicia sesión.

La sesión permanece en memoria, sin restauración desde disco. `accessToken()` renueva
el acceso cuando quedan 30 segundos o menos y serializa las operaciones para evitar
renovaciones simultáneas. Un fallo de renovación invalida la sesión, salvo las
categorías de servicio no disponible y límite de solicitudes. El logout elimina el
estado local después del éxito remoto; cerrar el cliente no ejecuta logout remoto.

La recuperación utiliza un intento y un código de ocho dígitos. El cambio desde el
perfil requiere contraseña actual y nueva confirmada. Un cambio exitoso o una
mutación con resultado incierto invalida la sesión local; esta última no debe
repetirse automáticamente.

Google permite un flujo simultáneo y su cancelación evita instalar sesiones tardías.
La vinculación requiere una sesión y puede exigir reautenticación. La coincidencia
de correos no vincula cuentas automáticamente. Los secretos del proveedor y la
validación de identidad permanecen en Auth.

En Windows, `DesktopGoogleBrowser` registra `sonimbus://auth/google`.
`GoogleCallbackLauncher` entrega el retorno mediante `GoogleCallbackChannel`, con
loopback y endpoints privados en `~/.sonimbus/google-callbacks`.
La ventana de autorización solicita su cierre al recibir el retorno; el login
depende del intercambio final con Auth. `SingleInstance` conserva un único cliente
por directorio de usuario mediante bloqueo y activación local en `~/.sonimbus`.

`ViewManager` comprueba la sesión al abrir vistas protegidas. El modo de demostración
omite ese control visual, sin conceder autorización en el servidor.

## Errores y diagnóstico

Los adaptadores traducen `ApiException` a errores de su funcionalidad; los servicios
los propagan y los controladores presentan mensajes junto al formulario.
`AuthLog` utiliza `java.util.logging` y escribe en `~/.sonimbus/logs/auth-%g.log`,
con tres archivos rotativos de aproximadamente 1 MiB y consola como alternativa.
Registra operación, estado HTTP, categoría de fallo y etapas de Google, sin
contraseñas, tokens, códigos, cuerpos HTTP ni stack traces.

## Mantenimiento y pruebas

Los cambios conservan la separación por funcionalidad, la inyección por constructor
y el cierre explícito de recursos. Las validaciones compartidas permanecen en el
dominio y los mensajes en presentación. Los métodos mantienen responsabilidades
concretas; las solicitudes con muchos datos se agrupan en objetos.

Las rutas y respuestas se contrastan con el
[contrato público de Auth](../../../contracts/auth/public-api.md).
La ejecución y configuración se concentran en el [README del cliente](../README.md);
la apariencia se describe en [temas y paleta](temas-y-paleta.md).

Desde `clients/desktop`, con Java 21 y Maven:

```bash
mvn -pl app verify
```

Las pruebas cubren servicios, adaptadores HTTP y pantallas con TestFX y Monocle.
No requieren Auth ni Google reales. La revisión manual complementaria verifica
correo, OAuth, retorno y ventana del navegador en Windows, además de fallos de red.
