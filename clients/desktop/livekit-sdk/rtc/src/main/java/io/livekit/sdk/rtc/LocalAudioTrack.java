package io.livekit.sdk.rtc;

import dev.onvoid.webrtc.media.audio.AudioTrack;

/** Wrapper for a local audio track to be published. */
public class LocalAudioTrack {
  private final String id;
  private final String name;
  private final AudioTrack nativeTrack;
  private String sid;
  private boolean muted;
  private boolean disposed;

  public LocalAudioTrack(String id, String name, AudioTrack nativeTrack) {
    this.id = id;
    this.name = name;
    this.nativeTrack = nativeTrack;
    this.muted = false;
  }

  public String getId() {
    return id;
  }

  public String getName() {
    return name;
  }

  public String getSid() {
    return sid;
  }

  public void setSid(String sid) {
    this.sid = sid;
  }

  public AudioTrack getNativeTrack() {
    return nativeTrack;
  }

  public boolean isMuted() {
    return muted;
  }

  public void setMuted(boolean muted) {
    this.muted = muted;
    if (nativeTrack != null) {
      nativeTrack.setEnabled(!muted);
    }
  }

  public synchronized void dispose() {
    if (disposed) {
      return;
    }
    disposed = true;
    if (nativeTrack != null) {
      nativeTrack.dispose();
    }
  }
}
