package io.livekit.sdk.rtc;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class LocalTrackDisposeTest {

  @Test
  void audioTrackDisposeIsIdempotent() {
    LocalAudioTrack track = new LocalAudioTrack("audio-1", "mic", null);
    assertDoesNotThrow(track::dispose);
    assertDoesNotThrow(track::dispose);
  }

  @Test
  void videoTrackDisposeIsIdempotent() {
    LocalVideoTrack track = new LocalVideoTrack("video-1", "cam", null, 640, 480);
    assertDoesNotThrow(track::dispose);
    assertDoesNotThrow(track::dispose);
  }

  @Test
  void videoTrackMuteAfterDisposeKeepsState() {
    LocalVideoTrack track = new LocalVideoTrack("video-1", "cam", null, 640, 480);
    track.dispose();
    assertDoesNotThrow(() -> track.setMuted(true));
    assertTrue(!track.isMuted());
  }
}
