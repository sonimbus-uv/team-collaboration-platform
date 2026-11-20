# Fork del SDK comunitario de LiveKit para Java

Fork de [Trirrin/livekit-java-sdk](https://github.com/Trirrin/livekit-java-sdk) (tag `v0.1.4`), adoptado como código propio según la conclusión de la prueba de concepto de voz.

## Por qué está aquí

El SDK no funciona sin modificaciones y no tiene mantenimiento activo. Se incorpora al repositorio para corregir sus defectos en lugar de rodearlos desde la aplicación.

## Reglas

- Conservar el archivo `LICENSE` original (Apache-2.0) y registrar en este README cada cambio respecto al original.
- Fijar `dev.onvoid.webrtc:webrtc-java` en 0.18.0.
- Solo `clients/desktop/app` puede depender de este módulo, y solo a través de la interfaz de voz propia.

## Defectos conocidos (estado original; los cambios aplicados están abajo)

Identificados en la prueba de concepto (https://github.com/sonimbus-uv/poc-voice-javafx, `REPORT.md`):

Secciones 7.1 a 7.6 del `REPORT.md` de la PoC (actualizado el 2026-11-18):

1. No publica el micrófono ni la cámara: el cliente nunca envía la oferta del publisher; hay que llamar a `onPublisherNegotiationNeeded()` tras publicar (7.1).
2. Liberar la pista local después de cerrar el cliente termina la JVM (SIGSEGV); el orden correcto es quitar la pista, liberarla y cerrar (7.2).
3. Si el servidor expulsa al cliente, este no lo detecta y queda bloqueado (7.3).
4. Al cerrar, sobre todo tras sesiones largas o con video, el cliente a veces aborta (`Pure virtual function called!`) o queda bloqueado en `disposeInternal`. **Causa identificada:** `PeerConnectionEngine.close()` no está protegido y se ejecuta dos veces a la vez (desde el hilo que cierra y desde el hilo del WebSocket al llegar `leave`). Un parche de esa clase dio 0 de 60 abortos con video frente a 4 de 30 sin él, y 0 de 6 frente a 3 de 6 en sesiones largas de audio (p = 0,09, no concluyente) (7.4). **Debe confirmarse con más muestra y con sesiones largas con video antes de integrar.** Ver D-31.
5. `createAudioTrack` y `createVideoTrack` devuelven `null` y tragan las excepciones; no hay configuración de eco ni ruido ni `getStats()` (7.5).
6. `LocalVideoTrack` no guarda la fuente de la cámara y `setCameraEnabled(false)` solo silencia la pista: la captura podría seguir abierta (hipótesis, sin reproducir) (7.6).
7. La pantalla compartida no está implementada (`setScreenShareEnabled` está vacío). Queda fuera del MVP (D-31).

## Contenido y construcción

Código importado del tag `v0.1.4` (commit `0fa2340`) en el commit `c3d45d7`, sin cambios. Se excluyeron `README.md` y `README_CN.md` originales (el primero se conserva como `UPSTREAM_README.md`) y `.github/`. El build es de Gradle (módulos `protocol`, `core`, `signaling`, `rtc` y `examples`); `app/` aún no lo usa.

Verificación local: `gradle test spotlessCheck` desde esta carpeta. Las pruebas de `rtc` cargan las librerías nativas de `webrtc-java` 0.18.0 (`linux-x86_64`) solo en `testRuntimeOnly`.

## Convenciones del código importado

Los archivos importados conservan sus comentarios y Javadoc originales (decisión del equipo para el código de terceros, que facilita comparar con el original). El código nuevo o modificado respecto al original no lleva comentarios (§28.1).

## Licencia

El SDK declara Apache-2.0 en su README (sección License) y en `build.gradle.kts`, pero el tag `v0.1.4` no incluye un archivo `LICENSE`. Se añadió `LICENSE` con el texto estándar de Apache License 2.0 (copia sin modificar de `/usr/share/common-licenses/Apache-2.0`, de Debian). No es un archivo del original: no contiene aviso de copyright de los autores.

## Cambios respecto al original

Un commit por cambio, en el orden de aplicación:

1. **Cierre serializado e idempotente** (defecto 4). `PeerConnectionEngine.initialize()` y `close()` son `synchronized`; `close()` no hace nada si ya se cerró y `initialize()` permite abrir de nuevo. El cierre sigue ejecutándose en el hilo del llamante: moverlo a otro hilo empeora el aborto (PoC, 7.4). Pruebas: `PeerConnectionEngineCloseTest`.
2. **`webrtc-java` fijado en 0.18.0** (`rtc/build.gradle.kts`; el original fijaba 0.14.0).
3. **Publicación sin workaround** (defecto 1, 7.1). `RtcClient.publishAudioTrack()` y `publishVideoTrack()` llaman a `onPublisherNegotiationNeeded()` tras añadir la pista, de modo que la oferta del publisher se envía sin que la aplicación lo haga.
4. **Orden de cierre** (defecto 2, 7.2). `RtcClient.disconnect()` y `onLeave()` primero quitan la pista del emisor y la liberan, y después cierran el motor. `LocalAudioTrack.dispose()` y `LocalVideoTrack.dispose()` son idempotentes, así que la aplicación puede liberarlas antes sin provocar un doble `dispose`.
5. **Expulsión y pérdida de la señal** (defecto 3, 7.3). Dos causas en el código: `SignalClient.reconnect()` salía de inmediato si el estado ya era `RECONNECTING`, por lo que el reintento programado nunca se ejecutaba y el cliente no llegaba a `FAILED`; y `RoomSignalHandler` ignoraba `FAILED`. Ahora el reintento se ejecuta, `FAILED` libera las pistas y el motor y `Room` notifica `onDisconnected(SIGNAL_CLOSE)` una sola vez.
6. **Errores visibles** (defecto 5, 7.5). `MediaDevicesHelper.createAudioTrack()` y `createVideoTrack()` lanzan `IllegalStateException` en lugar de devolver `null`; `RtcClient.onError(String)` ya no está vacío. Se añade `RoomListener.onError(Room, Exception)` y `Room` reenvía los errores de señalización. Pruebas: `RtcClientErrorTest` y `RoomSignalHandlerTest`.
7. **Fuente de la cámara** (defecto 6, 7.6). `LocalVideoTrack` guarda el `VideoDeviceSource`; `setMuted(true)` detiene la captura, `setMuted(false)` la reanuda y `dispose()` la detiene y libera. Si la creación de la pista falla, la fuente se libera. Pruebas: `LocalTrackDisposeTest`.

## Qué solo puede validarse con LiveKit levantado

- Defecto 1: que el servidor llegue a `mediaTrack published` sin llamar a `onPublisherNegotiationNeeded()` desde la aplicación (audio y cámara).
- Defecto 3: que un cliente expulsado (`DUPLICATE_IDENTITY`) reciba `onDisconnected`. En el peor caso la notificación tarda unos 31 s (5 reintentos con espera creciente de 1 s a 16 s) si el servidor solo cierra el WebSocket sin enviar `LeaveRequest`.
- Defecto 4: que el cierre ya no aborte ni se cuelgue. Falta repetir el A/B de la PoC (sesiones largas de audio y de video, varios pares de clientes) para confirmar la tasa; las pruebas de este módulo solo comprueban que el cierre doble no lanza excepciones.
- Defecto 2: el orden `quitar pista, liberar, cerrar` con un cliente conectado y publicando.
- Defecto 6: que la cámara quede libre al silenciar y al salir (LED y apertura por otro proceso), con una cámara física.

## Pendientes

- Defecto 7 (pantalla compartida): fuera del alcance, T-15.
- Sin cancelación de eco ni supresión de ruido, sin `getStats()` y sin elección de dispositivo de salida (7.5, sin parche).
- Integración con `app/` a través de la interfaz de voz propia: aún no.
