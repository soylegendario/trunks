import { open } from "@tauri-apps/plugin-dialog";

/**
 * Opens the native folder picker. Returns the chosen directory, or `null` when the user cancels
 * or the dialog fails (never throws: callers just leave their fields untouched).
 */
export async function pickDirectory(defaultPath?: string): Promise<string | null> {
  try {
    const picked = await open({ directory: true, multiple: false, defaultPath });
    return typeof picked === "string" ? picked : null;
  } catch (err) {
    console.warn("Folder picker failed:", err);
    return null;
  }
}
