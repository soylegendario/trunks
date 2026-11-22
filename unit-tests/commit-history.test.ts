import { test } from "node:test";
import assert from "node:assert/strict";
import {
  applyPage,
  emptyHistory,
  historyKey,
  isHistoryStale,
  retarget,
} from "../src/lib/commit-history-logic.ts";
import type { GitCommitHistoryPage, GitCommitInfo } from "../src/lib/git.ts";

const c = (hash: string): GitCommitInfo => ({
  hash,
  short_hash: hash.slice(0, 7),
  subject: hash,
  author: "A",
  commit_time: 1,
  group: "flat",
});

const page = (hashes: string[], over: Partial<GitCommitHistoryPage> = {}): GitCommitHistoryPage => ({
  branch: "feat",
  head_state: "branch",
  head_oid: "h1",
  base: "main",
  base_source: "auto",
  partial_history: false,
  reset: false,
  commits: hashes.map(c),
  next_cursor: { group: "flat", offset: hashes.length, head_oid: "h1", base_oid: null },
  ...over,
});

const hashes = (s: { commits: GitCommitInfo[] }) => s.commits.map((x) => x.hash);

test("appends the next page", () => {
  const key = historyKey("/wt", null);
  let s = applyPage(emptyHistory(key), key, page(["a", "b"]), true);
  s = applyPage(s, key, page(["c"], { next_cursor: null }), false);
  assert.deepEqual(hashes(s), ["a", "b", "c"]);
  assert.equal(s.cursor, null);
});

test("rejects stale responses after a worktree switch", () => {
  const k1 = historyKey("/wt1", null);
  const k2 = historyKey("/wt2", null);
  const s = retarget(applyPage(emptyHistory(k1), k1, page(["a"]), true), k2);
  assert.equal(s.key, k2);
  assert.deepEqual(hashes(s), []);
  assert.equal(applyPage(s, k1, page(["late"]), true), s);
});

test("changing the base resets pages and rejects its stale response", () => {
  const auto = historyKey("/wt", null);
  const dev = historyKey("/wt", "develop");
  let s = applyPage(emptyHistory(auto), auto, page(["a", "b"]), true);
  s = retarget(s, dev);
  assert.deepEqual(hashes(s), []);
  assert.equal(s.meta, null);
  assert.equal(applyPage(s, auto, page(["x"]), false), s);
});

test("retarget keeps state when the key is unchanged", () => {
  const key = historyKey("/wt", null);
  const s = applyPage(emptyHistory(key), key, page(["a"]), true);
  assert.equal(retarget(s, key), s);
});

test("a reset page replaces accumulated pages", () => {
  const key = historyKey("/wt", null);
  let s = applyPage(emptyHistory(key), key, page(["a", "b"]), true);
  s = applyPage(s, key, page(["n", "a"], { reset: true, head_oid: "h2" }), false);
  assert.deepEqual(hashes(s), ["n", "a"]);
  assert.equal(s.meta?.head_oid, "h2");
});

test("detects HEAD changes from status polling", () => {
  const key = historyKey("/wt", null);
  const s = applyPage(emptyHistory(key), key, page(["a"]), true);
  assert.equal(isHistoryStale(s, "h1"), false);
  assert.equal(isHistoryStale(s, "h2"), true);
  assert.equal(isHistoryStale(s, undefined), false);
  assert.equal(isHistoryStale(emptyHistory(key), "h2"), false);
});
