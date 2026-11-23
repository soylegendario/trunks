import { test } from "node:test";
import assert from "node:assert/strict";
import { isMarkdownPath } from "../src/lib/markdown-logic.ts";

test("isMarkdownPath detects markdown extensions", () => {
  assert.equal(isMarkdownPath("README.md"), true);
  assert.equal(isMarkdownPath("docs/a.MD"), true);
  assert.equal(isMarkdownPath("x.markdown"), true);
  assert.equal(isMarkdownPath("x.mdx"), true);
});

test("isMarkdownPath rejects other files", () => {
  assert.equal(isMarkdownPath("a.txt"), false);
  assert.equal(isMarkdownPath("Makefile"), false);
  assert.equal(isMarkdownPath("md"), false);
  assert.equal(isMarkdownPath("a.md.bak"), false);
});
