import DOMPurify from "dompurify";
import { marked } from "marked";

// A report's or a reply's body, written in markdown by a person, as markup a screen lays into a `markdown` container
// (bmo-ui's `docs/boundary.md`, the rendered text): GitHub's flavour with a line break kept where it was typed, every
// tag and attribute a script could ride on taken out, and every link opening apart from the console.

const purifier = typeof window === "undefined" ? null : DOMPurify(window);

purifier?.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "A") {
    node.setAttribute("target", "_blank");
    node.setAttribute("rel", "noopener noreferrer");
  }
});

/** The markup of a markdown body, safe to lay into the page. */
export function markdownHtml(body: string): string {
  const html = marked.parse(body, { gfm: true, breaks: true, async: false });

  return purifier === null ? "" : purifier.sanitize(html, { FORBID_TAGS: ["style", "form", "input", "button", "textarea", "select"] });
}
