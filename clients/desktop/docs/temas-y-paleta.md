# Temas claro y oscuro de escritorio

`theme.css` contiene la paleta aprobada como colores consultables `-sn-*` de
JavaFX. Claro es el valor inicial. Los FXML siguen utilizando la misma hoja y
las mismas clases de componentes; Java y FXML no introducen colores fijos.

El selector **Apariencia** está en las cuatro pantallas de autenticación y en la
barra principal. Se puede operar con teclado. No depende del estado del backend.

## Aplicación y persistencia

`ViewManager` registra cada raíz FXML en `ThemeManager`, tanto al navegar como al
abrir contenido o diálogos. El gestor aplica `theme-light` o `theme-dark` y
sincroniza los selectores. También registra las ventanas emergentes de JavaFX y
su contenido visual para que menús y listas abiertas cambien en vivo.

La elección se guarda con `java.util.prefs.Preferences`, el mismo mecanismo
que usa el cliente para las preferencias de navegación de demostración. La clave
`appearance` está en el nodo del paquete `ui.theme`. Una preferencia inexistente
o inválida utiliza Claro. Si no puede guardarse, el tema sigue funcionando durante
la sesión y se emite un aviso sin datos sensibles.

Las raíces se mantienen mediante referencias débiles para no retener vistas que
ya no se usan. El mecanismo no requiere editar configuración ni dependencias.

## Estados y contraste

Los botones principales utilizan morado/lavanda con su color aprobado de texto e
iconos. La selección utiliza fondo violeta, barra lateral y negritas; conserva los
contadores. Las confirmaciones y reacciones positivas utilizan menta con texto o
marca visible. Los errores conservan su texto e icono junto al formulario. El foco
usa el color de acción con borde visible y los controles deshabilitados conservan
texto legible.

Ratios calculados mediante luminancia relativa sRGB de los pares utilizados:

| Par | Claro | Oscuro |
|---|---:|---:|
| Texto principal / fondo principal | 15.81:1 | 16.12:1 |
| Texto secundario / superficie | 8.25:1 | 9.37:1 |
| Texto e iconos / acción principal | 8.98:1 | 9.16:1 |
| Texto principal / selección | 12.94:1 | 11.43:1 |
| Texto e iconos / menta | 6.78:1 | 9.67:1 |
| Texto de error / fondo de error | 5.94:1 | 8.57:1 |
| Borde de control / superficie | 5.02:1 | 5.03:1 |
| Indicador de selección / fondo seleccionado | 6.98:1 | 6.92:1 |
| Foco / fondo principal | 8.53:1 | 9.76:1 |

El divisor decorativo no se usa como único indicador de un control o estado.

## Validación reproducible

En la VM con Java 21 y Maven, desde `clients/desktop`:

```bash
mvn -pl app verify
```

`ThemeIntegrationTest` utiliza JavaFX real con TestFX y Monocle, sin pantalla:

- Carga las 16 vistas en ambos temas y comprueba el contraste de sus etiquetas
  visibles habilitadas frente a los fondos calculados por JavaFX.
- Comprueba colores de login, error, selección y confirmación, así como el
  contraste de la acción principal en estado normal, hover y deshabilitado.
- Usa Tab para comprobar un indicador de foco de al menos 3:1.
- Abre el diálogo real de crear grupo y el menú del shell; cambia el tema
  mientras están abiertos.
- Comprueba restauración mediante una instancia nueva del gestor y lectura de la
  preferencia guardada desde otra JVM. Los tests usan preferencias aisladas.
- Guarda capturas en `app/target/theme-previews`: login, error, shell y diálogo,
  en Claro y Oscuro. Los datos de canales usados en las capturas son fixtures.

Se inspeccionaron las capturas generadas. Queda pendiente comprobar manualmente
las decoraciones nativas de ventanas, escalado de Windows y renderizado con GPU
en una sesión de escritorio real. La prueba de persistencia entre JVM no sustituye
esa revisión manual. Los tests verifican etiquetas y pares críticos, no cada píxel
de todas las posibles combinaciones de datos del backend.
