/**
 * Pure logic for terminal snapshots (capture timing, restore sequence, purge targets).
 * No DOM, no xterm, no Tauri: unit-tested with `npm run test:unit`.
 */
// Provisional timings: Task 4 sets them from measurements in the real app.
export const CAPTURE_DEBOUNCE_MS = 3000;
export const CAPTURE_MAX_WAIT_MS = 15000;

/**
 * Resets every terminal mode addon-serialize can restore (it emits application cursor keys,
 * keypad, insert, origin, autowrap off, reverse-wrap, focus reporting, bracketed paste and mouse
 * tracking). DECSTR (`CSI ! p`) soft-resets the core modes; the rest are turned off explicitly
 * because xterm.js keeps mouse/focus/paste modes outside DECSTR.
 */
export const MODE_RESET =
  "\x1b[0m\x1b[!p" +
  "\x1b[?9l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1004l\x1b[?1005l\x1b[?1006l\x1b[?1015l" +
  "\x1b[?2004l\x1b[?66l\x1b[?45l\x1b[?6l\x1b[?7h\x1b[4l\x1b[?1l\x1b[?25h";
export const LEAVE_ALT_SCREEN = "\x1b[?1049l";

export function captureDelayMs(args: {
  now: number;
  firstDirtyAt: number;
  lastOutputAt: number;
  debounceMs?: number;
  maxWaitMs?: number;
  /** Earliest time a capture may run (failure backoff). */
  notBefore?: number;
}): number {
  const { now, firstDirtyAt, lastOutputAt } = args;
  const debounceAt = lastOutputAt + (args.debounceMs ?? CAPTURE_DEBOUNCE_MS);
  const deadline = firstDirtyAt + (args.maxWaitMs ?? CAPTURE_MAX_WAIT_MS);
  const at = Math.max(Math.min(debounceAt, deadline), args.notBefore ?? 0);
  return Math.max(0, at - now);
}

/** Delay before retrying after `consecutiveFailures` failed captures: 5 s doubling up to 5 min. */
export function failureBackoffMs(consecutiveFailures: number): number {
  const n = Math.max(1, consecutiveFailures);
  return Math.min(5000 * 2 ** Math.min(n - 1, 20), 300000);
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

export interface CaptureDeps {
  isEnabled: () => boolean;
  /** Serializes the terminal and stores it; rejects on failure. */
  capture: () => Promise<void>;
  now: () => number;
  schedule: (fn: () => void, ms: number) => unknown;
  cancel: (handle: unknown) => void;
}

/**
 * Decides when a terminal's snapshot is captured: debounced while output changes, bounded by a max
 * wait, backed off after failures, and never losing output that arrives during a capture.
 * Pure of DOM/xterm/Tauri: the session supplies `capture` and the timers.
 */
export class CaptureController {
  private dirty = false;
  private firstDirtyAt = 0;
  private lastOutputAt = 0;
  private timer: unknown = null;
  private inflight: Promise<void> | null = null;
  private recapture = false;
  private failures = 0;
  private notBefore = 0;
  private disposed = false;

  private readonly deps: CaptureDeps;

  constructor(deps: CaptureDeps) {
    this.deps = deps;
  }

  /** New live output: marks the buffer dirty and (re)schedules the debounced capture. */
  markOutput(): void {
    if (this.disposed || !this.deps.isEnabled()) return;
    const now = this.deps.now();
    if (!this.dirty) {
      this.dirty = true;
      this.firstDirtyAt = now;
    }
    this.lastOutputAt = now;
    this.schedule(
      captureDelayMs({
        now,
        firstDirtyAt: this.firstDirtyAt,
        lastOutputAt: this.lastOutputAt,
        notBefore: this.notBefore,
      })
    );
  }

  /** Marks the buffer dirty without scheduling (the caller captures right away, e.g. on exit). */
  markDirtyNow(): void {
    if (this.disposed) return;
    if (!this.dirty) {
      this.dirty = true;
      this.firstDirtyAt = this.deps.now();
    }
  }

  /** Captures now. If a capture is in flight, requests one more right after it. */
  capture(): Promise<void> {
    this.clearTimer();
    if (this.inflight) {
      this.recapture = true;
      return this.inflight;
    }
    if (!shouldCapture({ enabled: this.deps.isEnabled(), disposed: this.disposed, dirty: this.dirty })) {
      return Promise.resolve();
    }
    this.dirty = false;
    this.firstDirtyAt = 0;
    const run = (async () => {
      try {
        await this.deps.capture();
        this.failures = 0;
        this.notBefore = 0;
      } catch {
        // Keep the data pending, but back off: retrying per output chunk would stall the terminal.
        const now = this.deps.now();
        this.dirty = true;
        this.firstDirtyAt = now;
        this.failures++;
        this.notBefore = now + failureBackoffMs(this.failures);
        this.schedule(this.notBefore - now);
      }
    })();
    this.inflight = run.then(async () => {
      this.inflight = null;
      if (this.recapture) {
        this.recapture = false;
        if (this.deps.now() >= this.notBefore) await this.capture();
      }
    });
    return this.inflight;
  }

  /** Waits for any in-flight capture, then captures the newest state (window close). */
  async flush(): Promise<void> {
    if (this.inflight) {
      this.recapture = true;
      await this.inflight;
      return;
    }
    await this.capture();
  }

  /** Stops capturing and waits for an in-flight save, so a later delete cannot be overtaken. */
  async dispose(): Promise<void> {
    this.disposed = true;
    this.clearTimer();
    this.recapture = false;
    if (this.inflight) await this.inflight.catch(() => {});
  }

  private schedule(ms: number): void {
    this.clearTimer();
    this.timer = this.deps.schedule(() => {
      this.timer = null;
      void this.capture();
    }, ms);
  }

  private clearTimer(): void {
    if (this.timer !== null) {
      this.deps.cancel(this.timer);
      this.timer = null;
    }
  }
}

/**
 * Loads and writes the snapshot of a restored tab. The tab state is re-checked after the load
 * resolves: if the user started a shell meanwhile, nothing is written into the live terminal.
 * Never throws.
 */
export async function restoreSnapshotGuarded(deps: {
  load: () => Promise<{ data: string; created_at: number } | null>;
  stillRestorable: () => boolean;
  write: (sequence: string) => Promise<void>;
}): Promise<{ restored: boolean; hasAltScreen: boolean }> {
  try {
    const snap = await deps.load();
    if (!snap || !deps.stillRestorable()) return { restored: false, hasAltScreen: false };
    await deps.write(buildRestoreSequence(snap.data, snap.created_at));
    return { restored: true, hasAltScreen: snapshotHasAltScreen(snap.data) };
  } catch {
    return { restored: false, hasAltScreen: false };
  }
}
