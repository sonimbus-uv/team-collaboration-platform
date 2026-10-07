# Sonimbus — Requisitos y restricciones

Requisitos funcionales, no funcionales y restricciones de la especificación AR02, con el módulo responsable de cada uno y actualizados a las decisiones de [`docs/decisions/`](../decisions/). Los casos de uso que los realizan están en [`casos-de-uso.md`](casos-de-uso.md).

## Requisitos funcionales

### Identidad y acceso — Auth

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-AUTH-01 | CU-01 | El sistema permitirá que un visitante registre una cuenta con nombre, correo electrónico y contraseña. | Alta | La cuenta se crea con datos válidos; si el correo ya existe, el registro se rechaza. |
| RF-AUTH-02 | CU-03 | El sistema permitirá que un usuario inicie sesión mediante correo electrónico y contraseña. | Alta | Con credenciales válidas se concede acceso; con credenciales inválidas se rechaza sin revelar cuál dato falló. |
| RF-AUTH-03 | CU-06 | El sistema permitirá cerrar sesión desde el cliente utilizado. | Alta | Al cerrar sesión, el token de renovación queda invalidado y no puede usarse nuevamente. |
| RF-AUTH-04 | CU-05 | El sistema permitirá recuperar la contraseña mediante un código o enlace de un solo uso enviado al correo registrado. | Media | El código o enlace caduca, no puede reutilizarse y permite definir una nueva contraseña. |
| RF-AUTH-05 | CU-07 | El sistema permitirá que el usuario gestione los datos básicos de su perfil. | Media | El usuario puede actualizar nombre visible, foto (de un paquete predefinido) y contraseña, y los cambios se reflejan en sus grupos. |
| RF-AUTH-06 | CU-04 | El sistema permitirá que un visitante inicie sesión mediante su cuenta de Google utilizando OAuth. | Media | El backend valida el token de identidad emitido por Google antes de conceder acceso. Si el correo no tiene cuenta, se crea una; si ya existe, se vincula sin duplicarla. Si el usuario cancela o Google rechaza la autenticación, no se crea ninguna sesión. |
| RF-AUTH-07 | CU-02 | El sistema enviará un código o enlace de verificación al correo registrado para confirmar la cuenta. | Media | El código o enlace caduca y no puede reutilizarse. Al usarse correctamente, la cuenta queda marcada como verificada. |

### Grupos y permisos — Core

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-GRP-01 | CU-08 | El sistema permitirá crear grupos de trabajo. | Alta | Un usuario autenticado puede crear un grupo y queda registrado como su administrador. |
| RF-GRP-02 | CU-10 | El sistema permitirá gestionar invitaciones para unirse a grupos. | Alta | Administradores y moderadores crean y revocan invitaciones. Las vigentes permiten el ingreso; las vencidas, revocadas o agotadas se rechazan. |
| RF-GRP-03 | CU-11 | El sistema permitirá que un usuario se una a un grupo mediante una invitación válida. | Alta | El usuario queda agregado al grupo sin duplicar membresías. |
| RF-GRP-04 | CU-14 | El sistema permitirá asignar roles y permisos dentro de un grupo. | Alta | Las acciones se autorizan de acuerdo con el rol vigente del usuario. |
| RF-GRP-05 | CU-15 | El sistema permitirá a moderadores y administradores realizar acciones de moderación sobre miembros del grupo. | Media | Un usuario expulsado o bloqueado pierde el acceso al grupo según la acción aplicada. |
| RF-GRP-06 | CU-13 | El sistema permitirá crear, modificar y eliminar canales de texto y de voz dentro de un grupo. | Alta | Los cambios en canales se guardan y se reflejan para los miembros del grupo. |
| RF-GRP-07 | CU-09 | El sistema permitirá al administrador modificar, archivar o eliminar el grupo. | Media | Solo el administrador puede aplicar estos cambios, y se reflejan para todos los miembros. |
| RF-GRP-08 | CU-12 | El sistema permitirá que un miembro abandone el grupo. | Media | El usuario pierde el acceso a los canales del grupo. Si es el único administrador, el sistema exige transferir el rol antes de salir. |

### Mensajería y archivos — Core y Files

