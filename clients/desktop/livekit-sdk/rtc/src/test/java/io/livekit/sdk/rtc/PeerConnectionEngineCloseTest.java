package io.livekit.sdk.rtc;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class PeerConnectionEngineCloseTest {

  @Test
  void closeWithoutInitializeDoesNotThrow() {
    PeerConnectionEngine engine = new PeerConnectionEngine();
    assertDoesNotThrow(engine::close);
    assertDoesNotThrow(engine::close);
  }

  @Test
  void repeatedCloseAfterInitializeDoesNotThrow() {
    PeerConnectionEngine engine = new PeerConnectionEngine();
    engine.initialize(List.of());
    assertDoesNotThrow(engine::close);
    assertDoesNotThrow(engine::close);
  }

  @Test
  void concurrentCloseCallsCompleteWithoutFailure() throws Exception {
    PeerConnectionEngine engine = new PeerConnectionEngine();
    engine.initialize(List.of());
    int callers = 8;
    ExecutorService pool = Executors.newFixedThreadPool(callers);
    CountDownLatch start = new CountDownLatch(1);
    AtomicInteger failures = new AtomicInteger();
    for (int i = 0; i < callers; i++) {
      pool.submit(
          () -> {
            try {
              start.await();
              engine.close();
            } catch (Throwable t) {
              failures.incrementAndGet();
            }
          });
    }
    start.countDown();
    pool.shutdown();
    assertTrue(pool.awaitTermination(30, TimeUnit.SECONDS));
    assertTrue(failures.get() == 0);
  }

  @Test
  void engineCanBeInitializedAgainAfterClose() {
    PeerConnectionEngine engine = new PeerConnectionEngine();
    engine.initialize(List.of());
    engine.close();
    engine.initialize(List.of());
    assertDoesNotThrow(engine::close);
  }
}
