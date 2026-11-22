/**
 * Pure logic for terminal snapshots (capture timing, restore sequence, purge targets).
 * No DOM, no xterm, no Tauri: unit-tested with `npm run test:unit`.
 */
// Provisional timings: Task 4 sets them from measurements in the real app.
export const CAPTURE_DEBOUNCE_MS = 3000;
export const CAPTURE_MAX_WAIT_MS = 15000;

/** Terminal modes a TUI may leave on; reset so the restored screen cannot capture mouse or paste. */
export const MODE_RESET =
  "\x1b[0m\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?2004l\x1b[?1l\x1b[?25h";
export const LEAVE_ALT_SCREEN = "\x1b[?1049l";

export function captureDelayMs(args: {
  now: number;
  firstDirtyAt: number;
  lastOutputAt: number;
  debounceMs?: number;
  maxWaitMs?: number;
}): number {
  const { now, firstDirtyAt, lastOutputAt } = args;
  const debounceAt = lastOutputAt + (args.debounceMs ?? CAPTURE_DEBOUNCE_MS);
  const deadline = firstDirtyAt + (args.maxWaitMs ?? CAPTURE_MAX_WAIT_MS);
  return Math.max(0, Math.min(debounceAt, deadline) - now);
}

export function shouldCapture(args: {
  enabled: boolean;
  disposed: boolean;
  dirty: boolean;
}): boolean {
  return args.enabled && !args.disposed && args.dirty;
}

export function shouldRestoreSnapshot(args: {
  enabled: boolean;
  isRestored: boolean;
  status: "running" | "stopped";
}): boolean {
  return args.enabled && args.isRestored && args.status === "stopped";
}

const pad = (n: number) => String(n).padStart(2, "0");

export function formatSnapshotDate(createdAtSec: number, utc = false): string {
  const d = new Date(createdAtSec * 1000);
  const y = utc ? d.getUTCFullYear() : d.getFullYear();
  const mo = utc ? d.getUTCMonth() : d.getMonth();
  const day = utc ? d.getUTCDate() : d.getDate();
  const h = utc ? d.getUTCHours() : d.getHours();
  const mi = utc ? d.getUTCMinutes() : d.getMinutes();
  return `${y}-${pad(mo + 1)}-${pad(day)} ${pad(h)}:${pad(mi)}`;
}

export function separatorLine(createdAtSec: number, utc = false): string {
  return `[Snapshot from ${formatSnapshotDate(createdAtSec, utc)} — process not restored]`;
}

/** True when the serialized buffer ends inside a TUI's alternate screen. */
export function snapshotHasAltScreen(data: string): boolean {
  return data.includes("\x1b[?1049h");
}

/**
 * Bytes written into xterm to show a snapshot: the content, a clean line, a mode reset and the
 * visible separator. The alternate screen is deliberately NOT left here, or the TUI content would
 * vanish; `LEAVE_ALT_SCREEN` is written when a new shell starts.
 */
export function buildRestoreSequence(data: string, createdAtSec: number, utc = false): string {
  return `${data}\r\n${MODE_RESET}\r\n\x1b[36m${separatorLine(createdAtSec, utc)}\x1b[0m\r\n`;
}

/** What to write before a new shell starts on a tab that showed a snapshot. */
export function newShellPrelude(hadAltScreen: boolean): string {
  return `${hadAltScreen ? LEAVE_ALT_SCREEN : ""}${MODE_RESET}`;
}
