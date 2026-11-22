import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  CaptureController,
  captureDelayMs,
  failureBackoffMs,
  type CaptureDeps,
} from "../src/lib/snapshot-logic.ts";

type Timer = { at: number; fn: () => void; id: number };

function env(opts: { enabled?: () => boolean; capture?: () => Promise<void> } = {}) {
  let t = 1_000_000;
  let nextId = 1;
  const timers: Timer[] = [];
  const delays: number[] = [];
  const log = { captures: 0 };
  const deps: CaptureDeps = {
    isEnabled: opts.enabled ?? (() => true),
    capture:
      opts.capture ??
      (async () => {
        log.captures++;
      }),
    now: () => t,
    schedule: (fn, ms) => {
      delays.push(ms);
      const timer = { at: t + ms, fn, id: nextId++ };
      timers.push(timer);
      return timer.id;
    },
    cancel: (h) => {
      const i = timers.findIndex((x) => x.id === h);
      if (i !== -1) timers.splice(i, 1);
    },
  };
  async function advance(ms: number) {
    const end = t + ms;
    for (;;) {
      timers.sort((a, b) => a.at - b.at);
      const next = timers[0];
      if (!next || next.at > end) break;
      timers.shift();
      t = Math.max(t, next.at);
      next.fn();
      await flushMicrotasks();
    }
    t = end;
  }
  return { deps, advance, delays, log, timers, now: () => t };
}

const flushMicrotasks = async () => {
  for (let i = 0; i < 10; i++) await Promise.resolve();
};

function deferred() {
  let resolve!: () => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("failureBackoffMs and captureDelayMs", () => {
  it("grows exponentially and is capped", () => {
    assert.equal(failureBackoffMs(1), 5000);
    assert.equal(failureBackoffMs(2), 10000);
    assert.equal(failureBackoffMs(3), 20000);
    assert.equal(failureBackoffMs(50), 300000);
  });
  it("never schedules before notBefore", () => {
    assert.equal(
      captureDelayMs({ now: 0, firstDirtyAt: 0, lastOutputAt: 0, debounceMs: 3000, maxWaitMs: 15000, notBefore: 9000 }),
      9000
    );
  });
});

describe("CaptureController scheduling", () => {
  it("captures once, a debounce after the last output", async () => {
    const e = env();
    const c = new CaptureController(e.deps);
    c.markOutput();
    await e.advance(2999);
    assert.equal(e.log.captures, 0);
    await e.advance(2);
    assert.equal(e.log.captures, 1);
  });

  it("does nothing while the setting is off", async () => {
    const e = env({ enabled: () => false });
    const c = new CaptureController(e.deps);
    c.markOutput();
    assert.equal(e.timers.length, 0);
    await c.flush();
    assert.equal(e.log.captures, 0);
  });

  it("still captures under continuous output once the max wait passes", async () => {
    const e = env();
    const c = new CaptureController(e.deps);
    for (let i = 0; i < 20; i++) {
      c.markOutput();
      await e.advance(1000); // output every second keeps resetting the debounce
    }
    assert.ok(e.log.captures >= 1, "max wait must force a capture");
  });
});

describe("CaptureController failures", () => {
  it("backs off instead of retrying on every output chunk", async () => {
    let calls = 0;
    const e = env({
      capture: async () => {
        calls++;
        throw new Error("disk full");
      },
    });
    const c = new CaptureController(e.deps);
    c.markOutput();
    await e.advance(3001);
    assert.equal(calls, 1);
    // A flood of output right after the failure must not trigger capture per chunk.
    for (let i = 0; i < 50; i++) {
      c.markOutput();
      await e.advance(10);
    }
    assert.equal(calls, 1, "no retry before the backoff elapses");
    const lastDelay = e.delays[e.delays.length - 1];
    assert.ok(lastDelay >= 4000, `delay ${lastDelay} must respect the backoff`);
    await e.advance(20000);
    assert.ok(calls >= 2, "retries once the backoff elapsed");
  });

  it("resets the backoff after a success", async () => {
    let fail = true;
    let calls = 0;
    const e = env({
      capture: async () => {
        calls++;
        if (fail) throw new Error("x");
      },
    });
    const c = new CaptureController(e.deps);
    c.markOutput();
    await e.advance(3001); // fails
    fail = false;
    await e.advance(6000); // retry succeeds
    const okCalls = calls;
    c.markOutput();
    await e.advance(3001);
    assert.equal(calls, okCalls + 1, "back to the normal debounce");
  });
});

describe("CaptureController in-flight handling", () => {
  it("does not drop final output that arrives during a capture", async () => {
    const first = deferred();
    let calls = 0;
    const e = env({
      capture: async () => {
        calls++;
        if (calls === 1) await first.promise;
      },
    });
    const c = new CaptureController(e.deps);
    c.markOutput();
    await e.advance(3001); // capture #1 starts and blocks
    assert.equal(calls, 1);
    c.markDirtyNow(); // process exit: last screen + notice
    const done = c.capture();
    first.resolve();
    await done;
    await flushMicrotasks();
    assert.equal(calls, 2, "the output that arrived mid-capture must be captured afterwards");
  });

  it("flush waits for the in-flight capture and then captures the newest state", async () => {
    const first = deferred();
    let calls = 0;
    const e = env({
      capture: async () => {
        calls++;
        if (calls === 1) await first.promise;
      },
    });
    const c = new CaptureController(e.deps);
    c.markOutput();
    await e.advance(3001);
    c.markOutput(); // newer output while #1 is in flight
    let flushed = false;
    const f = c.flush().then(() => (flushed = true));
    await flushMicrotasks();
    assert.equal(flushed, false, "flush must wait for the in-flight capture");
    first.resolve();
    await f;
    assert.equal(calls, 2);
  });

  it("dispose waits for the in-flight capture and blocks any later one", async () => {
    const first = deferred();
    let calls = 0;
    const e = env({
      capture: async () => {
        calls++;
        await first.promise;
      },
    });
    const c = new CaptureController(e.deps);
    c.markOutput();
    await e.advance(3001);
    let disposed = false;
    const d = c.dispose().then(() => (disposed = true));
    await flushMicrotasks();
    assert.equal(disposed, false, "closing a tab must not delete before the in-flight save lands");
    first.resolve();
    await d;
    c.markOutput();
    await e.advance(60000);
    await c.flush();
    assert.equal(calls, 1);
  });

  it("dispose cancels a pending timer", async () => {
    const e = env();
    const c = new CaptureController(e.deps);
    c.markOutput();
    await c.dispose();
    await e.advance(60000);
    assert.equal(e.log.captures, 0);
  });
});
