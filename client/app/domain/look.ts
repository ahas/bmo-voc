import type { Kind, State } from "../../shared/types";

// How each kind and state is drawn: its mark and its colour, one look on every screen.

export type Tone = "neutral" | "info" | "warning" | "success" | "error";

export const KIND_ICONS: Record<Kind, string> = {
  bug: "i-material-symbols-bug-report-outline",
  suggestion: "i-material-symbols-lightbulb-outline"
};

export const STATE_LOOK: Record<State, { icon: string; tone: Tone }> = {
  received: { icon: "i-material-symbols-inbox-outline", tone: "info" },
  "under-review": { icon: "i-material-symbols-manage-search", tone: "warning" },
  done: { icon: "i-material-symbols-check-circle-outline", tone: "success" },
  declined: { icon: "i-material-symbols-block", tone: "error" }
};

/** The text colour class of a tone, at the step a coloured mark reads at. */
export const TONE_TEXT: Record<Tone, string> = {
  neutral: "text-muted",
  info: "text-info",
  warning: "text-warning",
  success: "text-success",
  error: "text-error"
};
