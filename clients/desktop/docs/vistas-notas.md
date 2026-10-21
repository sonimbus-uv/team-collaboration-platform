# Notas de las vistas JavaFX

Bitácora de lo hecho al seguir `vistas-javafx.md`: qué se agregó fuera del prototipo, qué no se pudo representar y qué decisiones se tomaron sin consultar. El resumen y la lista "Para revisar" están al final.

## Tareas iniciales

- **`View.java`:** solo tenía `LOGIN`. Se agregaron `REGISTER`, `VERIFY_ACCOUNT`, `RECOVER_PASSWORD` y `MAIN`, y después una constante por cada vista nueva.
- **`theme.css`:** se comparó contra `HEAD` selector por selector y propiedad por propiedad. La versión anterior no tenía ninguna regla que falte en la actual (solo cambiaron comentarios), así que **no hizo falta** la sección "Reglas previas".
- **Compilación — problema encontrado:** `clients/desktop/pom.xml` (el pom padre, `sonimbus-desktop`) **no existe**: se agregó en `05cfe4b` y se borró en `8f4f33a` ("Antes de generar vistas JavaFX"). Sin él, `mvn -f clients/desktop/app/pom.xml compile` falla con *Non-resolvable parent POM*. Como el documento prohíbe tocar el `pom.xml`, no lo restauré. Para verificar usé una copia temporal fuera del repositorio (el pom padre de `05cfe4b` + copia de `app/pom.xml` + enlace simbólico a `app/src`). Para recuperarlo: `git checkout 05cfe4b -- clients/desktop/pom.xml`. **Actualización:** restaurado con ese comando, con aprobación del usuario; `mvn -f clients/desktop/pom.xml test` pasa. Queda en el índice de git (staged), sin commit.
- **Prueba de humo:** el `exec-maven-plugin` no está en el repositorio local de Maven y no descargué nada (regla de no instalar dependencias). Ejecuté `FxmlSmokeCheck` directamente con `java -cp` (clases compiladas + los jar de JavaFX 21.0.8 de `~/.m2`). Ya se borró de `src/`.
- **Revisión visual:** además de lo pedido, generé capturas de cada vista (dentro del shell, con los paneles ocultos visibles y con cada pestaña/diálogo) usando una clase temporal fuera del repositorio, y corregí lo que se veía mal (formulario de "nuevo canal" aplastado, color del texto de ayuda de los `TextArea`).
- **No se probó en Scene Builder** (no está disponible en esta sesión). Los FXML solo usan construcciones que Scene Builder genera o admite (`fx:define` + `ToggleGroup`, `FXCollections fx:factory`, `TableView fx:constant`, `labelFor="$id"`).
- **Archivos existentes:** no se modificó ningún FXML ni controlador de `auth/` o `shell/`. Los cambios que `git status` muestra en `auth/` ya estaban antes de empezar.

## Decisiones comunes a todas las vistas

- **Encabezado (decisión 1 del documento):** ninguna vista repite el título ni el subtítulo; los pone el shell. Los `h1` internos de los prototipos de configuración ("General", "Canales"…) tampoco se repiten porque ya los dice la pestaña.
- **Raíz:** todas las vistas de `contentPane` usan la clase `content-view` y no fijan tamaño. Solo los tres diálogos (`create-group`, `join-group`, `task-detail`) llevan `prefWidth`.
- **Avisos ocultos agregados:** casi ningún prototipo dibuja el estado de error. Agregué en los formularios una caja `alert-error` oculta (y `alert-success` en perfil) con el mismo patrón de `register.fxml`, para que el controlador tenga dónde mostrar la validación.
- **Acciones por fila:** los botones que el prototipo dibuja dentro de cada fila (Descargar, Copiar, Revocar, Renombrar, Eliminar, Expulsar, Bloquear, Desbloquear, selector de rol) no pueden declararse en el FXML porque las filas no existen; quedan como `// TODO` de cell factory. En `GroupSettingsController` dejé además métodos privados vacíos (`revokeInvite`, `kickMember`…) para que cada acción tenga su lugar.
- **Listas y combos:** los `ComboBox` con opciones fijas del sistema (tipo de archivo, rango de fecha, usos y caducidad de invitación, estado de tarea, tipo de reporte) traen sus opciones en el FXML; los que dependen de datos (canal, responsable) quedan vacíos con `// TODO: modelo`.
- **CSS global nuevo:** `.combo-box`, `.date-picker` y `.text-area` se estilizaron de forma global (44 px de alto, borde y foco de la paleta) para que coincidan con `.text-field`. Las tablas y listas se estilizan solo con clase (`data-table`, `row-list`, `message-list`…).
- **Colores nuevos:** uno solo, `#850E22` (hover de `button-danger`; blanco encima = 10.1:1). En la barra de respuesta el borde usa `-sn-primary` (4.2:1 sobre `-sn-accent-soft`) porque `-sn-accent` daba 2.98:1.
- **Números de caso de uso** en los `// TODO`: se tomaron de `docs/requirements/requisitos.md` (CU-16 enviar, CU-17 responder, CU-18 reaccionar, CU-19 editar y eliminar, CU-20 historial, CU-23 compartir archivo).

