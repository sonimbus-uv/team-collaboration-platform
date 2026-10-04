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

1. No publica el micrófono: descarta la renegociación antes de que exista la pista de audio.
2. Liberar la pista local después de cerrar el cliente termina la JVM (SIGSEGV).
3. Si el servidor expulsa al cliente, este no lo detecta y queda bloqueado.
4. Al cerrar tras sesiones de más de 7 minutos, el cliente a veces aborta o queda bloqueado. Causa no localizada. **Debe resolverse antes de integrar la voz en la aplicación.**

## Cambios respecto al original

Ninguno todavía.
