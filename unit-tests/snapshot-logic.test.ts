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
