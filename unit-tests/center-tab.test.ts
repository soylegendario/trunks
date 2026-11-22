import { test } from "node:test";
import assert from "node:assert/strict";
import { commitTabId, commitTabTitle, fileTabId } from "../src/lib/center-tab-logic.ts";

const oid = "a".repeat(40);

test("commit tab id is stable so reopening focuses the same tab", () => {
  assert.equal(commitTabId("/wt", oid), commitTabId("/wt", oid));
});

test("commit tab ids differ per worktree and per commit", () => {
  assert.notEqual(commitTabId("/wt1", oid), commitTabId("/wt2", oid));
  assert.notEqual(commitTabId("/wt", oid), commitTabId("/wt", "b".repeat(40)));
});

test("commit tab ids never collide with file or diff tab ids", () => {
  const ids = [
    fileTabId("/wt", "file", false, oid),
    fileTabId("/wt", "diff", false, oid),
    fileTabId("/wt", "diff", true, oid),
    fileTabId("/wt", "diff", false, `commit|${oid}`),
  ];
  for (const id of ids) assert.notEqual(id, commitTabId("/wt", oid));
});

test("commit tab title is the short hash and subject", () => {
  assert.equal(commitTabTitle("abc1234", "fix thing"), "abc1234 fix thing");
  assert.equal(commitTabTitle("abc1234", ""), "abc1234");
});
