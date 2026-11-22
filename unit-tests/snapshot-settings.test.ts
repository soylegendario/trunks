import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  CLEAR_NOTICE,
  DISABLE_NOTICE,
  ENABLE_NOTICE,
  applyClearAll,
  applyDisable,
  applyEnable,
  stepForToggle,
} from "../src/lib/snapshot-settings-logic.ts";

describe("stepForToggle", () => {
  it("asks for confirmation in both directions and does nothing when unchanged", () => {
    assert.equal(stepForToggle(false, true), "confirm-enable");
    assert.equal(stepForToggle(true, false), "confirm-disable");
    assert.equal(stepForToggle(true, true), "idle");
    assert.equal(stepForToggle(false, false), "idle");
  });
});

describe("notices", () => {
  it("warn about secrets and unencrypted storage before enabling", () => {
    assert.match(ENABLE_NOTICE, /secrets/i);
    assert.match(ENABLE_NOTICE, /unencrypted/i);
  });
  it("never promise redaction or process restoration", () => {
    for (const text of [ENABLE_NOTICE, DISABLE_NOTICE, CLEAR_NOTICE]) {
      assert.doesNotMatch(text, /will be redacted|are redacted|automatically redact/i);
      assert.doesNotMatch(text, /process (is|will be) restored/i);
    }
    assert.match(ENABLE_NOTICE, /not redacted/i);
    assert.match(ENABLE_NOTICE, /process is never restored/i);
  });
  it("states that disabling deletes the stored history", () => {
    assert.match(DISABLE_NOTICE, /delet/i);
  });
});

describe("applyEnable", () => {
  it("turns the flag on and reports success", async () => {
    const calls: string[] = [];
    const r = await applyEnable({
      setEnabled: async (v) => void calls.push(`set:${v}`),
      clearAll: async () => void calls.push("clear"),
    });
    assert.deepEqual(calls, ["set:true"]);
    assert.deepEqual(r, { ok: true, enabled: true });
  });
  it("reports failure and stays off when the backend rejects", async () => {
    const r = await applyEnable({
      setEnabled: async () => {
        throw new Error("db down");
      },
      clearAll: async () => {},
    });
    assert.deepEqual(r, { ok: false, enabled: false, error: "Error: db down" });
  });
});

describe("applyDisable", () => {
  it("disables first so no new snapshot can be written, then deletes everything", async () => {
    const calls: string[] = [];
    const r = await applyDisable({
      setEnabled: async (v) => void calls.push(`set:${v}`),
      clearAll: async () => void calls.push("clear"),
    });
    assert.deepEqual(calls, ["set:false", "clear"]);
    assert.deepEqual(r, { ok: true, enabled: false });
  });
  it("stays disabled and surfaces the error when deletion fails", async () => {
    const r = await applyDisable({
      setEnabled: async () => {},
      clearAll: async () => {
        throw new Error("io");
      },
    });
    assert.equal(r.ok, false);
    assert.equal(r.enabled, false);
    assert.match(r.error ?? "", /io/);
  });
  it("does not delete anything when the flag could not be turned off", async () => {
    const calls: string[] = [];
    const r = await applyDisable({
      setEnabled: async () => {
        throw new Error("db down");
      },
      clearAll: async () => void calls.push("clear"),
    });
    assert.deepEqual(calls, []);
    assert.equal(r.ok, false);
    assert.equal(r.enabled, true, "flag is still on, so the UI must keep showing it on");
  });
});

describe("applyClearAll", () => {
  it("deletes snapshots without touching the setting", async () => {
    const calls: string[] = [];
    const r = await applyClearAll({
      setEnabled: async (v) => void calls.push(`set:${v}`),
      clearAll: async () => void calls.push("clear"),
    });
    assert.deepEqual(calls, ["clear"]);
    assert.equal(r.ok, true);
  });
});