| ID | CU | Requisito | Prioridad | Módulo | Criterio de aceptación |
|---|---|---|---|---|---|
| RF-MSG-01 | CU-16 | El sistema permitirá enviar mensajes de texto en canales. | Alta | Core | El mensaje se almacena y se entrega a los miembros con acceso al canal. |
| RF-MSG-02 | CU-20 | El sistema permitirá consultar el historial de mensajes de un canal. | Alta | Core | El historial se muestra de forma ordenada y puede cargarse por páginas. |
| RF-MSG-03 | CU-16 | El sistema deberá evitar duplicados al reenviar mensajes después de una reconexión. | Alta | Core | Un mensaje reenviado no se guarda más de una vez. |
| RF-MSG-04 | CU-23 | El sistema permitirá compartir imágenes y archivos dentro de un canal. | Alta | Files + Core | El archivo se almacena correctamente y queda asociado a un mensaje visible para los miembros autorizados. |
| RF-MSG-05 | CU-19 | El sistema permitirá editar y eliminar mensajes propios. | Media | Core | Solo el autor puede modificar o eliminar sus mensajes, y el cambio se refleja en el canal. |
| RF-MSG-06 | CU-17 | El sistema permitirá responder mensajes específicos. | Media | Core | La respuesta conserva la referencia al mensaje original. |
| RF-MSG-07 | CU-18 | El sistema permitirá reaccionar a mensajes. | Media | Core | El usuario puede agregar o retirar una reacción sin generar duplicados. |
| RF-MSG-08 | CU-24 | El sistema permitirá consultar y descargar los archivos compartidos en un grupo. | Media | Files | El miembro ve solo los archivos de los canales a los que tiene acceso y puede descargarlos. |

### Proyectos y tareas — Core

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-PRJ-01 | CU-25 | El sistema permitirá gestionar proyectos básicos dentro de un grupo. | Media | El administrador crea, modifica y archiva proyectos; los miembros los consultan. |
| RF-PRJ-02 | CU-26, CU-27, CU-28 | El sistema permitirá gestionar tareas asociadas a proyectos del grupo. | Alta | Los miembros crean tareas, el administrador las asigna y el responsable o el administrador cambian su estado, sin sobrescribir cambios concurrentes sin aviso. |

### Reportes — Reports

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-REP-01 | CU-29 | El sistema permitirá consultar reportes básicos de actividad del grupo. | Media | El administrador puede consultar información resumida de mensajes, participación o estado de tareas. |
| RF-REP-02 | CU-30 | El sistema permitirá exportar reportes en PDF o CSV. | Baja | El archivo exportado contiene la información mostrada en el reporte consultado. |

### Notificaciones — Core

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-NOT-01 | CU-31 | El sistema permitirá mostrar notificaciones internas sobre eventos relevantes. | Media | El usuario recibe avisos sobre eventos como respuestas, tareas asignadas o cambios importantes dentro del grupo. |

### Voz — Core + LiveKit (MVP según D-01)

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-VOZ-01 | CU-22 | El sistema permitirá la participación en canales de voz. | Media | El usuario con permisos puede unirse, escuchar y transmitir audio en un canal de voz. |
| RF-VOZ-02 | CU-22 | El sistema permitirá controlar el micrófono dentro de un canal de voz. | Media | El usuario puede silenciar y reactivar su micrófono, y el estado se muestra a los demás participantes. |

### Inteligencia artificial — fase posterior

| ID | CU | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|---|
| RF-IA-01 | CU-32, CU-33 | El sistema podrá incorporar transcripción y resumen inteligente de reuniones. | Fase posterior | La transcripción y el resumen solo se generan cuando la funcionalidad esté habilitada y exista autorización. |
| RF-IA-02 | CU-21 | El sistema podrá incorporar traducción de mensajes a petición del usuario. | Fase posterior | El usuario puede visualizar una traducción sin modificar el mensaje original almacenado. |

## Requisitos no funcionales

