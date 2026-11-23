import { marked } from "marked";
import DOMPurify from "dompurify";

const MD_EXT = /\.(md|markdown|mdx)$/i;

export function isMarkdownPath(path: string): boolean {
  return MD_EXT.test(path);
}

let hooked = false;
function ensureHook() {
  if (hooked || !DOMPurify.isSupported) return;
  hooked = true;
  DOMPurify.addHook("afterSanitizeAttributes", (node) => {
    if (node.tagName === "A" && node.hasAttribute("href")) {
      node.setAttribute("target", "_blank");
      node.setAttribute("rel", "noopener noreferrer");
    }
  });
}

/** Renders Markdown to sanitized HTML. Requires a DOM (browser/webview). */
export function renderMarkdown(src: string): string {
  ensureHook();
  const raw = marked.parse(src, { async: false, gfm: true }) as string;
  return DOMPurify.isSupported ? DOMPurify.sanitize(raw) : "";
}
