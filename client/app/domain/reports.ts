import { PICTURE_TYPES } from "../../shared/types";

import type { Attachment } from "../../shared/types";

// What the screens read off a report beyond what the service answers.

/** The word among `words` a value is, or nothing where it is none of them, as an address's query is read. */
export function oneOf<T extends string>(value: unknown, words: readonly T[]): T | undefined {
  return typeof value === "string" && (words as readonly string[]).includes(value) ? (value as T) : undefined;
}

/** Whether an attachment is a picture a browser shows, which the screen draws as a thumbnail. */
export function isPicture(attachment: Pick<Attachment, "type">): boolean {
  return (PICTURE_TYPES as readonly string[]).includes(attachment.type.toLowerCase());
}

const UNITS = ["B", "KB", "MB", "GB"] as const;

/** A size in bytes in its largest whole unit, one decimal under ten. */
export function sizeText(bytes: number, locale?: string): string {
  let value = bytes;
  let unit = 0;

  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }

  const digits = unit > 0 && value < 10 ? 1 : 0;

  return `${value.toLocaleString(locale, { maximumFractionDigits: digits })} ${UNITS[unit]}`;
}

/**
 * The console path a person came to the screen from: the router's own record of the address before this one, where
 * it stands outside the screen's own path. A path inside the screen says nothing of where the person met what they
 * write about.
 */
export function cameFromPath(back: unknown, screenPath: string): string | null {
  if (typeof back !== "string" || back === "") return null;

  return back === screenPath || back.startsWith(`${screenPath}/`) || back.startsWith(`${screenPath}?`) ? null : back;
}