## 1. `chat/text-channel.fxml`

- **Agregado:** `emptyMessagesLabel` (estado vacío del canal, oculto); botón solo-icono para cancelar la respuesta en `replyBox`.
- **No representado:** el contenido de cada mensaje (reacciones, referencia a la respuesta, "editado", "Mensaje eliminado", adjunto, "Enviando…") va en el cell factory; dejé las clases CSS listas (`message-author`, `reaction-chip`, `attachment-card`…). El menú "···" es un `ContextMenu` en código, como pide el documento.
- **Decisiones:** sin franja de acciones propia del canal (el prototipo no tiene ninguna y el shell cubre el encabezado). `messageField` se declara antes que los botones por `labelFor`, así que el orden de tabulación es campo → Adjuntar → Enviar.

## 2. `voice/voice-channel.fxml`

- **Agregado:** aviso de "solo escucha" (`listenOnlyBox`), que la tabla pide y el prototipo no dibuja.
- **No representado:** las tarjetas de participante se crean en código dentro de `participantsPane` (clases `participant-card`, `avatar-large`, `status-chip`).
- **Decisiones:** los participantes van en un `TilePane` de 2 columnas (fiel a la cuadrícula del prototipo) y no en un `ListView`. El aviso "voz no disponible" usa `alert-error`. "Silenciar" es un `ToggleButton`; "Salir" se dejó como botón primario, igual que en el prototipo, y no como acción destructiva.

## 3. `files/shared-files.fxml`

- **Agregado:** panel lateral de vista previa (`previewPane`, oculto) con "Descargar" e "Ir al mensaje", que la tabla pide y el prototipo no tiene; selectores "Desde/Hasta" ocultos para la opción "Rango personalizado…".
- **Decisiones:** se conservan las columnas del prototipo (incluida "Canal") y una columna "Acciones" para el botón Descargar por fila. La vista "Cuadrícula" del prototipo queda como `TilePane` oculto que se llena en código. El filtro "Buscar" del prototipo se conservó aunque la tabla no lo menciona.

## 4. `groups/create-group.fxml`

- **Agregado:** selector de imagen del grupo (vista previa + "Elegir imagen…" + "Quitar"), que la tabla pide y el prototipo no tiene.
- **No representado:** el prototipo es un solo diálogo "Agregar un grupo" con pestañas "Crear grupo / Unirse a grupo". La tabla pide dos FXML, así que cada uno es un diálogo independiente con su propio título y sin pestañas; el menú "+" del shell ya separa las dos opciones. Tampoco lleva el botón "✕" (lo da la ventana).

## 5. `groups/join-group.fxml`

- **Decisiones:** misma separación que la vista 4. La vista previa del grupo encontrado está oculta hasta que el código valide la invitación.

## 6. `groups/group-settings.fxml`

