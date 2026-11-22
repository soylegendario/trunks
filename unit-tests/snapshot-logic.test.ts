import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  buildRestoreSequence,
  captureDelayMs,
  formatSnapshotDate,
  newShellPrelude,
  separatorLine,
  shouldCapture,
  shouldRestoreSnapshot,
  snapshotHasAltScreen,
  MODE_RESET,
  restoreSnapshotGuarded,
} from "../src/lib/snapshot-logic.ts";

describe("captureDelayMs", () => {
  it("waits the debounce after the last output", () => {
    assert.equal(
      captureDelayMs({ now: 1000, firstDirtyAt: 1000, lastOutputAt: 1000, debounceMs: 3000, maxWaitMs: 15000 }),
      3000
    );
  });
  it("never waits past the max wait under continuous output", () => {
    // Output keeps arriving, so the debounce keeps sliding; the deadline from first dirty wins.
    assert.equal(
      captureDelayMs({ now: 14000, firstDirtyAt: 0, lastOutputAt: 14000, debounceMs: 3000, maxWaitMs: 15000 }),
      1000
    );
  });
  it("is zero once the deadline has passed", () => {
    assert.equal(
      captureDelayMs({ now: 20000, firstDirtyAt: 0, lastOutputAt: 20000, debounceMs: 3000, maxWaitMs: 15000 }),
      0
    );
  });
});

describe("shouldCapture", () => {
  it("requires opt-in, a live session and new output", () => {
    assert.equal(shouldCapture({ enabled: true, disposed: false, dirty: true }), true);
    assert.equal(shouldCapture({ enabled: false, disposed: false, dirty: true }), false);
    assert.equal(shouldCapture({ enabled: true, disposed: true, dirty: true }), false);
    assert.equal(shouldCapture({ enabled: true, disposed: false, dirty: false }), false);
  });
});

describe("shouldRestoreSnapshot", () => {
  it("only for stopped restored tabs with the setting on", () => {
    assert.equal(shouldRestoreSnapshot({ enabled: true, isRestored: true, status: "stopped" }), true);
    assert.equal(shouldRestoreSnapshot({ enabled: false, isRestored: true, status: "stopped" }), false);
    assert.equal(shouldRestoreSnapshot({ enabled: true, isRestored: false, status: "stopped" }), false);
    assert.equal(shouldRestoreSnapshot({ enabled: true, isRestored: true, status: "running" }), false);
  });
});

describe("separator", () => {
  it("formats the date and states that the process is not restored", () => {
    assert.equal(formatSnapshotDate(1795356180, true), "2026-11-22 14:03");
    assert.equal(
      separatorLine(1795356180, true),
      "[Snapshot from 2026-11-22 14:03 — process not restored]"
    );
  });
});

describe("buildRestoreSequence", () => {
  const seq = buildRestoreSequence("hello", 1795356180, true);
  it("keeps the content first, then resets modes, then the visible separator", () => {
    assert.ok(seq.startsWith("hello\r\n"));
    assert.ok(seq.indexOf(MODE_RESET) > seq.indexOf("hello"));
    assert.ok(seq.indexOf("process not restored") > seq.indexOf(MODE_RESET));
  });
  it("turns off mouse tracking, bracketed paste and shows the cursor", () => {
    for (const m of ["\x1b[?1000l", "\x1b[?1003l", "\x1b[?1006l", "\x1b[?2004l", "\x1b[?25h"]) {
      assert.ok(seq.includes(m), `missing ${JSON.stringify(m)}`);
    }
  });
  it("does not leave the alternate screen, or the TUI content would disappear", () => {
    assert.ok(!seq.includes("\x1b[?1049l"));
  });
  it("never claims the process was restored", () => {
    assert.ok(!/process restored|session restored/i.test(seq));
  });
});

describe("alternate screen handling", () => {
  it("detects a snapshot taken inside a TUI", () => {
    assert.equal(snapshotHasAltScreen("a\x1b[?1049h\x1b[Hscreen"), true);
    assert.equal(snapshotHasAltScreen("plain"), false);
  });
  it("leaves the alternate screen before a new shell only when needed", () => {
    assert.ok(newShellPrelude(true).startsWith("\x1b[?1049l"));
    assert.ok(!newShellPrelude(false).includes("\x1b[?1049l"));
    assert.ok(newShellPrelude(false).includes(MODE_RESET));
  });
});

describe("MODE_RESET covers every mode addon-serialize can emit", () => {
  it("soft-resets the terminal and explicitly turns off mouse, focus and paste reporting", () => {
    for (const m of [
      "\x1b[!p", // DECSTR: insert, origin, autowrap, cursor keys, keypad...
      "\x1b[?9l",
      "\x1b[?1000l",
      "\x1b[?1002l",
      "\x1b[?1003l",
      "\x1b[?1004l",
      "\x1b[?1005l",
      "\x1b[?1006l",
      "\x1b[?1015l",
      "\x1b[?2004l",
      "\x1b[?66l",
      "\x1b[?45l",
      "\x1b[?6l",
      "\x1b[?7h",
      "\x1b[4l",
      "\x1b[?25h",
    ]) {
      assert.ok(MODE_RESET.includes(m), `missing ${JSON.stringify(m)}`);
    }
  });
});

describe("restoreSnapshotGuarded", () => {
  const snap = { data: "old", created_at: 1795356180 };
  it("writes the restore sequence when the tab is still a stopped restored tab", async () => {
    const written: string[] = [];
    const r = await restoreSnapshotGuarded({
      load: async () => snap,
      stillRestorable: () => true,
      write: async (seq) => void written.push(seq),
    });
    assert.equal(written.length, 1);
    assert.ok(written[0].startsWith("old\r\n"));
    assert.equal(r.restored, true);
  });
  it("does not write into a live shell when the user started it during the load", async () => {
    let restorable = true;
    const written: string[] = [];
    const r = await restoreSnapshotGuarded({
      load: async () => {
        restorable = false; // Start was clicked while the snapshot was loading
        return snap;
      },
      stillRestorable: () => restorable,
      write: async (seq) => void written.push(seq),
    });
    assert.deepEqual(written, []);
    assert.equal(r.restored, false);
  });
  it("reports the alternate screen of a TUI snapshot", async () => {
    const r = await restoreSnapshotGuarded({
      load: async () => ({ data: "a\x1b[?1049hb", created_at: 1 }),
      stillRestorable: () => true,
      write: async () => {},
    });
    assert.equal(r.hasAltScreen, true);
  });
  it("swallows load failures and restores nothing", async () => {
    const r = await restoreSnapshotGuarded({
      load: async () => {
        throw new Error("io");
      },
      stillRestorable: () => true,
      write: async () => {
        assert.fail("must not write");
      },
    });
    assert.equal(r.restored, false);
  });
  it("restores nothing when there is no snapshot", async () => {
    const r = await restoreSnapshotGuarded({
      load: async () => null,
      stillRestorable: () => true,
      write: async () => assert.fail("must not write"),
    });
    assert.equal(r.restored, false);
  });
});
