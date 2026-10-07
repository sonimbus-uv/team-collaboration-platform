# Sonimbus — Especificación de casos de uso

**Versión canónica del repositorio.** Consolida la entrega AR02 (20 sep 2026) y la revisión de casos de uso (30 sep 2026) con las decisiones del equipo del 2026-10-07. Las diferencias respecto a la AR02 están en [`README.md`](README.md#cambios-respecto-a-la-ar02).

## Convenciones

Cada caso de uso se describe con los mismos campos: actores, descripción, precondiciones, flujo principal, flujos alternativos, postcondiciones y excepciones. Un campo se omite solo cuando no aplica.

- **Flujos alternativos (FA):** caminos válidos que se separan del flujo principal. Indican el paso donde inician y el paso al que regresan.
- **Excepciones (EX):** errores que impiden completar el caso. Indican el paso donde ocurren y si el caso regresa a un paso anterior o termina.
- **Relaciones:** las relaciones «include», «extend» y de generalización se señalan en el paso donde ocurren.
- **Servicio:** cada caso indica el servicio dueño de la lógica. Los clientes implementan la interfaz de todos los casos de su alcance (ver [`README.md`](README.md#trazabilidad-por-módulo)).

| Actor | Tipo | Descripción |
| --- | --- | --- |
| Visitante | Primario | Persona sin sesión activa. |
| Usuario autenticado | Primario | Persona con sesión activa, pertenezca o no a un grupo. |
| Miembro | Primario | Usuario autenticado que pertenece al grupo en cuestión. Hereda de Usuario autenticado. |
| Moderador | Primario | Miembro con permisos de moderación. Hereda de Miembro. |
| Administrador de grupo | Primario | Moderador con control de la configuración del grupo. Hereda de Moderador. |
| Servicio de correo | Secundario | Entrega correos de verificación y recuperación. |
| Proveedor de identidad de Google | Secundario | Autentica cuentas de Google mediante OAuth 2.0 / OpenID Connect. |
| Servidor de medios | Secundario | Transmite el audio de los canales de voz (LiveKit). |
| Proveedor de IA | Secundario | Traduce mensajes, transcribe y resume reuniones (fase posterior). |

Las excepciones comunes a todos los casos no se repiten en cada uno:

- **EX-G1 Pérdida de conexión.** El cliente informa la desconexión, conserva los datos capturados y permite reintentar (RNF-CON-03).
- **EX-G2 Sesión expirada.** Si el token de renovación no es válido, el sistema solicita iniciar sesión de nuevo.
- **EX-G3 Error interno.** El sistema muestra un mensaje genérico sin exponer detalles internos (RNF-SEG-04) y el caso termina sin cambios.
- **EX-G4 Permisos insuficientes.** Si el rol del usuario cambió durante el caso, el sistema rechaza la acción e informa que ya no tiene permiso (RNF-SEG-03).

## Catálogo

| ID | Caso de uso | Actor principal | Fase | Servicio | Requisitos |
|---|---|---|---|---|---|
| CU-01 | Registrar cuenta | Visitante | MVP | Auth | RF-AUTH-01 |
| CU-02 | Verificar cuenta | Visitante | MVP | Auth | RF-AUTH-07 |
| CU-03 | Iniciar sesión | Visitante | MVP | Auth | RF-AUTH-02 |
| CU-04 | Iniciar sesión con Google | Visitante | MVP | Auth | RF-AUTH-06 |
| CU-05 | Recuperar contraseña | Visitante | MVP | Auth | RF-AUTH-04 |
| CU-06 | Cerrar sesión | Usuario autenticado | MVP | Auth | RF-AUTH-03 |
| CU-07 | Gestionar perfil | Usuario autenticado | MVP | Auth | RF-AUTH-05 |
| CU-08 | Crear grupo | Usuario autenticado | MVP | Core | RF-GRP-01 |
| CU-09 | Gestionar configuración del grupo | Administrador de grupo | MVP | Core | RF-GRP-07 |
| CU-10 | Gestionar invitaciones | Administrador de grupo, Moderador | MVP | Core | RF-GRP-02 |
| CU-11 | Unirse a grupo | Usuario autenticado | MVP | Core | RF-GRP-03 |
| CU-12 | Abandonar grupo | Miembro | MVP | Core | RF-GRP-08 |
| CU-13 | Gestionar canales | Administrador de grupo | MVP | Core | RF-GRP-06 |
| CU-14 | Asignar roles y permisos | Administrador de grupo | MVP | Core | RF-GRP-04 |
| CU-15 | Moderar miembros o contenido | Moderador | MVP | Core | RF-GRP-05 |
| CU-16 | Enviar mensaje | Miembro | MVP | Core | RF-MSG-01, RF-MSG-03 |
| CU-17 | Responder mensaje | Miembro | MVP | Core | RF-MSG-06 |
| CU-18 | Reaccionar a mensaje | Miembro | MVP | Core | RF-MSG-07 |
| CU-19 | Editar o eliminar mensaje propio | Miembro | MVP | Core | RF-MSG-05 |
| CU-20 | Consultar historial | Miembro | MVP | Core | RF-MSG-02 |
| CU-21 | Traducir mensaje | Miembro | Fase posterior | Por definir | RF-IA-02 |
| CU-22 | Participar en canal de voz | Miembro | MVP (D-01) | Core + LiveKit | RF-VOZ-01, RF-VOZ-02 |
| CU-23 | Compartir archivo o imagen | Miembro | MVP | Files + Core | RF-MSG-04 |
| CU-24 | Consultar archivos compartidos | Miembro | MVP | Files | RF-MSG-08 |
| CU-25 | Gestionar proyectos | Administrador de grupo | MVP | Core | RF-PRJ-01 |
| CU-26 | Gestionar tareas | Miembro | MVP | Core | RF-PRJ-02 |
| CU-27 | Asignar tarea | Administrador de grupo | MVP | Core | RF-PRJ-02 |
| CU-28 | Actualizar estado de tarea | Miembro (responsable) | MVP | Core | RF-PRJ-02 |
| CU-29 | Consultar reportes básicos | Administrador de grupo | MVP | Reports | RF-REP-01 |
| CU-30 | Exportar reportes | Administrador de grupo | MVP (prioridad baja) | Reports | RF-REP-02 |
| CU-31 | Consultar notificaciones | Miembro | MVP | Core | RF-NOT-01 |
| CU-32 | Transcribir reunión | Proveedor de IA | Fase posterior | Por definir | RF-IA-01 |
| CU-33 | Generar resumen de reunión | Miembro | Fase posterior | Por definir | RF-IA-01 |

## Paquete: Identidad y acceso

Siete casos cubren el ciclo de la cuenta: registro, verificación, acceso con correo o Google, recuperación, cierre de sesión y perfil. Servicio: **Auth**.

### CU-01 Registrar cuenta

**Actores:** Visitante (primario). Servicio de correo (secundario, a través de CU-02).

**Descripción:** Permite que un visitante cree una cuenta con nombre, correo electrónico y contraseña.

**Precondiciones:**

- El visitante no tiene una sesión activa.

**Flujo principal:**

1. El visitante selecciona “Crear cuenta”.
2. El sistema muestra el formulario con nombre, correo, contraseña y confirmación de contraseña.
3. El visitante captura los datos y confirma.
4. El sistema valida el formato de los campos y la política de contraseña.
5. El sistema verifica que el correo no esté registrado.
6. El sistema crea la cuenta en estado “pendiente de verificación” y almacena la contraseña con hash (RNF-SEG-01).
7. El sistema ejecuta CU-02 Verificar cuenta «include».
8. El sistema informa que envió un correo de verificación.

**Flujos alternativos:**

- **FA-01 (paso 1) Registro con Google.** El visitante elige “Continuar con Google”. Se ejecuta CU-04 y este caso termina.

**Postcondiciones:**

- Existe una cuenta nueva en estado “pendiente de verificación”.

**Excepciones:**

- **EX-01 (paso 4) Datos inválidos.** El sistema señala los campos con error y conserva los válidos. Regresa al paso 3.
- **EX-02 (paso 5) Correo ya registrado.** El sistema rechaza el registro y ofrece iniciar sesión o recuperar la contraseña. El caso termina.

### CU-02 Verificar cuenta

**Actores:** Servicio de correo (secundario). Se ejecuta por «include» desde CU-01; el visitante interviene al usar el código o enlace.

**Descripción:** Confirma que el correo registrado pertenece a quien creó la cuenta mediante un código o enlace de un solo uso (RF-AUTH-07).

**Precondiciones:**

- Existe una cuenta en estado “pendiente de verificación”.

**Flujo principal:**

1. El sistema genera un código o enlace de un solo uso con fecha de caducidad.
2. El sistema solicita al Servicio de correo el envío al correo registrado.
3. El Servicio de correo entrega el mensaje.
4. El visitante abre el enlace o captura el código en el cliente.
5. El sistema valida que el código esté vigente y no se haya usado.
6. El sistema marca la cuenta como verificada e invalida el código.
7. El sistema informa que la cuenta está verificada y permite iniciar sesión.

**Flujos alternativos:**

- **FA-01 (paso 4) Reenviar verificación.** El visitante solicita un nuevo correo. El sistema invalida el código anterior y regresa al paso 1.

**Postcondiciones:**

- La cuenta queda verificada y el código queda invalidado.

**Excepciones:**

- **EX-01 (paso 2) Servicio de correo no disponible.** El sistema registra el fallo e informa que no pudo enviar el correo. La cuenta sigue pendiente y el visitante puede usar FA-01 más tarde.
- **EX-02 (paso 5) Código caducado o usado.** El sistema rechaza el código y ofrece FA-01.

### CU-03 Iniciar sesión

**Actores:** Visitante (primario).

**Descripción:** Autentica al visitante con correo y contraseña y abre una sesión en el dispositivo que usa.

**Precondiciones:**

- El visitante no tiene una sesión activa en ese dispositivo.
- La cuenta existe.

**Flujo principal:**

1. El visitante selecciona “Iniciar sesión”.
2. El sistema muestra el formulario de correo y contraseña.
3. El visitante captura sus credenciales y confirma.
4. El sistema valida las credenciales.
5. El sistema verifica que la cuenta esté verificada.
6. El sistema comprueba que no exista otra sesión activa en el mismo tipo de cliente (RES-04).
7. El sistema genera el token de acceso y el token de renovación y registra la sesión con dispositivo, IP y agente de usuario (RNF-SEG-05).
8. El sistema muestra la pantalla principal con los grupos del usuario.

**Flujos alternativos:**

- **FA-01 (paso 1) Inicio con Google.** El visitante elige “Continuar con Google”. Se ejecuta CU-04, que especializa este caso.
- **FA-02 (paso 3) Contraseña olvidada.** El visitante selecciona “¿Olvidaste tu contraseña?”. Se ejecuta CU-05.
- **FA-03 (paso 6) Sesión previa en el mismo tipo de cliente.** El sistema informa que la sesión anterior se cerrará. Si el visitante confirma, el sistema invalida esa sesión y continúa en el paso 7; si cancela, el caso termina sin sesión.

**Postcondiciones:**

- Existe una sesión activa registrada para el usuario en ese tipo de cliente.

**Excepciones:**

- **EX-01 (paso 4) Credenciales inválidas.** El sistema muestra un mensaje genérico sin indicar qué dato falló (RF-AUTH-02). Regresa al paso 3.
- **EX-02 (paso 5) Cuenta sin verificar.** El sistema informa que debe verificar su correo y ofrece reenviar la verificación (CU-02, FA-01). El caso termina sin sesión.

### CU-04 Iniciar sesión con Google

**Actores:** Visitante (primario, heredado de CU-03). Proveedor de identidad de Google (secundario).

**Descripción:** Especializa CU-03. Autentica al visitante con su cuenta de Google mediante OAuth 2.0 / OpenID Connect y crea la cuenta si no existe (RF-AUTH-06).

**Precondiciones:**

- El visitante no tiene una sesión activa en ese dispositivo.
- El visitante tiene una cuenta de Google.

**Flujo principal:**

1. El visitante selecciona “Continuar con Google”.
2. El sistema redirige al Proveedor de identidad de Google.
3. El visitante se autentica en Google y autoriza el acceso a su nombre y correo.
4. Google devuelve un código de autorización al sistema.
5. El sistema intercambia el código y valida el token de identidad: firma, emisor, audiencia y vigencia.
6. El sistema verifica que Google marque el correo como verificado (`email_verified`).
7. El sistema busca la cuenta vinculada al identificador de Google.
8. El sistema abre la sesión como en los pasos 6 a 8 de CU-03.

**Flujos alternativos:**

- **FA-01 (paso 7) Cuenta existente con el mismo correo.** No hay cuenta vinculada a Google, pero sí una con ese correo. El sistema vincula la identidad de Google a esa cuenta sin duplicarla y continúa en el paso 8.
- **FA-02 (paso 7) Sin cuenta.** El sistema crea una cuenta verificada con el nombre y correo de Google, sin contraseña local, y continúa en el paso 8.

**Postcondiciones:**

- Existe una sesión activa y la cuenta queda vinculada al identificador de Google.

**Excepciones:**

- **EX-01 (paso 3) Autorización cancelada.** El visitante cancela en Google. El sistema regresa a la pantalla de inicio de sesión sin crear sesión.
- **EX-02 (paso 5) Token inválido o Google no disponible.** El sistema rechaza la autenticación con un mensaje genérico. El caso termina.
- **EX-03 (paso 6) Correo no verificado por Google.** El sistema rechaza el acceso y sugiere registrarse con correo y contraseña (CU-01). El caso termina.

### CU-05 Recuperar contraseña

**Actores:** Visitante (primario). Servicio de correo (secundario).

**Descripción:** Permite definir una nueva contraseña mediante un código o enlace de un solo uso enviado al correo registrado (RF-AUTH-04).

**Precondiciones:**

- El visitante no tiene una sesión activa.

**Flujo principal:**

1. El visitante selecciona “¿Olvidaste tu contraseña?”.
2. El sistema solicita el correo de la cuenta.
3. El visitante captura su correo.
4. El sistema genera un código o enlace de un solo uso con caducidad.
5. El sistema solicita al Servicio de correo el envío.
6. El sistema muestra un mensaje neutro: si el correo está registrado, llegarán instrucciones.
7. El visitante abre el enlace o captura el código.
8. El sistema valida que el código esté vigente y no se haya usado.
9. El visitante captura la nueva contraseña y su confirmación.
10. El sistema valida la política, almacena el hash e invalida el código.
11. El sistema invalida los tokens de renovación activos de la cuenta.
12. El sistema confirma el cambio y muestra la pantalla de inicio de sesión.

**Flujos alternativos:**

- **FA-01 (paso 4) Correo no registrado.** El sistema no genera código ni envía correo y continúa en el paso 6 con el mismo mensaje, para no revelar qué correos existen.

**Postcondiciones:**

- La contraseña queda actualizada, el código invalidado y las sesiones previas cerradas.

**Excepciones:**

- **EX-01 (paso 5) Servicio de correo no disponible.** El sistema registra el fallo y muestra el mismo mensaje neutro. El visitante puede volver a solicitarlo desde el paso 1.
- **EX-02 (paso 8) Código caducado o usado.** El sistema rechaza el código y ofrece solicitar uno nuevo. Regresa al paso 2.
- **EX-03 (paso 10) Contraseña inválida.** La contraseña no cumple la política o no coincide con su confirmación. Regresa al paso 9.

### CU-06 Cerrar sesión

**Actores:** Usuario autenticado (primario).

**Descripción:** Termina la sesión del usuario en el cliente que está usando (RF-AUTH-03).

**Precondiciones:**

- El usuario tiene una sesión activa en ese dispositivo.

**Flujo principal:**

1. El usuario selecciona “Cerrar sesión”.
2. El sistema solicita confirmación.
3. El usuario confirma.
4. El sistema invalida el token de renovación de esa sesión.
5. El cliente elimina los tokens almacenados localmente.
6. El sistema muestra la pantalla de inicio de sesión.

**Flujos alternativos:**

- **FA-01 (paso 3) Cancelar.** El usuario cancela y regresa a la pantalla anterior sin cambios.

**Postcondiciones:**

- El token de renovación queda invalidado y no puede reutilizarse.
- El cliente no conserva credenciales.

**Excepciones:**

- **EX-01 (paso 4) Sin conexión.** El cliente elimina los tokens locales y muestra la pantalla de inicio de sesión. El token de renovación sigue válido en el servidor hasta su caducidad.

### CU-07 Gestionar perfil

**Actores:** Usuario autenticado (primario).

**Descripción:** Permite consultar y actualizar los datos básicos del perfil: nombre visible, foto (elegida de un paquete predefinido) y contraseña (RF-AUTH-05). El perfil vive en Auth (D-06).

**Precondiciones:**

- El usuario tiene una sesión activa.

**Flujo principal:**

1. El usuario abre su perfil.
2. El sistema muestra los datos actuales.
3. El usuario modifica su nombre visible y confirma.
4. El sistema valida los datos.
5. El sistema guarda los cambios.
6. El sistema confirma la actualización y la refleja en los grupos del usuario (evento `user.updated`).

**Flujos alternativos:**

- **FA-01 (paso 3) Cambiar foto.** El usuario elige una imagen del paquete predefinido incluido en los clientes. El sistema valida que el identificador pertenezca al paquete, lo guarda y continúa en el paso 6.
- **FA-02 (paso 3) Cambiar contraseña.** El usuario captura su contraseña actual y la nueva. El sistema verifica la actual, almacena el hash de la nueva y continúa en el paso 6.

**Postcondiciones:**

- El perfil queda actualizado y visible para los miembros de sus grupos.

**Excepciones:**

- **EX-01 (paso 4) Datos inválidos.** El sistema señala los campos con error. Regresa al paso 3.
- **EX-02 (FA-01) Imagen no válida.** El identificador no pertenece al paquete predefinido. El sistema la rechaza y regresa al paso 3.
- **EX-03 (FA-02) Contraseña actual incorrecta.** El sistema rechaza el cambio. Regresa al paso 3.

## Paquete: Grupos y permisos

Ocho casos cubren la vida de un grupo: creación, configuración, ingreso y salida de miembros, canales, roles y moderación. Servicio: **Core** (CommunityService).

### CU-08 Crear grupo

**Actores:** Usuario autenticado (primario).

**Descripción:** Permite crear un grupo de trabajo; quien lo crea queda como su administrador (RF-GRP-01).

**Precondiciones:**

- El usuario tiene una sesión activa.

**Flujo principal:**

1. El usuario selecciona “Crear grupo”.
2. El sistema solicita nombre (obligatorio), descripción e imagen (opcionales).
3. El usuario captura los datos y confirma.
4. El sistema valida los datos.
5. El sistema crea el grupo y registra al usuario como Administrador de grupo.
6. El sistema muestra el grupo nuevo.

**Flujos alternativos:**

- **FA-01 (paso 3) Cancelar.** El usuario cancela y el sistema no crea el grupo.

**Postcondiciones:**

- El grupo existe y su creador es Administrador de grupo.

**Excepciones:**

- **EX-01 (paso 4) Datos inválidos.** El sistema señala los campos con error. Regresa al paso 3.

### CU-09 Gestionar configuración del grupo

**Actores:** Administrador de grupo (primario).

**Descripción:** Permite modificar los datos del grupo, archivarlo o eliminarlo (RF-GRP-07).

**Precondiciones:**

- El usuario es Administrador del grupo.

**Flujo principal:**

1. El administrador abre la configuración del grupo.
2. El sistema muestra nombre, descripción e imagen actuales.
3. El administrador modifica los datos y confirma.
4. El sistema valida los datos.
5. El sistema guarda los cambios.
6. El sistema refleja los cambios para todos los miembros.

**Flujos alternativos:**

- **FA-01 (paso 3) Archivar grupo.** El administrador elige archivar y confirma. El grupo pasa a solo lectura: los miembros consultan el historial pero no publican.
- **FA-02 (paso 3) Eliminar grupo.** El administrador elige eliminar y confirma escribiendo el nombre del grupo. El sistema elimina el grupo y revoca el acceso de todos los miembros.

**Postcondiciones:**

- La configuración queda actualizada, o el grupo queda archivado o eliminado.

**Excepciones:**

- **EX-01 (paso 4) Datos inválidos.** El sistema señala los campos con error. Regresa al paso 3.
- **EX-02 (FA-02) Confirmación incorrecta.** El nombre escrito no coincide. El sistema cancela la eliminación.

### CU-10 Gestionar invitaciones

**Actores:** Administrador de grupo, Moderador (primarios).

**Descripción:** Permite crear y revocar invitaciones con caducidad y límite de usos (RF-GRP-02).

**Precondiciones:**

- El usuario es Administrador o Moderador del grupo.

**Flujo principal:**

1. El administrador o moderador abre las invitaciones del grupo.
2. El sistema muestra las invitaciones vigentes con sus usos restantes y caducidad.
3. El administrador o moderador selecciona “Crear invitación”.
4. El administrador o moderador define la caducidad y el número máximo de usos.
5. El sistema genera un código y enlace únicos.
6. El sistema muestra la invitación para copiarla o compartirla.

**Flujos alternativos:**

- **FA-01 (paso 3) Revocar invitación.** El administrador o moderador selecciona una invitación vigente y confirma. El sistema la marca como revocada.

**Postcondiciones:**

- Existe una invitación vigente, o la invitación seleccionada queda revocada.

**Excepciones:**

- **EX-01 (paso 4) Parámetros inválidos.** La caducidad ya pasó o el límite de usos es menor a 1. Regresa al paso 4.

### CU-11 Unirse a grupo

**Actores:** Usuario autenticado (primario).

**Descripción:** Permite ingresar a un grupo con una invitación válida, sin duplicar membresías (RF-GRP-03).

**Precondiciones:**

- El usuario tiene una sesión activa.
- El usuario cuenta con un código o enlace de invitación.

**Flujo principal:**

1. El usuario abre el enlace o selecciona “Unirse a grupo”.
2. El usuario captura el código, si no usó un enlace.
3. El sistema valida que la invitación esté vigente, no revocada y con usos disponibles.
4. El sistema verifica que el usuario no sea miembro del grupo.
5. El sistema verifica que el usuario no esté bloqueado en el grupo.
6. El sistema agrega al usuario con rol Miembro y descuenta un uso de la invitación, en una sola operación atómica.
7. El sistema muestra el grupo.

**Flujos alternativos:**

- **FA-01 (paso 1) Sin sesión.** El enlace se abre sin sesión activa. El sistema solicita iniciar sesión (CU-03) y después continúa en el paso 3.
- **FA-02 (paso 2) Cancelar.** El usuario cancela y no se une al grupo.

**Postcondiciones:**

- El usuario es Miembro del grupo y la invitación tiene un uso menos.

**Excepciones:**

- **EX-01 (paso 3) Invitación no válida.** Está vencida, revocada o agotada. El sistema la rechaza y el caso termina.
- **EX-02 (paso 4) Ya es miembro.** El sistema muestra el grupo sin crear otra membresía ni descontar usos.
- **EX-03 (paso 5) Usuario bloqueado.** El sistema rechaza el ingreso y el caso termina.

### CU-12 Abandonar grupo

**Actores:** Miembro (primario).

**Descripción:** Permite que un miembro salga de un grupo por decisión propia (RF-GRP-08).

**Precondiciones:**

- El usuario es miembro del grupo.

**Flujo principal:**

1. El miembro selecciona “Abandonar grupo”.
2. El sistema solicita confirmación.
3. El miembro confirma.
4. El sistema verifica que el miembro no sea el único administrador.
5. El sistema elimina la membresía.
6. El sistema retira el grupo de la lista del usuario.

**Flujos alternativos:**

- **FA-01 (paso 3) Cancelar.** El miembro cancela y conserva su membresía.

**Postcondiciones:**

- El usuario pierde el acceso al grupo. Sus mensajes anteriores permanecen en el historial.

**Excepciones:**

- **EX-01 (paso 4) Único administrador.** El sistema exige transferir el rol de administrador (CU-14) o eliminar el grupo (CU-09) antes de salir. El caso termina.

### CU-13 Gestionar canales

**Actores:** Administrador de grupo (primario).

**Descripción:** Permite crear, modificar y eliminar canales de texto y de voz del grupo (RF-GRP-06).

**Precondiciones:**

- El usuario es Administrador del grupo.

**Flujo principal:**

1. El administrador abre la lista de canales del grupo.
2. El administrador selecciona “Crear canal”.
3. El administrador captura nombre, descripción y tipo: texto o voz.
4. El sistema valida que el nombre no se repita dentro del grupo.
5. El sistema crea el canal.
6. El canal aparece para los miembros del grupo.

**Flujos alternativos:**

- **FA-01 (paso 2) Modificar canal.** El administrador selecciona un canal y cambia su nombre o descripción. Continúa en el paso 4.
- **FA-02 (paso 2) Eliminar canal.** El administrador selecciona un canal y confirma. El sistema elimina el canal y su historial deja de estar disponible.

**Postcondiciones:**

- El canal queda creado, modificado o eliminado, y el cambio es visible para los miembros.

**Excepciones:**

- **EX-01 (paso 4) Nombre repetido o inválido.** Regresa al paso 3.

### CU-14 Asignar roles y permisos

**Actores:** Administrador de grupo (primario).

**Descripción:** Permite cambiar el rol de un miembro y ajustar los permisos de cada rol (RF-GRP-04).

**Precondiciones:**

- El usuario es Administrador del grupo.

**Flujo principal:**

1. El administrador abre la lista de miembros.
2. El administrador selecciona un miembro.
3. El administrador elige el rol: Miembro, Moderador o Administrador de grupo.
4. El administrador confirma.
5. El sistema actualiza el rol e invalida la caché de permisos del miembro.
6. El sistema aplica el nuevo rol a las acciones siguientes del miembro.
7. El sistema notifica el cambio al miembro (RF-NOT-01).

**Flujos alternativos:**

- **FA-01 (paso 2) Ajustar permisos de un rol.** El administrador selecciona un rol y activa o desactiva permisos específicos. El sistema los aplica a todos los miembros con ese rol.

**Postcondiciones:**

- El miembro tiene el nuevo rol, o el rol tiene los nuevos permisos.

**Excepciones:**

- **EX-01 (paso 5) Grupo sin administrador.** El cambio dejaría al grupo sin administradores. El sistema lo rechaza. Regresa al paso 3.

### CU-15 Moderar miembros o contenido

**Actores:** Moderador (primario).

**Descripción:** Permite eliminar mensajes de otros, expulsar o bloquear miembros y retirar bloqueos (RF-GRP-05).

**Precondiciones:**

- El usuario es Moderador o Administrador del grupo.

**Flujo principal:**

1. El moderador selecciona un mensaje o un miembro.
2. El moderador elige la acción: eliminar mensaje, expulsar o bloquear.
3. El moderador captura un motivo (opcional) y confirma.
4. El sistema verifica que el afectado no tenga un rol igual o superior.
5. El sistema aplica la acción según FA-01, FA-02 o FA-03.
6. El sistema registra quién aplicó la acción, sobre quién y cuándo.
7. El sistema notifica al afectado.

**Flujos alternativos:**

- **FA-01 (paso 5) Eliminar mensaje.** El sistema reemplaza el mensaje por el aviso “Mensaje eliminado por moderación”.
- **FA-02 (paso 5) Expulsar.** El sistema elimina la membresía. Si el afectado está en un canal de voz, lo desconecta. El afectado puede volver con una nueva invitación.
- **FA-03 (paso 5) Bloquear.** El sistema elimina la membresía, lo desconecta de la voz e impide el reingreso.
- **FA-04 (paso 1) Retirar bloqueo.** El moderador consulta los usuarios bloqueados, selecciona uno y confirma. El usuario puede volver con una invitación.

**Postcondiciones:**

- La acción queda aplicada y registrada.

**Excepciones:**

- **EX-01 (paso 4) Rol igual o superior.** El sistema rechaza la acción y el caso termina.

## Paquete: Comunicación

Siete casos cubren la mensajería en canales de texto y la voz. CU-21 es de fase posterior. Servicio: **Core** (MessagingService, RealtimeService y VoiceService), con LiveKit para el audio.

### CU-16 Enviar mensaje

**Actores:** Miembro (primario).

**Descripción:** Permite publicar un mensaje de texto en un canal y entregarlo en tiempo real, sin duplicados ante reintentos (RF-MSG-01, RF-MSG-03).

**Precondiciones:**

- El usuario es miembro del grupo y tiene acceso al canal de texto.
- El grupo no está archivado.

**Flujo principal:**

1. El miembro abre el canal.
2. El miembro escribe el mensaje. *Punto de extensión: Responder (CU-17).*
3. El miembro envía el mensaje.
4. El cliente asigna al mensaje un identificador único (`client_message_id`).
5. El sistema valida permisos y contenido: no vacío y dentro de la longitud máxima.
6. El sistema almacena el mensaje y le asigna el orden en el servidor (D-08).
7. El sistema entrega el mensaje en tiempo real a los miembros conectados con acceso al canal.
8. El cliente muestra el mensaje como enviado.

**Flujos alternativos:**

- **FA-01 (paso 2) Adjuntar archivo.** El miembro adjunta un archivo o imagen. Se ejecuta CU-23.
- **FA-02 (paso 6) Reenvío tras reconexión.** El identificador ya existe. El sistema no vuelve a almacenarlo y confirma el mensaje original (RNF-CON-01).
- **FA-03 (paso 7) Miembros desconectados.** Reciben el mensaje al abrir el canal (CU-20).

**Postcondiciones:**

- El mensaje está almacenado una sola vez y es visible para los miembros con acceso al canal.

**Excepciones:**

- **EX-01 (paso 5) Contenido inválido.** El mensaje está vacío o excede la longitud máxima. Regresa al paso 2.
- **EX-02 (paso 6) Sin conexión.** El cliente marca el mensaje como pendiente y lo reenvía con el mismo identificador al reconectarse (FA-02).
- **EX-03 (paso 5) Grupo archivado.** El sistema rechaza la publicación y el caso termina.

### CU-17 Responder mensaje

**Actores:** Miembro (primario).

**Descripción:** Extiende CU-16 «extend». Permite publicar un mensaje que conserva la referencia a otro mensaje del canal (RF-MSG-06).

**Precondiciones:**

- El mensaje original existe y es visible para el miembro.

**Flujo principal:**

1. El miembro selecciona un mensaje y elige “Responder”.
2. El cliente muestra la referencia al mensaje original en el cuadro de escritura.
3. El miembro escribe la respuesta y la envía.
4. Continúa CU-16 desde el paso 4, con la referencia incluida.
5. El sistema muestra la respuesta con un vínculo al mensaje original.

**Flujos alternativos:**

- **FA-01 (paso 3) Quitar referencia.** El miembro descarta la referencia y el mensaje se envía como mensaje normal.

**Postcondiciones:**

- La respuesta queda almacenada con la referencia al mensaje original.

**Excepciones:**

- **EX-01 (paso 4) Original eliminado.** El original se eliminó antes del envío. La respuesta se guarda y muestra el aviso “Mensaje original no disponible”.

### CU-18 Reaccionar a mensaje

**Actores:** Miembro (primario).

**Descripción:** Permite agregar o retirar una reacción a un mensaje sin generar duplicados (RF-MSG-07).

**Precondiciones:**

- El mensaje existe y es visible para el miembro.

**Flujo principal:**

1. El miembro selecciona un mensaje.
2. El miembro elige una reacción.
3. El sistema verifica que el miembro no tenga ya esa reacción en el mensaje.
4. El sistema registra la reacción.
5. El sistema actualiza el contador de reacciones en tiempo real.

**Flujos alternativos:**

- **FA-01 (paso 3) Reacción existente.** El miembro ya tenía esa reacción. El sistema la retira y continúa en el paso 5.

**Postcondiciones:**

- Cada miembro tiene como máximo una reacción de cada tipo por mensaje.

**Excepciones:**

- **EX-01 (paso 4) Mensaje eliminado.** El sistema rechaza la reacción y el caso termina.

### CU-19 Editar o eliminar mensaje propio

**Actores:** Miembro (primario).

**Descripción:** Permite al autor modificar o eliminar sus propios mensajes (RF-MSG-05).

**Precondiciones:**

- El miembro es autor del mensaje.

**Flujo principal:**

1. El miembro selecciona un mensaje propio.
2. El miembro elige “Editar”.
3. El miembro modifica el texto y confirma.
4. El sistema verifica que el usuario sea el autor.
5. El sistema guarda el nuevo contenido y marca el mensaje como “editado”.
6. El sistema refleja el cambio para los miembros del canal.

**Flujos alternativos:**

- **FA-01 (paso 2) Eliminar.** El miembro elige “Eliminar” y confirma. El sistema reemplaza el mensaje por el aviso “Mensaje eliminado” y sus adjuntos dejan de estar disponibles. Continúa en el paso 6.

**Postcondiciones:**

- El mensaje queda editado o eliminado, y el cambio es visible en el canal.

**Excepciones:**

- **EX-01 (paso 4) No es el autor.** El sistema rechaza el cambio y el caso termina.
- **EX-02 (paso 3) Contenido vacío o demasiado extenso.** Regresa al paso 3.

### CU-20 Consultar historial

**Actores:** Miembro (primario).

**Descripción:** Permite consultar los mensajes anteriores de un canal, cargados por páginas (RF-MSG-02, RNF-REN-02).

**Precondiciones:**

- El usuario es miembro del grupo y tiene acceso al canal.

**Flujo principal:**

1. El miembro abre el canal.
2. El sistema entrega la página más reciente de mensajes en orden cronológico. *Punto de extensión: Traducir mensaje (CU-21).*
3. El miembro se desplaza hacia mensajes anteriores.
4. El sistema entrega la página anterior (paginación por cursor).
5. Los pasos 3 y 4 se repiten mientras el miembro lo requiera.

**Flujos alternativos:**

- **FA-01 (paso 3) Ir al mensaje original.** El miembro selecciona la referencia de una respuesta. El sistema carga la página que contiene el original.
- **FA-02 (paso 4) Inicio del canal.** No hay más mensajes. El sistema lo indica.

**Postcondiciones:**

- El historial no se modifica.

**Excepciones:**

- **EX-01 (paso 2) Canal no disponible.** El canal fue eliminado o el miembro perdió el acceso. El sistema lo informa y el caso termina.

### CU-21 Traducir mensaje «fase posterior»

**Actores:** Miembro (primario). Proveedor de IA (secundario).

**Descripción:** Extiende CU-20 «extend». Muestra, a petición del miembro, la traducción de un mensaje sin modificar el original almacenado (RF-IA-02).

**Precondiciones:**

- La funcionalidad de traducción está habilitada.
- El miembro está consultando el historial de un canal.

**Flujo principal:**

1. El miembro selecciona un mensaje y elige “Traducir”.
2. El miembro elige el idioma destino.
3. El sistema envía el texto al Proveedor de IA.
4. El Proveedor de IA devuelve la traducción.
5. El sistema muestra la traducción junto al mensaje original.

**Flujos alternativos:**

- **FA-01 (paso 5) Ver original.** El miembro oculta la traducción y el sistema muestra solo el mensaje original.

**Postcondiciones:**

- El mensaje original almacenado no cambia.

**Excepciones:**

- **EX-01 (paso 3) Proveedor no disponible.** El sistema informa que la traducción no está disponible y conserva el original visible.
- **EX-02 (paso 4) Idioma no admitido.** El sistema lo informa. Regresa al paso 2.

### CU-22 Participar en canal de voz

**Actores:** Miembro (primario). Servidor de medios (secundario).

**Descripción:** Permite unirse a un canal de voz, escuchar, transmitir audio y controlar el micrófono (RF-VOZ-01, RF-VOZ-02). Forma parte del MVP (D-01). Core verifica permisos y emite el token de LiveKit (D-15); el audio viaja por WebRTC entre el cliente y LiveKit.

**Precondiciones:**

- El canal de voz existe y el miembro tiene permiso para unirse.

**Flujo principal:**

1. El miembro selecciona un canal de voz.
2. El sistema verifica sus permisos.
3. El sistema genera las credenciales de la sala del Servidor de medios y las entrega al cliente.
4. El cliente se conecta al Servidor de medios.
5. El miembro escucha y transmite audio.
6. El sistema muestra a los participantes y el estado de su micrófono.
7. El miembro selecciona “Salir”.
8. El cliente cierra la conexión y el sistema actualiza la lista de participantes.

**Flujos alternativos:**

- **FA-01 (paso 5) Silenciar o reactivar micrófono.** El cliente detiene o reanuda la transmisión. El sistema actualiza el estado visible para los demás.
- **FA-02 (paso 4) Micrófono no permitido por el dispositivo.** El miembro entra en modo solo escucha.
- **FA-03 (paso 5) Expulsión.** Si el miembro es expulsado o bloqueado (CU-15), el sistema lo desconecta de la sala.

**Postcondiciones:**

- Al salir, el miembro deja de aparecer como participante.

**Excepciones:**

- **EX-01 (paso 3) Servidor de medios no disponible.** El sistema informa que la voz no está disponible. El resto del sistema sigue operando (RNF-DIS-01).
- **EX-02 (paso 5) Pérdida de conexión.** El cliente intenta reconectarse; si no lo logra, el miembro sale del canal.

## Paquete: Archivos

Dos casos cubren el contenido compartido: subirlo dentro de un mensaje y consultarlo o descargarlo después. Servicio: **Files** (contenido en MinIO y metadatos en PostgreSQL); Core publica el mensaje y decide el acceso al canal.

### CU-23 Compartir archivo o imagen

**Actores:** Miembro (primario).

**Descripción:** Permite adjuntar un archivo o imagen a un mensaje del canal. El contenido se guarda en el almacenamiento de objetos y el mensaje conserva su referencia (RF-MSG-04).

**Precondiciones:**

- El usuario es miembro del grupo y tiene acceso al canal.
- El grupo no está archivado.

**Flujo principal:**

1. El miembro selecciona “Adjuntar” (o llega desde CU-16, FA-01).
2. El miembro elige un archivo o imagen de su dispositivo.
3. El cliente valida tipo y tamaño y muestra nombre, tamaño y, si es imagen, una miniatura.
4. El miembro agrega un texto opcional y envía.
5. El sistema valida nuevamente el tipo y el tamaño del archivo.
6. El sistema almacena el archivo y sus metadatos (nombre, tipo, tamaño, autor y canal) y genera la vista previa si es una imagen.
7. El sistema publica un mensaje asociado al archivo, como en los pasos 6 a 8 de CU-16.
8. Los miembros del canal ven el archivo; las imágenes se muestran con vista previa.

**Flujos alternativos:**

- **FA-01 (paso 4) Cancelar.** El miembro descarta el adjunto antes de enviarlo y no se publica nada.

**Postcondiciones:**

- El archivo queda almacenado y asociado a un mensaje visible solo para los miembros con acceso al canal, y aparece en los archivos compartidos del grupo (CU-24).

**Excepciones:**

- **EX-01 (pasos 3 y 5) Tipo o tamaño no permitido.** El sistema rechaza el archivo e indica los límites. Regresa al paso 2.
- **EX-02 (paso 4) Sin conexión.** El cliente conserva el archivo como pendiente y lo envía al recuperar la conexión.
- **EX-03 (paso 6) Carga interrumpida.** El cliente informa el fallo y permite reintentar. No se publica ningún mensaje; el archivo incompleto queda en estado pendiente y se limpia periódicamente.
- **EX-04 (paso 6) Almacenamiento no disponible.** El sistema informa que no puede recibir archivos. Los mensajes de texto siguen funcionando (RNF-DIS-01).

### CU-24 Consultar archivos compartidos

**Actores:** Miembro (primario).

**Descripción:** Permite ver y descargar los archivos compartidos en los canales a los que el miembro tiene acceso (RF-MSG-08).

**Precondiciones:**

- El usuario es miembro del grupo.

**Flujo principal:**

1. El miembro abre la sección de archivos del grupo.
2. El sistema lista los archivos de los canales con acceso, del más reciente al más antiguo, por páginas (RNF-REN-03).
3. El miembro selecciona un archivo.
4. El sistema muestra la vista previa (imágenes) o sus datos: nombre, tamaño, autor y fecha.
5. El miembro elige “Descargar”.
6. El sistema verifica el permiso y entrega el archivo.

**Flujos alternativos:**

- **FA-01 (paso 2) Filtrar.** El miembro filtra por canal, tipo de archivo o fecha. El sistema actualiza la lista.
- **FA-02 (paso 4) Ir al mensaje.** El miembro elige ver el mensaje donde se compartió. Se ejecuta CU-20 en esa posición.

**Postcondiciones:**

- Los archivos no se modifican.

**Excepciones:**

- **EX-01 (paso 2) Sin archivos.** El sistema muestra un estado vacío y el caso termina.
- **EX-02 (paso 6) Archivo eliminado.** El mensaje asociado se eliminó. El sistema informa que el archivo ya no está disponible y lo retira de la lista.
- **EX-03 (paso 6) Sin permiso.** El miembro perdió el acceso al canal. El sistema rechaza la descarga.

## Paquete: Proyectos y tareas

Cuatro casos cubren la gestión ligera de proyectos. Las tareas usan tres estados: **Pendiente**, **En progreso** y **Completada**, y tienen un número de versión para el control optimista de concurrencia (RNF-CON-02). Servicio: **Core** (ProjectService).

### CU-25 Gestionar proyectos

**Actores:** Administrador de grupo (primario).

**Descripción:** Permite crear, modificar y archivar proyectos del grupo (RF-PRJ-01). Todos los miembros consultan los proyectos al trabajar con sus tareas (CU-26).

**Precondiciones:**

- El usuario es Administrador del grupo.

**Flujo principal:**

1. El administrador abre la sección de proyectos.
2. El sistema lista los proyectos activos.
3. El administrador selecciona “Crear proyecto”.
4. El administrador captura nombre, descripción y fecha objetivo (opcional).
5. El sistema valida los datos.
6. El sistema crea el proyecto.
7. El proyecto aparece para los miembros del grupo.

**Flujos alternativos:**

- **FA-01 (paso 3) Modificar proyecto.** El administrador selecciona un proyecto y cambia sus datos. Continúa en el paso 5.
- **FA-02 (paso 3) Archivar proyecto.** El administrador selecciona un proyecto y confirma. Si tiene tareas abiertas, el sistema lo advierte y pide confirmación. El proyecto y sus tareas pasan a solo lectura.

**Postcondiciones:**

- El proyecto queda creado, modificado o archivado, y es visible solo para los miembros del grupo.

**Excepciones:**

- **EX-01 (paso 5) Datos inválidos o nombre repetido.** Regresa al paso 4.
- **EX-02 (paso 3) Sin permiso.** Un usuario que no es administrador intenta crear, modificar o archivar. El sistema rechaza la operación.

### CU-26 Gestionar tareas

**Actores:** Miembro (primario).

**Descripción:** Permite crear, modificar y eliminar tareas de un proyecto sin sobrescribir cambios concurrentes (RF-PRJ-02, RNF-CON-02).

**Precondiciones:**

- El usuario es miembro del grupo.
- Existe un proyecto activo.

**Flujo principal:**

1. El miembro abre un proyecto.
2. El sistema muestra sus tareas con estado, responsable y fecha límite.
3. El miembro selecciona “Crear tarea”.
4. El miembro captura título, descripción y fecha límite (opcional). *Punto de extensión: Asignar tarea (CU-27).*
5. El sistema valida los datos.
6. El sistema crea la tarea con estado Pendiente y versión 1.
7. La tarea aparece para los miembros conectados del grupo.

**Flujos alternativos:**

- **FA-01 (paso 3) Modificar tarea.** El miembro selecciona una tarea y cambia sus datos. El cliente envía la versión que el miembro consultó; el sistema verifica que nadie la haya modificado desde entonces, incrementa la versión y continúa en el paso 5.
- **FA-02 (paso 3) Eliminar tarea.** El creador de la tarea o el administrador la selecciona y confirma. El sistema la elimina.

**Postcondiciones:**

- La tarea queda creada, modificada o eliminada sin perder cambios de otros usuarios.

**Excepciones:**

- **EX-01 (paso 5) Datos inválidos.** Regresa al paso 4.
- **EX-02 (FA-01) Conflicto de edición.** La versión almacenada es distinta de la consultada. El sistema no sobrescribe, muestra la versión actual y permite volver a aplicar los cambios (RNF-CON-02).
- **EX-03 (FA-02) Sin permiso.** El miembro no es el creador ni administrador. El sistema rechaza la eliminación.

### CU-27 Asignar tarea

**Actores:** Administrador de grupo (primario).

**Descripción:** Extiende CU-26 «extend». Permite asignar o reasignar el responsable de una tarea (RF-PRJ-02).

**Precondiciones:**

- El usuario es Administrador del grupo.
- La tarea existe o se está creando en CU-26.

**Flujo principal:**

1. El administrador selecciona “Asignar” en una tarea.
2. El sistema muestra los miembros del grupo.
3. El administrador elige al responsable.
4. El sistema verifica que el responsable pertenezca al grupo, registra la asignación e incrementa la versión de la tarea.
5. El sistema notifica al responsable (CU-31) y refleja el cambio para los miembros conectados.

**Flujos alternativos:**

- **FA-01 (paso 3) Quitar responsable.** El administrador deja la tarea sin responsable. El sistema registra el cambio y notifica a quien la tenía asignada.

**Postcondiciones:**

- La tarea tiene el responsable elegido, o queda sin responsable.

**Excepciones:**

- **EX-01 (paso 4) Responsable fuera del grupo.** El miembro elegido abandonó o fue expulsado. El sistema rechaza la asignación. Regresa al paso 3.
- **EX-02 (paso 4) Conflicto de edición.** Otro usuario modificó la tarea. Se maneja como EX-02 de CU-26.

### CU-28 Actualizar estado de tarea

**Actores:** Miembro (primario, como responsable de la tarea).

**Descripción:** Permite al responsable o al administrador cambiar el estado de una tarea (RF-PRJ-02).

**Precondiciones:**

- El usuario es responsable de la tarea o Administrador del grupo.

**Flujo principal:**

1. El miembro abre una tarea asignada.
2. El miembro elige el nuevo estado: Pendiente, En progreso o Completada.
3. El sistema verifica que sea el responsable o un administrador.
4. El sistema verifica que nadie haya modificado la tarea desde que la abrió.
5. El sistema guarda el estado y la fecha del cambio e incrementa la versión.
6. El sistema refleja el cambio y notifica a quien creó la tarea.

**Flujos alternativos:**

- **FA-01 (paso 2) Reabrir tarea.** La tarea estaba Completada y el miembro la regresa a Pendiente o En progreso. Continúa en el paso 3.

**Postcondiciones:**

- La tarea tiene el nuevo estado y su fecha de cambio.

**Excepciones:**

- **EX-01 (paso 3) No es responsable.** El sistema rechaza el cambio y el caso termina.
- **EX-02 (paso 4) Conflicto de edición.** Se maneja como EX-02 de CU-26.

## Paquete: Reportes y notificaciones

Tres casos cubren la información resumida para el administrador y los avisos internos para los miembros. Servicios: **Reports** (CU-29, CU-30) y **Core** (CU-31, NotificationService).

### CU-29 Consultar reportes básicos

**Actores:** Administrador de grupo (primario).

**Descripción:** Permite consultar reportes con gráficas de la actividad del grupo: mensajes por periodo, participación por miembro y tareas por estado (RF-REP-01).

**Precondiciones:**

- El usuario es Administrador del grupo.

**Flujo principal:**

1. El administrador abre la sección de reportes.
2. El administrador elige el tipo de reporte (actividad de mensajes, participación de miembros o estado de tareas) y el periodo.
3. El sistema valida el periodo, que no puede exceder el rango máximo permitido.
4. El sistema obtiene la información resumida del periodo.
5. El cliente muestra las gráficas y la tabla de datos con sus totales. *Punto de extensión: Exportar reportes (CU-30).*

**Flujos alternativos:**

- **FA-01 (paso 4) Periodo sin actividad.** El sistema muestra el reporte vacío con un aviso.

**Postcondiciones:**

- Los datos del grupo no se modifican.

**Excepciones:**

- **EX-01 (paso 3) Periodo inválido o excedido.** La fecha final es anterior a la inicial o el rango supera el máximo. Regresa al paso 2.
- **EX-02 (paso 4) Reportes no disponibles.** El sistema lo informa. El resto de las funciones sigue operando (RNF-DIS-01).

### CU-30 Exportar reportes

**Actores:** Administrador de grupo (a través de CU-29).

**Descripción:** Extiende CU-29 «extend». Genera un archivo PDF o CSV con la información del reporte consultado (RF-REP-02, prioridad baja).

**Precondiciones:**

- Hay un reporte consultado en pantalla.

**Flujo principal:**

1. El administrador elige “Exportar”.
2. El administrador elige el formato: PDF o CSV.
3. El sistema genera el archivo con la misma información mostrada.
4. El cliente guarda el archivo en el dispositivo.

**Postcondiciones:**

- Existe un archivo con la información del reporte consultado.

**Excepciones:**

- **EX-01 (paso 3) Error al generar.** El sistema informa que no pudo generar el archivo. El reporte sigue visible.

### CU-31 Consultar notificaciones

**Actores:** Miembro (primario).

**Descripción:** Permite recibir en tiempo real y consultar los avisos internos sobre eventos relevantes del grupo e ir al elemento relacionado (RF-NOT-01).

**Precondiciones:**

- El usuario tiene una sesión activa y pertenece al menos a un grupo.

**Flujo principal:**

1. Ocurre un evento relevante: respuesta a un mensaje propio, tarea asignada, cambio de rol, acción de moderación o actividad en un canal de voz.
2. El sistema determina los destinatarios y crea la notificación.
3. El sistema entrega la notificación en tiempo real y el cliente muestra un indicador de no leídas.
4. El miembro abre la bandeja de notificaciones.
5. El sistema lista las notificaciones de la más reciente a la más antigua.
6. El miembro selecciona una notificación.
7. El sistema la marca como leída y muestra el elemento relacionado.

**Flujos alternativos:**

- **FA-01 (paso 3) Miembro desconectado.** La notificación se conserva y se muestra al reconectarse.
- **FA-02 (paso 5) Marcar todas como leídas.** El sistema marca todas como leídas y retira el indicador.
- **FA-03 (paso 3) Notificación push «fase posterior».** Si el cliente móvil no está activo, se envía una notificación push.

**Postcondiciones:**

- Las notificaciones abiertas quedan marcadas como leídas.

**Excepciones:**

- **EX-01 (paso 7) Elemento no disponible.** El mensaje, tarea o grupo relacionado ya no existe o el miembro perdió el acceso. El sistema marca la notificación como leída e informa que el elemento no está disponible.

## Paquete: Reuniones «fase posterior»

Dos casos de fase posterior procesan las reuniones de los canales de voz. Ambos requieren el consentimiento de los participantes (RNF-PRI-01) y dependen de CU-22.

### CU-32 Transcribir reunión

**Actores:** Proveedor de IA (secundario). Se ejecuta por «include» desde CU-33.

**Descripción:** Convierte en texto el audio de una reunión de un canal de voz (RF-IA-01).

**Precondiciones:**

- La funcionalidad de transcripción está habilitada.
- La reunión se grabó en un canal de voz con el consentimiento de todos los participantes.

**Flujo principal:**

1. El sistema obtiene el audio de la reunión.
2. El sistema verifica que exista el consentimiento registrado de todos los participantes.
3. El sistema envía el audio al Proveedor de IA.
4. El Proveedor de IA devuelve la transcripción con marcas de tiempo.
5. El sistema almacena la transcripción asociada a la reunión, visible para los miembros con acceso al canal.

**Postcondiciones:**

- Existe una transcripción de la reunión.

**Excepciones:**

- **EX-01 (paso 2) Sin consentimiento.** Falta el consentimiento de al menos un participante. El sistema no transcribe y el caso termina.
- **EX-02 (paso 3) Proveedor no disponible.** El sistema registra el fallo e informa que la transcripción puede reintentarse más tarde.

### CU-33 Generar resumen de reunión

**Actores:** Miembro (primario). Proveedor de IA (secundario).

**Descripción:** Genera un resumen de una reunión a partir de su transcripción e incluye CU-32 «include» (RF-IA-01).

**Precondiciones:**

- La funcionalidad de resúmenes está habilitada.
- La reunión se grabó con el consentimiento de todos los participantes.
- El miembro tiene acceso al canal de voz.

**Flujo principal:**

1. El miembro selecciona una reunión del canal de voz.
2. El miembro elige “Generar resumen”.
3. El sistema ejecuta CU-32 Transcribir reunión «include».
4. El sistema envía la transcripción al Proveedor de IA.
5. El Proveedor de IA devuelve el resumen con temas, acuerdos y pendientes.
6. El sistema almacena el resumen y lo muestra junto a la transcripción.

**Flujos alternativos:**

- **FA-01 (paso 3) Transcripción existente.** La reunión ya tiene transcripción. El sistema la reutiliza y continúa en el paso 4.
- **FA-02 (paso 2) Resumen existente.** La reunión ya tiene resumen. El sistema lo muestra sin generar otro.

**Postcondiciones:**

- El resumen queda almacenado y asociado a la reunión.

**Excepciones:**

- **EX-01 (paso 3) Falla CU-32.** No hay consentimiento o el proveedor no respondió. El caso termina sin resumen.
- **EX-02 (paso 4) Proveedor no disponible.** El sistema informa que el resumen puede reintentarse más tarde. La transcripción se conserva.