- **Agregado:** pestaña "Moderación" (sin prototipo) con la tabla de bloqueados.
- **Decisiones:**
  - `TabPane` con General, Invitaciones, Canales, Miembros y roles, Moderación; "Volver al grupo" y "Abandonar grupo" en una barra superior de la vista (excepción prevista en el documento).
  - La sección "Bloqueados" que el prototipo dibuja dentro de "Miembros y roles" **se movió a Moderación**. "Expulsar" y "Bloquear" siguen en cada fila de miembros, como en el prototipo.
  - "Abandonar grupo" es `button-secondary` (así lo dibuja el prototipo y no está en la lista de acciones destructivas). No tiene capa de confirmación propia: queda como `// TODO` con un diálogo estándar.
  - "Archivar" tampoco tiene capa de confirmación (el prototipo solo la dibuja para eliminar).
  - En la confirmación de eliminar, el título y la etiqueta llevan texto genérico; el controlador debe insertar el nombre del grupo.
  - Los canales son dos `ListView` (texto y voz) en lugar de filas fijas.

## 7. `projects/projects.fxml`

- **Terminología:** el prototipo dice "Terminada"; se usa **"Completada"**, como pide el documento.
- **Agregado:** `noProjectLabel` (sin proyecto seleccionado, oculto).
- **Decisiones:** cada columna es un `ListView` con su encabezado como `status-chip` con icono + texto (círculo, reloj, palomita). "+ Agregar tarea" solo en Pendiente y En progreso, como en el prototipo. Editar o archivar un proyecto (parte de CU-25) no aparece ni en el prototipo ni en la tabla: no se incluyó.

## 8. `projects/task-detail.fxml`

- **Sin prototipo del formulario:** diálogo de 600 px con título, descripción, fecha límite, estado y responsable, en el estilo de `Grupo-crear`.
- **Decisiones:** el conflicto de edición es un panel (`conflictBox`) que **sustituye** al formulario dentro del mismo diálogo, con su comparación "Versión vigente / Tus cambios" y sus botones Cancelar y Recargar; `conflictLabel` es la etiqueta del aviso. El `// TODO` de `onSave` indica cómo intercambiar `defaultButton` y `cancelButton`. "Eliminar tarea" usa `button-danger-outline`; su confirmación queda como `// TODO`.

## 9. `reports/reports.fxml`

- **Decisiones:**
  - Los tipos de reporte son los de la tabla (mensajes por periodo, participación por miembro, tareas por estado), **no** los del prototipo (tareas terminadas por miembro, avance por proyecto, actividad por canal).
  - El botón dice "Consultar" (tabla) en lugar de "Generar" (prototipo).
  - "Exportar" es un `MenuButton` con PDF y CSV, deshabilitado hasta que haya un reporte consultado.
  - `reportTable` no declara columnas: dependen del tipo de reporte y se crean en código.
  - `chartPane` muestra un texto de ayuda hasta que el código coloque la gráfica.

## 10. `notifications/notifications.fxml`

- **No representado:** el prototipo es un panel flotante bajo la campana. `MainController` ya indica que se carga en `contentPane`, así que es una vista completa. El enlace "Ver todas las notificaciones" se omitió porque la vista ya las muestra todas.
- **Agregado:** `emptyStateLabel`.
- **Decisiones:** el filtro "Todas / No leídas" son dos `ToggleButton` unidos; "Marcar todas como leídas" es un botón secundario en la misma barra.

## 11. `profile/profile.fxml`

- **Agregado:** avisos de guardado y de error en las dos tarjetas; error de "las contraseñas no coinciden" (patrón de `register.fxml`).
- **Decisiones:** la cuadrícula de avatares es un `TilePane` vacío que el código llena con un `ToggleButton` por imagen del paquete (`images/avatars`, hoy vacío); no hay ninguna opción para subir archivo. "Cerrar sesión" se dejó como botón primario, igual que en el prototipo. No se agregó "Mostrar contraseña" porque el prototipo no lo tiene.

## Fuera de alcance (no implementado)

- **"Inicio / mensajes directos"** (botón de la columna de grupos en `Main.html`): no corresponde a ningún caso de uso de la tabla.
- **Traducción de mensajes (CU-21)** y **transcripción/resumen (CU-32, CU-33):** fase posterior, sin prototipo.

## Cambios que harían falta en archivos que no debía tocar

