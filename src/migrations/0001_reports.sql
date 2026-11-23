-- The reports, their attachments' names, types and sizes, their state histories, their replies and each side's
-- read mark (docs/design.md, 「The store」). The attachments' bytes stand in the files folder under each id.

CREATE TABLE reports (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  state TEXT NOT NULL,
  sender_user TEXT NOT NULL,
  sender_name TEXT NOT NULL,
  -- Both null where the account service placed the sender in no company.
  company_id TEXT,
  company_name TEXT,
  context_path TEXT NOT NULL,
  context_console_version TEXT NOT NULL,
  context_browser TEXT NOT NULL,
  sent_at INTEGER NOT NULL,
  -- The latest act of either side: the arrival, a reply or a state change.
  activity_at INTEGER NOT NULL,
  -- The latest act the maker has to read: the arrival or a reply of the sender's.
  maker_activity_at INTEGER NOT NULL,
  -- The latest act the sender has to read: a reply of the maker's or a state change; null until one.
  sender_activity_at INTEGER,
  -- The moment each side last opened the report; null until it did.
  maker_seen_at INTEGER,
  sender_seen_at INTEGER
);
CREATE INDEX reports_activity ON reports (activity_at DESC, id DESC);
CREATE INDEX reports_sender ON reports (sender_user, activity_at DESC);

CREATE TABLE attachments (
  id TEXT PRIMARY KEY,
  report_id TEXT NOT NULL REFERENCES reports (id),
  -- The order the sender attached them in.
  position INTEGER NOT NULL,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  size INTEGER NOT NULL
);
CREATE INDEX attachments_report ON attachments (report_id, position);

CREATE TABLE states (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  report_id TEXT NOT NULL REFERENCES reports (id),
  state TEXT NOT NULL,
  by_user TEXT NOT NULL,
  by_name TEXT NOT NULL,
  side TEXT NOT NULL,
  at INTEGER NOT NULL
);
CREATE INDEX states_report ON states (report_id, id);

CREATE TABLE replies (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  report_id TEXT NOT NULL REFERENCES reports (id),
  side TEXT NOT NULL,
  by_user TEXT NOT NULL,
  by_name TEXT NOT NULL,
  body TEXT NOT NULL,
  at INTEGER NOT NULL
);
CREATE INDEX replies_report ON replies (report_id, seq);
