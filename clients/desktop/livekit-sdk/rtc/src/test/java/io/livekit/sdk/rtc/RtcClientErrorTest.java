package io.livekit.sdk.rtc;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.livekit.sdk.Room;
import io.livekit.sdk.RoomListener;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.Test;

class RtcClientErrorTest {

  @Test
  void engineErrorsReachRoomListeners() {
    RtcClient client = new RtcClient();
    AtomicReference<Exception> received = new AtomicReference<>();
    client
        .getRoom()
        .addListener(
            new RoomListener() {
              @Override
              public void onError(Room room, Exception error) {
                received.set(error);
              }
            });

    client.onError("Failed to create offer: no transport");

    assertTrue(received.get() instanceof IllegalStateException);
    assertEquals("Failed to create offer: no transport", received.get().getMessage());
  }

  @Test
  void disconnectWithoutConnectionDoesNotThrow() {
    RtcClient client = new RtcClient();
    client.disconnect();
    client.disconnect();
  }
}
