# Instrucciones: vistas JavaFX del cliente de escritorio de Sonimbus

Eres el encargado de terminar los **FXML y los esqueletos de controladores** del cliente de escritorio de Sonimbus. Trabaja de forma autónoma vista por vista, siguiendo este documento al pie de la letra. El usuario no estará mirando; si algo es ambiguo, toma la opción más razonable, regístrala en `clients/desktop/docs/vistas-notas.md` y sigue.

## 1. Contexto

Sonimbus es una plataforma de comunicación y gestión ligera de proyectos (grupos, canales de texto y voz, mensajería en tiempo real, archivos, proyectos y tareas, notificaciones y reportes). Proyecto universitario (Universidad Veracruzana). Los casos de uso se citan como CU-NN.

**Stack:** Java 21, JavaFX 21.0.8, FXML, Maven. Sin `module-info.java` (classpath). Los FXML se editan con **Gluon Scene Builder** y deben abrir sin errores en él. Paquete base: `mx.uv.sonimbus.desktop`.

**Estructura:**

```text
clients/desktop/app/src/main/
├── java/mx/uv/sonimbus/desktop/
│   ├── App.java, Launcher.java
│   └── ui/
│       ├── navigation/View.java         enum de vistas (ruta relativa a resources/.../ui/)
│       ├── navigation/ViewManager.java  carga el FXML y lo pone en la ventana (Stage 1280x800)
│       ├── auth/ ...                    YA HECHO
│       └── shell/MainController.java    YA HECHO
└── resources/mx/uv/sonimbus/desktop/
    ├── ui/auth/*.fxml, ui/shell/main.fxml   YA HECHOS
    └── css/theme.css                        tema completo hasta el shell
```

## 2. Lo que ya está hecho (úsalo como referencia de estilo)

| FXML | Controlador | Estado |
|---|---|---|
| `auth/login.fxml` | `auth.LoginController` | hecho |
| `auth/register.fxml` | `auth.RegisterController` | hecho |
| `auth/verify-account.fxml` | `auth.VerifyAccountController` | hecho |
| `auth/recover-password.fxml` | `auth.RecoverPasswordController` | hecho |
| `shell/main.fxml` | `shell.MainController` | hecho |

**Antes de empezar, lee estos archivos completos:** `shell/main.fxml`, `auth/register.fxml`, `auth/recover-password.fxml`, sus controladores y `css/theme.css`. Copia sus patrones; no los modifiques salvo que sea indispensable (y si lo haces, anótalo).

## 3. Tareas iniciales (una sola vez)

1. Revisa `View.java`. Asegúrate de que tenga estas constantes (agrega las que falten, sin borrar las existentes):
   ```java
   LOGIN("auth/login.fxml"),
   REGISTER("auth/register.fxml"),
   VERIFY_ACCOUNT("auth/verify-account.fxml"),
   RECOVER_PASSWORD("auth/recover-password.fxml"),
   MAIN("shell/main.fxml"),
   ```
2. Revisa `theme.css` con `git diff` (o `git show HEAD:<ruta>`): si la versión anterior del repositorio tenía reglas que ya no están (por ejemplo dentro de `.root` o una clase `.login`), vuelve a agregarlas al final en una sección "Reglas previas". No borres nada del archivo actual.
3. Localiza el `pom.xml` del cliente (probablemente `clients/desktop/pom.xml` o `clients/desktop/app/pom.xml`) y comprueba que compila: `mvn -q -f <pom> compile` (o `./mvnw` si existe wrapper).
4. Ejecuta `python3 clients/desktop/docs/check_fxml.py` y confirma que las 5 vistas hechas dan OK.

## 4. Vistas que faltan (en este orden)

Los prototipos de baja fidelidad están exportados como HTML en `clients/desktop/docs/prototipos/`. **Léelos completos** antes de cada vista. Están en blanco y negro; el color lo da `theme.css`.

