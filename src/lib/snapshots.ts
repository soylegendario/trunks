import { invoke } from "@tauri-apps/api/core";

/** Mirrors the Rust `Snapshot` struct (src-tauri/src/snapshots.rs). */
export interface TerminalSnapshot {
  data: string;
  created_at: number;
  cols: number;
  rows: number;
}

// The backend is the source of truth for the opt-in flag and enforces it on save. This local copy
// only avoids pointless serialization work while the setting is off.
let enabledLocal = false;

export function isSnapshotsEnabledLocal(): boolean {
  return enabledLocal;
}

export function setSnapshotsEnabledLocal(value: boolean): void {
  enabledLocal = value;
}

export async function getSnapshotsEnabled(): Promise<boolean> {
  const value = await invoke<boolean>("db_get_snapshots_enabled");
  enabledLocal = value;
  return value;
}

export async function setSnapshotsEnabled(value: boolean): Promise<void> {
  await invoke("db_set_snapshots_enabled", { enabled: value });
  enabledLocal = value;
}

export async function snapshotSave(
  tabId: string,
  cols: number,
  rows: number,
  data: string
): Promise<void> {
  await invoke("snapshot_save", { tabId, cols, rows, data });
}

export async function snapshotLoad(tabId: string): Promise<TerminalSnapshot | null> {
  return await invoke<TerminalSnapshot | null>("snapshot_load", { tabId });
}

export async function snapshotDelete(tabId: string): Promise<void> {
  await invoke("snapshot_delete", { tabId });
}

export async function snapshotClearAll(): Promise<void> {
  await invoke("snapshot_clear_all");
}

export async function snapshotReconcile(liveTabIds: string[]): Promise<number> {
  return await invoke<number>("snapshot_reconcile", { liveTabIds });
}

/** Deletes snapshots without ever failing the caller (UI flows must not break on cleanup). */
export async function snapshotDeleteMany(tabIds: string[]): Promise<void> {
  await Promise.all(
    tabIds.map((id) =>
      snapshotDelete(id).catch((err) => console.warn("Failed to delete snapshot:", String(err)))
    )
  );
}
