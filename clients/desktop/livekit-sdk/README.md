# Fork del SDK comunitario de LiveKit para Java

Fork de [Trirrin/livekit-java-sdk](https://github.com/Trirrin/livekit-java-sdk) (tag `v0.1.4`), adoptado como código propio según la conclusión de la prueba de concepto de voz.

## Por qué está aquí

El SDK no funciona sin modificaciones y no tiene mantenimiento activo. Se incorpora al repositorio para corregir sus defectos en lugar de rodearlos desde la aplicación.

## Reglas

- Conservar el archivo `LICENSE` original (Apache-2.0) y registrar en este README cada cambio respecto al original.
- Fijar `dev.onvoid.webrtc:webrtc-java` en 0.18.0.
- Solo `clients/desktop/app` puede depender de este módulo, y solo a través de la interfaz de voz propia.

## Defectos conocidos

Identificados en la prueba de concepto (https://github.com/sonimbus-uv/poc-voice-javafx, `REPORT.md`):

Secciones 7.1 a 7.6 del `REPORT.md` de la PoC (actualizado el 2026-11-18):

1. No publica el micrófono ni la cámara: el cliente nunca envía la oferta del publisher; hay que llamar a `onPublisherNegotiationNeeded()` tras publicar (7.1).
2. Liberar la pista local después de cerrar el cliente termina la JVM (SIGSEGV); el orden correcto es quitar la pista, liberarla y cerrar (7.2).
3. Si el servidor expulsa al cliente, este no lo detecta y queda bloqueado (7.3).
4. Al cerrar, sobre todo tras sesiones largas o con video, el cliente a veces aborta (`Pure virtual function called!`) o queda bloqueado en `disposeInternal`. **Causa identificada:** `PeerConnectionEngine.close()` no está protegido y se ejecuta dos veces a la vez (desde el hilo que cierra y desde el hilo del WebSocket al llegar `leave`). Un parche de esa clase dio 0 de 60 abortos con video frente a 4 de 30 sin él, y 0 de 6 frente a 3 de 6 en sesiones largas de audio (p = 0,09, no concluyente) (7.4). **Debe confirmarse con más muestra y con sesiones largas con video antes de integrar.** Ver D-31.
5. `createAudioTrack` y `createVideoTrack` devuelven `null` y tragan las excepciones; no hay configuración de eco ni ruido ni `getStats()` (7.5).
6. `LocalVideoTrack` no guarda la fuente de la cámara y `setCameraEnabled(false)` solo silencia la pista: la captura podría seguir abierta (hipótesis, sin reproducir) (7.6).
7. La pantalla compartida no está implementada (`setScreenShareEnabled` está vacío). Queda fuera del MVP (D-31).

## Cambios respecto al original

Ninguno todavía. El punto de partida para el defecto 4 es la copia parcheada de `PeerConnectionEngine` de la PoC (`src/main/java/io/livekit/sdk/rtc/PeerConnectionEngine.java`, con `close()` sincronizado e idempotente).
