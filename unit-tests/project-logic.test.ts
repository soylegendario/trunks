// Run with `npm run test:unit` (Node's built-in runner; no extra dependencies).
// Lives in unit-tests/ on purpose: svelte-check type-checks src/, test/ and tests/, and this file
// needs `.ts` import extensions and node:* types, which the project does not configure.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  hydrateProjects,
  toPersistedProjects,
  resolveSelectedWorktree,
  restoreSelection,
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