- **`ViewManager`:** solo sabe reemplazar la raíz de la ventana. Hacen falta (1) un método para abrir un FXML en un `Stage` modal (`create-group`, `join-group`, `task-detail`) que aplique `theme.css` y entregue el controlador, y (2) una forma de cargar una vista dentro de `contentPane` del shell. Hoy `FXMLLoader.load` estático tampoco permite obtener el controlador para pasarle datos.

## Resumen

**Vistas creadas (11)**, cada una con su FXML y su controlador esqueleto:

| # | FXML | Controlador |
|---|---|---|
| 1 | `chat/text-channel.fxml` | `chat.TextChannelController` |
| 2 | `voice/voice-channel.fxml` | `voice.VoiceChannelController` |
| 3 | `files/shared-files.fxml` | `files.SharedFilesController` |
| 4 | `groups/create-group.fxml` | `groups.CreateGroupController` |
| 5 | `groups/join-group.fxml` | `groups.JoinGroupController` |
| 6 | `groups/group-settings.fxml` | `groups.GroupSettingsController` |
| 7 | `projects/projects.fxml` | `projects.ProjectsController` |
| 8 | `projects/task-detail.fxml` | `projects.TaskDetailController` |
| 9 | `reports/reports.fxml` | `reports.ReportsController` |
| 10 | `notifications/notifications.fxml` | `notifications.NotificationsController` |
| 11 | `profile/profile.fxml` | `profile.ProfileController` |

**Constantes `View` agregadas:** `REGISTER`, `VERIFY_ACCOUNT`, `RECOVER_PASSWORD`, `MAIN`, `TEXT_CHANNEL`, `VOICE_CHANNEL`, `SHARED_FILES`, `CREATE_GROUP`, `JOIN_GROUP`, `GROUP_SETTINGS`, `PROJECTS`, `TASK_DETAIL`, `REPORTS`, `NOTIFICATIONS`, `PROFILE`.

**`theme.css`:** no se borró ni cambió nada de lo existente; se agregaron al final las secciones de chat, voz, archivos, grupos (diálogos y configuración), proyectos, detalle de tarea, reportes, notificaciones y perfil.

**Verificación final:**

- `python3 clients/desktop/docs/check_fxml.py` → 16 archivos, 0 errores, 0 avisos.
- Compilación y `ViewTest` con Maven (usando la copia temporal del pom padre) → sin errores.
- `FxmlSmokeCheck` → las 16 vistas cargan; sin avisos de CSS en la salida.

### Para revisar

1. ~~Falta `clients/desktop/pom.xml`~~ — **resuelto:** restaurado desde `05cfe4b`; falta incluirlo en el próximo commit.
2. **Abrir los FXML en Scene Builder:** no se pudo comprobar aquí.
3. ~~Diálogos de grupo separados~~ — **confirmado por el usuario:** crear grupo y unirse son dos diálogos distintos.
4. **Bloqueados en la pestaña Moderación** (el prototipo los pone en "Miembros y roles"); Expulsar y Bloquear siguen en la fila de cada miembro.
5. **Tipos de reporte:** los de la tabla, distintos a los del prototipo. Botón "Consultar" en lugar de "Generar".
6. **Notificaciones como vista completa** en lugar de panel flotante.
7. **Conflicto de edición** dentro del diálogo de detalle de tarea, sustituyendo al formulario.
8. **Vista previa de archivos** como panel lateral; ¿conviene conservar también el botón Descargar por fila?
9. **Imagen del grupo** al crearlo: se elige con un archivo (`FileChooser`). ¿O debe ser de un paquete predefinido, como la foto de perfil?
10. **Sin confirmación propia** para Abandonar grupo, Archivar grupo, Revocar invitación, Eliminar canal, Expulsar, Bloquear y Eliminar tarea: quedaron como `// TODO` con diálogo estándar.
11. **"Salir" (voz), "Cerrar sesión" y "Abandonar grupo"** no usan el estilo de acción destructiva.
12. **Estilo global de `ComboBox`, `DatePicker` y `TextArea`:** afecta a cualquier vista futura.
13. **`ViewManager`** necesita métodos para diálogos modales y para cargar vistas en `contentPane`.
14. **Paquete de avatares:** `resources/.../images/avatars/` está vacío; la cuadrícula del perfil lo necesita.
