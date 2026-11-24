// Run with `npm run test:unit` (Node's built-in runner; no extra dependencies).
// Lives in unit-tests/ on purpose: svelte-check type-checks src/, test/ and tests/, and this file
// needs `.ts` import extensions and node:* types, which the project does not configure.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  tabIdsForRemovedPaths,
  hydrateProjects,
  toPersistedProjects,
  resolveSelectedWorktree,
  restoreSelection,
  findTabForWorktree,
  tabsForWorktree,
  terminalDotState,
  aggregateTerminalState,
  pruneCollapsedWorktrees,
  type ProjectState,
} from "../src/lib/project-logic.ts";
import type { GitWorktreeInfo } from "../src/lib/git.ts";
import type { ProjectItem } from "../src/lib/storage.ts";

const wt = (path: string, branch: string | null = "main"): GitWorktreeInfo => ({
  path,
  head: "abc1234",
  branch,
  bare: false,
  locked: null,
  prunable: null,
});

const project = (over: Partial<ProjectState> = {}): ProjectState => ({
  id: "p1",
  name: "repo",
  path: "/r",
  is_stale: false,
  stale_reason: null,
  worktrees: [wt("/r"), wt("/r-feat", "feat")],
  isLoading: false,
  error: null,
  ...over,
});

describe("project persistence (Git stays the single source of truth)", () => {
  it("persists only the project reference, never Git-derived state", () => {
    const persisted = toPersistedProjects([
      project({ is_stale: true, stale_reason: "git failed", error: "git failed" }),
    ]);

    assert.equal(persisted.length, 1);
    assert.equal(persisted[0].id, "p1");
    assert.equal(persisted[0].name, "repo");
    assert.equal(persisted[0].path, "/r");
    assert.deepEqual(persisted[0].worktrees, []);
    assert.equal(persisted[0].is_stale, false);
    assert.equal(persisted[0].stale_reason, null);
  });

  it("ignores cached worktree rows when hydrating and starts in a loading state", () => {
    const legacy: ProjectItem[] = [
      {
        id: "p1",
        name: "repo",
        path: "/r",
        worktrees: [
          { id: "p1-wt-0", project_id: "p1", name: "main", branch: "main", path: "/r" },
        ],
      },
    ];

    const [hydrated] = hydrateProjects(legacy);

    assert.deepEqual(hydrated.worktrees, [], "cached rows must not be painted as Git state");
    assert.equal(hydrated.isLoading, true);
    assert.equal(hydrated.error, null);
  });

  it("keeps the filesystem-derived stale flag reported for a project", () => {
    const [hydrated] = hydrateProjects([
      {
        id: "p1",
        name: "repo",
        path: "/gone",
        is_stale: true,
        stale_reason: "Project directory not found on filesystem",
        worktrees: [],
      },
    ]);

    assert.equal(hydrated.is_stale, true);
    assert.equal(hydrated.stale_reason, "Project directory not found on filesystem");
  });
});

describe("worktree selection after Git answers", () => {
  const listed = [wt("/r"), wt("/r-feat", "feat")];

  it("keeps the selected worktree while Git still lists it", () => {
    assert.equal(resolveSelectedWorktree("/r-feat", listed), "/r-feat");
  });

  it("falls back to the first worktree when the selected one disappeared", () => {
    assert.equal(resolveSelectedWorktree("/removed", listed), "/r");
  });

  it("selects the first worktree when nothing was selected", () => {
    assert.equal(resolveSelectedWorktree(null, listed), "/r");
  });

  it("selects nothing when the project has no worktrees", () => {
    assert.equal(resolveSelectedWorktree("/r", []), null);
  });
});

describe("selection restore on startup", () => {
  const registered = [{ id: "p1" }, { id: "p2" }];

  it("restores a selection that points at a registered project", () => {
    assert.deepEqual(restoreSelection("p2", "/r2-feat", registered), {
      projectId: "p2",
      worktreePath: "/r2-feat",
    });
  });

  it("keeps the project but no worktree when none was saved", () => {
    assert.deepEqual(restoreSelection("p1", null, registered), {
      projectId: "p1",
      worktreePath: null,
    });
  });

  it("drops a selection whose project is no longer registered", () => {
    assert.deepEqual(restoreSelection("gone", "/x", registered), {
      projectId: null,
      worktreePath: null,
    });
  });

  it("drops a worktree that has no project", () => {
    assert.deepEqual(restoreSelection(null, "/x", registered), {
      projectId: null,
      worktreePath: null,
    });
  });
});