| ID | Atributo | Requisito | Prioridad | Criterio de verificación | Aplica a |
|---|---|---|---|---|---|
| RNF-SEG-01 | Seguridad | Las contraseñas deberán almacenarse mediante un hash seguro (Argon2id, D-11). | Alta | No existen contraseñas en texto plano en la base de datos. | Auth |
| RNF-SEG-02 | Seguridad | Las operaciones protegidas deberán requerir autenticación mediante tokens. | Alta | Un usuario no autenticado no puede acceder a recursos privados. | Todos los servicios |
| RNF-SEG-03 | Seguridad | Las acciones dentro de un grupo deberán validarse según rol y permisos vigentes. | Alta | Un usuario sin permisos no puede ejecutar acciones administrativas o de moderación. | Core, Files, Reports |
| RNF-SEG-04 | Seguridad | Los errores no deberán exponer contraseñas, tokens, secretos ni detalles internos. | Alta | Las respuestas de error muestran mensajes controlados (Problem Details). | Todos |
| RNF-SEG-05 | Seguridad | Las sesiones deberán registrar dispositivo, IP y agente de usuario cuando corresponda. | Media | Cada sesión conserva metadatos mínimos para auditoría y control. | Auth |
| RNF-REN-01 | Rendimiento | Las pantallas principales y consultas ordinarias deberán responder en 2 segundos o menos en condiciones normales de red. | Alta | Las pruebas muestran tiempos iguales o menores a 2 segundos. | Todos |
| RNF-REN-02 | Rendimiento | El historial de mensajes deberá cargarse por páginas. | Alta | Al abrir un canal se carga solo una página inicial. | Core, clientes |
| RNF-REN-03 | Rendimiento | Las listas extensas deberán manejar carga progresiva o paginada. | Media | No se cargan grandes volúmenes de datos en una sola solicitud. | Todos |
| RNF-DIS-01 | Disponibilidad | La falla de una funcionalidad secundaria no deberá detener las funciones principales. | Alta | Si reportes, archivos o voz fallan, la autenticación y la mensajería siguen operando. | Todos |
| RNF-DIS-02 | Disponibilidad | Los servicios principales deberán exponer verificación de estado. | Media | Cada servicio tiene un *endpoint* de *health check* y cada contenedor un `healthcheck`. | Servicios, infraestructura |
| RNF-CON-01 | Confiabilidad | El sistema deberá evitar duplicidad de mensajes ante reintentos o reconexiones. | Alta | Un mensaje reenviado conserva su identificador y no se almacena dos veces. | Core, clientes |
| RNF-CON-02 | Confiabilidad | Las modificaciones concurrentes sobre tareas deberán controlarse. | Media | Si dos usuarios modifican una tarea a la vez, el sistema detecta el conflicto. | Core |
| RNF-CON-03 | Confiabilidad | Los clientes deberán manejar la pérdida temporal de conexión sin cerrar la sesión. | Media | El cliente informa la desconexión y permite reintentar o sincronizar pendientes. | Clientes |
| RNF-INT-01 | Interoperabilidad | Los clientes de escritorio y móvil deberán usar una API común. | Alta | Ambos clientes consumen los mismos servicios. | Servicios, clientes |
| RNF-INT-02 | Interoperabilidad | Los servicios se comunicarán eventos asíncronos mediante un broker (RabbitMQ, D-04). | Media | Los eventos entre servicios viajan por RabbitMQ con confirmación y cola de mensajes fallidos. **Cambia respecto a la AR02**, que lo dejaba opcional. | Servicios, infraestructura |
| RNF-USA-01 | Usabilidad | La interfaz deberá presentar navegación clara, mensajes comprensibles y estados visibles. | Alta | El usuario identifica acciones, errores y confirmaciones sin asistencia. | Clientes |
| RNF-USA-02 | Accesibilidad | La interfaz deberá considerar contraste, legibilidad y elementos interactivos visibles. | Alta | Las pantallas principales aplican criterios básicos de WCAG 2.2. | Clientes |
| RNF-MNT-01 | Mantenibilidad | El sistema deberá separar responsabilidades entre autenticación, lógica principal, archivos y reportes. | Alta | Cada servicio mantiene responsabilidades claramente diferenciadas. | Todos |
| RNF-MNT-02 | Mantenibilidad | La lógica principal deberá poder probarse automáticamente. | Media | Los módulos principales se prueban sin depender de la interfaz gráfica. | Todos |
| RNF-POR-01 | Portabilidad | Los servicios deberán ejecutarse en contenedores sobre GNU/Linux. | Alta | El sistema se levanta con Docker Compose en la infraestructura definida. | Infraestructura |
| RNF-POR-02 | Portabilidad | El cliente de escritorio deberá ejecutarse en computadoras compatibles con JavaFX. | Media | El cliente se ejecuta en el entorno objetivo. | Escritorio |
| RNF-PRI-01 | Privacidad | La transcripción o análisis de reuniones deberá requerir consentimiento de los participantes. | Fase posterior | No se transcribe una reunión sin autorización registrada. | — |

## Restricciones

| ID | Restricción | Justificación o impacto |
|---|---|---|
| RES-01 | La primera versión estará disponible mediante cliente de escritorio y cliente móvil. | Cubre los escenarios de uso desde computadora y dispositivo móvil. No hay cliente web. |
| RES-02 | Los clientes de escritorio y móvil serán aplicaciones independientes. | Cada cliente se adapta a su contexto, pantalla y forma de interacción. |
| RES-03 | Ambos clientes utilizarán una API común. | Mantiene consistencia funcional entre plataformas. |
| RES-04 | Cada usuario podrá tener como máximo una sesión activa en escritorio y una en móvil. | Simplifica el control de acceso (D-12). |
| RES-05 | El cliente móvil priorizará comunicación, consulta y participación básica. | Experiencia móvil más ligera. |
| RES-06 | La administración detallada se concentrará en el cliente de escritorio. | Requiere más espacio visual y capacidad de revisión. |
| RES-07 | El sistema deberá considerar un entorno con recursos limitados. | Favorece componentes viables de construir, probar y mantener (motivo de D-05). |
| RES-08 | El desarrollo deberá priorizar las funcionalidades esenciales del MVP. | Entregar primero lo que sostiene el valor principal. |
| RES-09 | El MVP se realizará en aproximadamente dos meses y medio. | El alcance se ajusta a una ventana limitada. |
| RES-10 | La seguridad deberá proteger cuentas, sesiones y permisos con complejidad razonable. | Equilibrio entre protección, viabilidad y mantenimiento. |
| RES-11 | Las interfaces principales deberán contemplar criterios básicos de accesibilidad. | Legibilidad y claridad para más usuarios. |