| # | FXML (en `resources/.../ui/`) | Controlador (en `java/.../ui/`) | Prototipo(s) | Casos de uso y contenido mínimo |
|---|---|---|---|---|
| 1 | `chat/text-channel.fxml` | `chat.TextChannelController` | `Main.html` (solo el área central) | CU-16 a CU-20, CU-23: `ListView` de mensajes, barra de respuesta (CU-17) oculta por defecto, área de escritura, botones adjuntar y enviar |
| 2 | `voice/voice-channel.fxml` | `voice.VoiceChannelController` | `Voz.html` | CU-22: lista de participantes con estado de micrófono, botones silenciar/activar y salir, aviso de "solo escucha" |
| 3 | `files/shared-files.fxml` | `files.SharedFilesController` | `Archivos.html`, `Archivos-vacio.html` | CU-24: filtros por canal, tipo y fecha; tabla nombre, tipo, tamaño, autor, fecha; vista previa; botones descargar e ir al mensaje; estado vacío |
| 4 | `groups/create-group.fxml` | `groups.CreateGroupController` | `Grupo-crear.html` | CU-08 diálogo: nombre (obligatorio), descripción, imagen; crear y cancelar |
| 5 | `groups/join-group.fxml` | `groups.JoinGroupController` | `Grupo-unirse.html` | CU-11 diálogo: código o enlace de invitación; unirse y cancelar |
| 6 | `groups/group-settings.fxml` | `groups.GroupSettingsController` | `Config-general.html`, `Config-eliminar.html`, `Config-invitaciones.html`, `Config-canales.html`, `Config-miembros.html` | `TabPane` con pestañas: General (CU-09: datos, archivar, eliminar con confirmación por nombre), Invitaciones (CU-10: tabla con usos restantes y caducidad, crear, revocar), Canales (CU-13: crear, editar, eliminar, tipo texto/voz), Miembros y roles (CU-14), Moderación (CU-15: bloqueados, retirar bloqueo); botón "Abandonar grupo" (CU-12) |
| 7 | `projects/projects.fxml` | `projects.ProjectsController` | `Proyectos.html` | CU-25 y CU-26: lista de proyectos; tareas en columnas Pendiente / En progreso / Completada con responsable y fecha límite; crear proyecto, crear tarea. (El prototipo dice "Terminada"; usa **"Completada"**, que es el término de los casos de uso, y anótalo.) |
| 8 | `projects/task-detail.fxml` | `projects.TaskDetailController` | `Proyectos-conflicto.html` (aviso de conflicto); **no hay prototipo del formulario de detalle**: diséñalo como diálogo modal con el estilo de `Grupo-crear.html` | CU-26 a CU-28: título, descripción, fecha límite, responsable (CU-27), estado (CU-28), guardar, eliminar; etiqueta para aviso de conflicto de edición (la comparación del prototipo de conflicto va como panel oculto dentro de este FXML) |
| 9 | `reports/reports.fxml` | `reports.ReportsController` | `Reportes.html` | CU-29 y CU-30: tipo de reporte (mensajes por periodo, participación por miembro, tareas por estado), fecha inicio y fin, consultar, `StackPane fx:id="chartPane"` para la gráfica, tabla de datos, exportar PDF/CSV |
| 10 | `notifications/notifications.fxml` | `notifications.NotificationsController` | `Notificaciones.html` | CU-31: lista de notificaciones (más reciente primero, leídas y no leídas), "Marcar todas como leídas" |
| 11 | `profile/profile.fxml` | `profile.ProfileController` | `Perfil.html`, `Perfil-cerrar-sesion.html` | CU-07: nombre visible; foto elegida de un **paquete predefinido** (cuadrícula de avatares, no subir archivo); cambiar contraseña (actual, nueva, confirmación); CU-06 "Cerrar sesión" |

Las variantes de estado de un prototipo (`-vacio`, `-eliminar`, `-conflicto`, `-cerrar-sesion`, `Login-error`) van **dentro del mismo FXML** como nodos ocultos (`visible="false" managed="false"`), no como FXML aparte.

## 5. Cómo leer los prototipos

- Cada prototipo dibuja la **ventana completa**. Ignora: la barra de título gris (minimizar/maximizar/cerrar), la etiqueta punteada "Pantalla N · CU-…", la columna de grupos, la columna de canales y la barra superior con campana y perfil. **Todo eso ya lo pone `shell/main.fxml`.**
- Reproduce **solo lo que está debajo de la barra superior, dentro del área central**. Eso es lo que se carga en `contentPane`.
- **Excepción, `group-settings`:** el prototipo reemplaza la columna de canales por una navegación (General / Canales / Invitaciones / Miembros y roles + "Volver al grupo" + "Abandonar grupo"). La tabla manda un `TabPane`: usa `TabPane` en el área central con las cinco pestañas (incluida **Moderación**, que no tiene prototipo: diséñala en el mismo estilo que "Miembros y roles"), y pon "Volver al grupo" y "Abandonar grupo" en una barra superior o inferior de la vista. Anótalo.
- Los diálogos (`create-group`, `join-group`) se abren en un `Stage` modal aparte: su raíz sí puede tener `prefWidth` (unos 480–520) y su propio padding; usa `defaultButton` en la acción principal y `cancelButton` en Cancelar.
- Las confirmaciones (eliminar grupo, cerrar sesión) van como capa superpuesta oculta dentro del mismo FXML: raíz `StackPane` con el contenido normal + un `StackPane` de fondo semitransparente (`styleClass="modal-scrim"`) con la tarjeta del diálogo, oculto por defecto.
- Menús contextuales (opciones de un mensaje: Responder, Reaccionar, Editar, Eliminar) **no** van en el FXML: déjalos como `// TODO` en el controlador (se crean con `ContextMenu` en código).
- **Los datos de ejemplo del prototipo no se copian** (nombres, mensajes, archivos). Las listas y tablas quedan vacías; sus celdas se describen con `// TODO` de cell factory en `initialize()`.

