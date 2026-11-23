// The voc service's wire shapes as the screens read them (`docs/api.md`): camelCase keys, moments in epoch
// milliseconds, ids as strings.

export interface ListPage<T> {
  items: T[];
  more: boolean;
}

/** The side a reader stands on: the maker, a sender, or a reader of everything (`docs/design.md`, 「The two sides」). */
export const SIDES = ["maker", "sender", "reader"] as const;

export type Side = (typeof SIDES)[number];

/** Who the reader is (`GET /me`). */
export interface Me {
  user: string | null;
  side: Side;
}

/** What a report is: a fault met, or a change wanted. */
export const KINDS = ["bug", "suggestion"] as const;

export type Kind = (typeof KINDS)[number];

/** The maker's judgement of a report, in the order it usually moves (`docs/design.md`, 「Its state」). */
export const STATES = ["received", "under-review", "done", "declined"] as const;

export type State = (typeof STATES)[number];

/** A person as the account service named them as they wrote. */
export interface Person {
  user: string;
  name: string;
}

/** A company as the account service named it as a report was sent. */
export interface Company {
  id: string;
  name: string;
}

/** Where a report was sent from, as the sender's console read it; each empty where it read none. */
export interface Context {
  path: string;
  consoleVersion: string;
  browser: string;
}

/** One report as a list holds it. */
export interface ReportSummary {
  id: string;
  kind: Kind;
  title: string;
  state: State;
  sender: Person;
  company: Company | null;
  sentAt: number;
  activityAt: number;
  /** How many replies it holds. */
  replies: number;
  unread: boolean;
}

export interface Attachment {
  id: string;
  name: string;
  type: string;
  /** In bytes. */
  size: number;
}

/** Who wrote an entry of a report's history. */
export interface Writer extends Person {
  side: Side;
}

export interface StateEntry {
  state: State;
  by: Writer;
  at: number;
}

export interface Reply {
  id: string;
  side: Side;
  by: Person;
  body: string;
  at: number;
}

/** One report whole: its summary but the reply count, and what the summary leaves out. */
export interface Report extends Omit<ReportSummary, "replies"> {
  body: string;
  context: Context;
  attachments: Attachment[];
  states: StateEntry[];
  replies: Reply[];
}

/** The `report` part of `POST /reports`. */
export interface NewReport {
  kind: Kind;
  title: string;
  body: string;
  context: Context;
}

/** The inbox's filters, each left out where it filters nothing. */
export interface ReportFilter {
  kind?: Kind;
  state?: State;
  company?: string;
}

/** A report's and a reply's bounds (`docs/api.md`, 「Reports」). */
export const TITLE_MAX = 200;
export const BODY_MAX = 20_000;
export const FILE_NAME_MAX = 200;

/**
 * What one report carries, as the service's settings ship them (`[attachments] max_files` and `max_bytes`). A
 * deployment may set others; these are the hints the form shows, and the service's answer is the judge.
 */
export const MAX_FILES = 5;
export const MAX_BYTES = 10 * 1024 * 1024;

/** The picture types a browser shows, which the service answers inline. */
export const PICTURE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"] as const;
