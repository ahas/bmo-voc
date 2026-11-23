// The refusal codes the pack has its own words for: every code of `docs/api.md`, 「Codes」, and the common ones a
// screen can meet. Any other code says the fallback of the act that met it.
export const WORDED_CODES = [
  "bad-request",
  "unauthorized",
  "forbidden",
  "no-route",
  "not-found",
  "conflict",
  "gone",
  "failed",
  "upstream-failed",
  "not-ready",
  "maker-side",
  "sender-side",
  "bad-report",
  "too-many-files",
  "file-too-large"
] as const;