describe("findTabForWorktree", () => {
  const tabs = [
    { id: "a", cwd: "/r/main", status: "stopped" },
    { id: "b", cwd: "/r/wt", status: "stopped" },
    { id: "c", cwd: "/r/wt", status: "running" },
  ];
  it("prefers a running tab for the worktree", () => {
    assert.equal(findTabForWorktree(tabs, "/r/wt")?.id, "c");
  });
  it("falls back to a stopped tab and ignores trailing slashes", () => {
    assert.equal(findTabForWorktree(tabs, "/r/main/")?.id, "a");
  });
  it("returns undefined when no tab is bound", () => {
    assert.equal(findTabForWorktree(tabs, "/r/other"), undefined);
  });
});

describe("tabsForWorktree", () => {
  const tabs = [
    { id: "a", cwd: "/r/main" },
    { id: "b", cwd: "/r/wt" },
    { id: "c", cwd: "/r/wt/" },
  ];
  it("returns every tab of the worktree, in order", () => {
    assert.deepEqual(tabsForWorktree(tabs, "/r/wt").map((t) => t.id), ["b", "c"]);
  });
  it("returns an empty list when nothing is bound", () => {
    assert.deepEqual(tabsForWorktree(tabs, "/r/none"), []);
  });
});

describe("tabIdsForRemovedPaths", () => {
  const tabs = [
    { id: "a", cwd: "/repo/wt1" },
    { id: "b", cwd: "/repo/wt2/" },
    { id: "c", cwd: "/other" },
  ];
  it("selects tabs bound to the removed worktrees, ignoring trailing slashes", () => {
    assert.deepEqual(tabIdsForRemovedPaths(tabs, ["/repo/wt2"]), ["b"]);
    assert.deepEqual(tabIdsForRemovedPaths(tabs, ["/repo/wt1", "/repo/wt2"]).sort(), ["a", "b"]);
  });
  it("returns nothing when no tab matches", () => {
    assert.deepEqual(tabIdsForRemovedPaths(tabs, ["/nope"]), []);
  });
});

describe("terminalDotState", () => {
  it("maps a running tab to running", () => {
    assert.equal(terminalDotState({ status: "running" }), "running");
  });
  it("maps a stopped restored tab to restored", () => {
    assert.equal(terminalDotState({ status: "stopped", isRestored: true }), "restored");
  });
  it("maps a plain stopped tab to stopped", () => {
    assert.equal(terminalDotState({ status: "stopped" }), "stopped");
  });
  it("keeps running when a running tab is still flagged as restored", () => {
    assert.equal(terminalDotState({ status: "running", isRestored: true }), "running");
  });
});

describe("aggregateTerminalState", () => {
  it("has no state and zero count without tabs", () => {
    assert.deepEqual(aggregateTerminalState([]), { state: null, count: 0 });
  });
  it("is running when any tab runs, even with restored ones", () => {
    const r = aggregateTerminalState([
      { status: "stopped", isRestored: true },
      { status: "running" },
    ]);
    assert.deepEqual(r, { state: "running", count: 2 });
  });
  it("is restored when none runs but one is restored", () => {
    const r = aggregateTerminalState([{ status: "stopped" }, { status: "stopped", isRestored: true }]);
    assert.deepEqual(r, { state: "restored", count: 2 });
  });
  it("has no state when only plain stopped tabs exist", () => {
    assert.deepEqual(aggregateTerminalState([{ status: "stopped" }]), { state: null, count: 1 });
  });
});

describe("pruneCollapsedWorktrees", () => {
  it("drops paths that are no longer live, keeping order", () => {
    assert.deepEqual(pruneCollapsedWorktrees(["/a", "/gone", "/b"], ["/b", "/a"]), ["/a", "/b"]);
  });
  it("drops everything when no path is live", () => {
    assert.deepEqual(pruneCollapsedWorktrees(["/a", "/b"], []), []);
  });
});
