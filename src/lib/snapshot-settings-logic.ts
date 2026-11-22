/**
 * Pure logic for the "Save terminal history" setting: confirmation steps, notice copy and the
 * ordering of enable / disable / clear operations. The UI only wires these to buttons.
 */

export type SettingsStep = "idle" | "confirm-enable" | "confirm-disable" | "confirm-clear";

export function stepForToggle(current: boolean, wanted: boolean): SettingsStep {
  if (wanted === current) return "idle";
  return wanted ? "confirm-enable" : "confirm-disable";
}

// The copy must never promise redaction or process restoration (design spec, non-goals).
export const ENABLE_NOTICE =
  "Terminal output can contain secrets: tokens, passwords echoed by tools, .env contents or " +
  "private code. With this setting on, the text of each terminal is saved unencrypted in this " +
  "computer's app data folder and is not redacted. After a restart you will see the saved text " +
  "marked as a snapshot; the process is never restored.";

export const DISABLE_NOTICE =
  "Turning this off deletes all saved terminal history from this computer. Open terminals are not affected.";

export const CLEAR_NOTICE =
  "This deletes all saved terminal history from this computer. The setting stays as it is.";

export interface SettingsDeps {
  setEnabled: (value: boolean) => Promise<void>;
  clearAll: () => Promise<void>;
}

export interface SettingsResult {
  ok: boolean;
  /** The flag as the UI must show it after the operation (never assumes success). */
  enabled: boolean;
  error?: string;
}

export async function applyEnable(deps: SettingsDeps): Promise<SettingsResult> {
  try {
    await deps.setEnabled(true);
    return { ok: true, enabled: true };
  } catch (err) {
    return { ok: false, enabled: false, error: String(err) };
  }
}

/** Turns the flag off first (the backend then refuses new saves), then deletes what is stored. */
export async function applyDisable(deps: SettingsDeps): Promise<SettingsResult> {
  try {
    await deps.setEnabled(false);
  } catch (err) {
    return { ok: false, enabled: true, error: String(err) };
  }
  try {
    await deps.clearAll();
  } catch (err) {
    return { ok: false, enabled: false, error: String(err) };
  }
  return { ok: true, enabled: false };
}

export async function applyClearAll(deps: SettingsDeps): Promise<{ ok: boolean; error?: string }> {
  try {
    await deps.clearAll();
    return { ok: true };
  } catch (err) {
    return { ok: false, error: String(err) };
  }
}