## 6. Decisiones ya tomadas (respétalas)

1. **Encabezado de la vista:** el título y subtítulo de la vista actual (p. ej. `# general` / descripción, "Archivos compartidos", "Configuración del grupo") se muestran en la barra superior del shell (`sectionTitleLabel`, `sectionSubtitleLabel` en `MainController`). Las vistas que se cargan en `contentPane` **no repiten esa fila**. En `text-channel.fxml` el "encabezado del canal" de la tabla queda cubierto por el shell; si quieres acciones propias del canal, ponlas en una franja delgada opcional. Anótalo en las notas como decisión.
2. **Vistas dentro de `contentPane`:** sin `prefWidth`/`prefHeight`/`minWidth` fijos en la raíz; deben estirarse. Usa `BorderPane`, `VBox`, `HBox`, `GridPane`, `StackPane`, `TabPane`, `ScrollPane`, `SplitPane`. Nada de `AnchorPane` con coordenadas.
3. **Paleta:** inspirada en Ubuntu (naranja + berenjena), definida como variables en `.root` de `theme.css`. Usa SIEMPRE las variables (`-sn-primary`, `-sn-text`, `-sn-text-muted`, `-sn-accent-soft`, `-sn-error`, `-sn-success`, etc.). El naranja puro `-sn-accent` nunca se usa para texto: solo foco, indicadores y bordes.
4. **Estados de tareas** (Pendiente / En progreso / Completada) y estados en general: se distinguen por **texto + icono**, no solo por color. Combinaciones permitidas (contraste ya verificado):
   - neutro: fondo `#F1ECEA`, texto `-sn-text`
   - en curso / destacado: fondo `-sn-accent-soft`, texto `-sn-accent-soft-text` (6.7:1)
   - completado / éxito: fondo `-sn-success-bg`, texto `-sn-success` (5.9:1)
   - error / peligro: fondo `-sn-error-bg`, texto `-sn-error`
   Si necesitas un color nuevo, verifica contraste (texto ≥ 4.5:1, componentes y bordes ≥ 3:1) con un script y pon la razón en un comentario junto al color.
5. **Acciones destructivas** (Eliminar grupo, Revocar, Eliminar tarea, Expulsar): clase `button-danger` (fondo `-sn-error`, texto blanco = 8:1) o `button-danger-outline` (borde y texto `-sn-error`). Créalas en el CSS la primera vez que las necesites.
6. **Iconos:** `SVGPath` con `styleClass="icon-stroke"` (o `icon-stroke-light` sobre fondo oscuro). Copia los `d="..."` de los `<svg>` del prototipo; un `<circle cx cy r>` se convierte a path con dos arcos (ver el engrane en `main.fxml`). Nada de imágenes ni emoji.
7. **Avatares:** `StackPane` con clase `avatar-small` (o una nueva `avatar-large`), un `Label` de iniciales y un `ImageView` encima, como en `main.fxml`.

## 7. Reglas obligatorias (del usuario)

