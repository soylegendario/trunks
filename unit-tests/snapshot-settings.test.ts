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
  type SettingsDeps,
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

function deps(over: Partial<SettingsDeps> = {}, calls: string[] = []): SettingsDeps {
  return {
    setEnabled: async (v) => void calls.push(`set:${v}`),
    clearAll: async () => void calls.push("clear"),
    disableAndClear: async () => void calls.push("disable-and-clear"),
    readEnabled: async () => {
      calls.push("read");
      return false;
    },
    ...over,
  };
}

describe("applyEnable", () => {
  it("turns the flag on and reports success", async () => {
    const calls: string[] = [];
    const r = await applyEnable(deps({}, calls));
    assert.deepEqual(calls, ["set:true"]);
    assert.deepEqual(r, { ok: true, enabled: true });
  });
  it("reports failure and stays off when the backend rejects", async () => {
    const r = await applyEnable(
      deps({
        setEnabled: async () => {
          throw new Error("db down");
        },
      })
    );
    assert.deepEqual(r, { ok: false, enabled: false, error: "Error: db down" });
  });
});

describe("applyDisable", () => {
  it("uses the single atomic backend step, never flag-then-clear as two calls", async () => {
    const calls: string[] = [];
    const r = await applyDisable(deps({}, calls));
    assert.deepEqual(calls, ["disable-and-clear"]);
    assert.deepEqual(r, { ok: true, enabled: false });
  });
  it("re-reads the real flag from the backend when the step fails", async () => {
    const r = await applyDisable(
      deps({
        disableAndClear: async () => {
          throw new Error("io");
        },
        readEnabled: async () => false, // flag was turned off, deletion failed
      })
    );
    assert.equal(r.ok, false);
    assert.equal(r.enabled, false);
    assert.match(r.error ?? "", /io/);
  });
  it("shows the flag as still on when the backend says so", async () => {
    const r = await applyDisable(
      deps({
        disableAndClear: async () => {
          throw new Error("db down");
        },
        readEnabled: async () => true,
      })
    );
    assert.equal(r.enabled, true);
  });
  it("assumes on (the safe display) when even the re-read fails", async () => {
    const r = await applyDisable(
      deps({
        disableAndClear: async () => {
          throw new Error("x");
        },
        readEnabled: async () => {
          throw new Error("y");
        },
      })
    );
    assert.equal(r.ok, false);
    assert.equal(r.enabled, true);
  });
});

describe("applyClearAll", () => {
  it("deletes snapshots without touching the setting", async () => {
    const calls: string[] = [];
    const r = await applyClearAll(deps({}, calls));
    assert.deepEqual(calls, ["clear"]);
    assert.equal(r.ok, true);
  });
});