1. **Fidelidad al prototipo:** respeta la distribución. Si un prototipo no muestra un elemento de la tabla, agrégalo de forma discreta y anótalo.
2. **Solo vista:** los controladores son esqueletos: campos `@FXML private` con su `fx:id`, `@FXML private void initialize()` y un método `@FXML private void onXxx()` vacío por cada acción, con un comentario `// TODO CU-NN: ...` que diga qué debe hacer. Nada de lógica de negocio, red ni datos falsos. Usa `ListView<Object>` / `TableView<Object>` / `TableColumn<Object, Object>` con `// TODO: modelo` cuando el tipo no exista todavía.
3. **Nombres:** `fx:id` en inglés y camelCase con sufijo del tipo (`emailField`, `sendButton`, `messagesList`, `filesTable`, `typeCombo`, `startDatePicker`, `replyBox`); manejadores `onSend`, `onCreateGroup`, etc.; **textos visibles en español**.
4. **Layouts redimensionables** (ver decisión 2).
5. **Imports explícitos** en cada FXML (nada de `javafx.scene.control.*`) y `xmlns="http://javafx.com/javafx/21"` + `xmlns:fx="http://javafx.com/fxml/1"` en la raíz.
6. **Estilo:** nada de `style="..."`; solo `styleClass`. Las clases nuevas se agregan al final de `theme.css`, en una sección por vista con un encabezado de comentario (`/* ===== Chat: canal de texto ===== */`). No dupliques clases que ya existen; reutiliza `button-primary`, `button-secondary`, `field-label`, `hint-text`, `alert-*`, `badge`, `icon-button`, `empty-state`, etc.
7. **Accesibilidad (WCAG 2.2):**
   - Cada control con `text` o `accessibleText`; los botones solo-icono llevan `accessibleText` **y** `Tooltip`.
   - Cada `Label` de un campo usa `labelFor="$campo"`. **El campo debe declararse ANTES que su `Label` en el documento** o el FXML falla; por eso los formularios usan `GridPane` con el campo primero y el orden visual con `GridPane.rowIndex` (copia el patrón de `register.fxml`).
   - Ayudas de un campo van también en `accessibleHelp`.
   - `defaultButton="true"` en la acción principal y `cancelButton="true"` en Cancelar donde aplique. Si hay dos acciones principales en paneles alternos, solo una lleva `defaultButton` en el FXML y el TODO del controlador dice cuándo intercambiarlo.
   - Objetivos táctiles ≥ 44 px de alto en botones principales.

## 8. Procedimiento por cada vista

1. Lee el/los prototipo(s) completos.
2. Escribe el FXML.
3. Escribe el controlador (mismo paquete que la carpeta: `ui/chat/TextChannelController.java` → `package mx.uv.sonimbus.desktop.ui.chat;`), con imports explícitos.
4. Agrega la constante al enum `View` (p. ej. `TEXT_CHANNEL("chat/text-channel.fxml")`, `VOICE_CHANNEL`, `SHARED_FILES`, `CREATE_GROUP`, `JOIN_GROUP`, `GROUP_SETTINGS`, `PROJECTS`, `TASK_DETAIL`, `REPORTS`, `NOTIFICATIONS`, `PROFILE`).
5. Agrega las clases CSS nuevas al final de `theme.css` en su sección.
6. Verifica:
   - `python3 clients/desktop/docs/check_fxml.py <carpeta>` → debe dar 0 errores.
   - Compila con Maven → sin errores.
7. Agrega una sección a `clients/desktop/docs/vistas-notas.md` (créalo si no existe) con:
   - qué agregaste que no estaba en el prototipo,
   - qué del prototipo no pudiste representar y por qué,
   - decisiones que tomaste por tu cuenta.
8. Pasa a la siguiente vista.

## 9. Verificación final (después de la vista 11)

1. `python3 clients/desktop/docs/check_fxml.py` sobre todo → 0 errores.
2. Prueba de humo en tiempo de ejecución (detecta propiedades mal escritas que el verificador no ve):
   - Copia `clients/desktop/docs/FxmlSmokeCheck.java` a `clients/desktop/app/src/main/java/mx/uv/sonimbus/desktop/dev/FxmlSmokeCheck.java`.
   - Ejecútalo, por ejemplo: `mvn -q -f <pom> compile exec:java -Dexec.mainClass=mx.uv.sonimbus.desktop.dev.FxmlSmokeCheck` (si el pom usa `javafx-maven-plugin`, también sirve configurar su `mainClass` temporalmente).
   - Corrige todas las vistas que den `ERR` y revisa en la salida los avisos de CSS (`WARNING: CSS Error` / `Could not resolve`), y corrígelos también.
   - **Borra** `FxmlSmokeCheck.java` de `src/` al terminar (el original se queda en `docs/`).
3. Al final de `vistas-notas.md` escribe un resumen: lista de vistas creadas, constantes `View` agregadas, y una lista "Para revisar" con las decisiones que el usuario debería confirmar.

## 10. Límites

- No hagas commits ni push; el usuario revisará los cambios con `git diff`.
- No modifiques `App.java`, `Launcher.java`, `ViewManager.java` ni el `pom.xml` (si crees que hace falta un cambio, por ejemplo un método para abrir diálogos, descríbelo en las notas).
- No instales dependencias nuevas.
- No inventes casos de uso: si algo del prototipo no está en la tabla (p. ej. "Inicio / mensajes directos"), no lo implementes y anótalo.
