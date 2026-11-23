//! The service's wire types (`docs/api.md`): what each route takes and answers, in the platform's format, each
//! with its JSON Schema for the OpenAPI document.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A word read as one of an enum's variants.
fn word_of<T: for<'a> Deserialize<'a>>(word: &str) -> Option<T> {
  serde_json::from_value(Value::String(word.to_owned())).ok()
}

/// The word an enum's variant is written as.
fn word<T: Serialize>(value: T) -> String {
  serde_json::to_value(value)
    .ok()
    .and_then(|value| value.as_str().map(str::to_owned))
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------------------
// The reader

/// The side a reader stands on (`docs/design.md`, 「The two sides」).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Side {
  /// The platform administrators: they read every report, move its state and answer it.
  Maker,
  /// A person who is not a platform administrator: they send reports and read their own.
  Sender,
  /// The service token alone: it reads every report and writes nothing.
  Reader,
}

impl Side {
  pub fn word(self) -> String {
    word(self)
  }

  /// The side a stored word names, the sender's where it names none.
  pub fn of(stored: &str) -> Self {
    word_of(stored).unwrap_or(Self::Sender)
  }
}

/// `GET /me`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Me {
  /// The account service's user id; null for a reader of everything.
  pub user: Option<String>,
  pub side: Side,
  /// What one report may carry, as the settings bound it, so a form holds files to the bounds the send is held to.
  pub limits: Limits,
}

/// What one report may carry (`docs/api.md`, 「Reports」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
  /// How many files at most.
  pub max_files: usize,
  /// How large one file may be, in bytes.
  pub max_bytes: usize,
}

// ---------------------------------------------------------------------------------------------------------
// Reports

/// What a report is (`docs/design.md`, 「A report」).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
  /// A fault the sender met.
  Bug,
  /// Something the sender wants changed or added.
  Suggestion,
}

impl Kind {
  pub fn word(self) -> String {
    word(self)
  }

  /// The kind a word names, where it names one.
  pub fn read(text: &str) -> Option<Self> {
    word_of(text)
  }

  /// The kind a stored word names, a bug where it names none.
  pub fn of(stored: &str) -> Self {
    Self::read(stored).unwrap_or(Self::Bug)
  }
}

/// The maker's judgement of a report (`docs/design.md`, 「Its state」).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum State {
  /// As it arrives.
  Received,
  /// While the maker looks into it.
  UnderReview,
  /// Answered or carried out.
  Done,
  /// Not to be carried out.
  Declined,
}

impl State {
  pub fn word(self) -> String {
    word(self)
  }

  /// The state a word names, where it names one.
  pub fn read(text: &str) -> Option<Self> {
    word_of(text)
  }

  /// The state a stored word names, received where it names none.
  pub fn of(stored: &str) -> Self {
    Self::read(stored).unwrap_or(Self::Received)
  }
}

/// Who sent a report, as the account service named them as it was sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Person {
  /// The account service's user id.
  pub user: String,
  /// Their full name, else their login where no full name stands.
  pub name: String,
}

/// A company as the account service named it as a report was sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Company {
  /// The account service's unit id.
  pub id: String,
  pub name: String,
}

/// Where a report was sent from, as the sender's console read it; each empty where it read none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Context {
  /// The console screen's path the sender came from.
  #[serde(default)]
  pub path: String,
  /// The console's version.
  #[serde(default)]
  pub console_version: String,
  /// The browser the console runs in.
  #[serde(default)]
  pub browser: String,
}

/// The `report` part of `POST /reports`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NewReport {
  /// `bug` or `suggestion`.
  pub kind: String,
  /// 1 to 200 characters once trimmed.
  pub title: String,
  /// Markdown, 1 to 20,000 characters.
  pub body: String,
  #[serde(default)]
  pub context: Context,
}

/// One report as a list holds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportSummary {
  pub id: String,
  pub kind: Kind,
  pub title: String,
  pub state: State,
  pub sender: Person,
  /// Null where the sender stood in no company.
  pub company: Option<Company>,
  pub sent_at: i64,
  /// Its latest act of either side: its arrival, a reply or a state change.
  pub activity_at: i64,
  /// How many replies it holds.
  pub replies: i64,
  /// Whether it is unread by the caller's side; false for a reader of everything.
  pub unread: bool,
}

/// One attachment of a report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Attachment {
  pub id: String,
  /// The file name the sender's browser gave it.
  pub name: String,
  /// The type the sender's browser gave it, `application/octet-stream` where it gave none.
  #[serde(rename = "type")]
  pub kind: String,
  /// In bytes.
  pub size: i64,
}

/// Who wrote an entry of a report's history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Writer {
  pub user: String,
  /// As the account service named them as the entry was written.
  pub name: String,
  pub side: Side,
}

/// One entry of a report's state history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StateEntry {
  pub state: State,
  pub by: Writer,
  pub at: i64,
}

/// One reply on a report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reply {
  pub id: String,
  pub side: Side,
  pub by: Person,
  /// Markdown.
  pub body: String,
  pub at: i64,
}

/// One report whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Report {
  pub id: String,
  pub kind: Kind,
  pub title: String,
  pub state: State,
  pub sender: Person,
  /// Null where the sender stood in no company.
  pub company: Option<Company>,
  pub sent_at: i64,
  /// Its latest act of either side: its arrival, a reply or a state change.
  pub activity_at: i64,
  /// Whether it is unread by the caller's side; false for a reader of everything.
  pub unread: bool,
  /// Markdown.
  pub body: String,
  pub context: Context,
  pub attachments: Vec<Attachment>,
  /// Oldest first, the arrival first.
  pub states: Vec<StateEntry>,
  /// Oldest first.
  pub replies: Vec<Reply>,
}

/// `POST /reports/{reportId}/replies`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NewReply {
  /// Markdown, 1 to 20,000 characters.
  pub body: String,
}

/// `PUT /reports/{reportId}/state`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StateWrite {
  /// `received`, `under-review`, `done` or `declined`.
  pub state: String,
}

/// The reports read.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct ReportsQuery {
  /// The kind alone; the maker and a reader of everything.
  #[serde(default)]
  pub kind: Option<Kind>,
  /// The state alone; the maker and a reader of everything.
  #[serde(default)]
  pub state: Option<State>,
  /// The company's id alone; the maker and a reader of everything.
  #[serde(default)]
  pub company: Option<String>,
  /// The number of rows read so far.
  #[serde(default)]
  pub after: Option<i64>,
  /// At most this many, 50 where none is given, never past 200.
  #[serde(default)]
  pub limit: Option<i64>,
}

/// `GET /unread`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Unread {
  /// The reports unread by the caller's side; 0 for a reader of everything.
  pub count: i64,
}

// ---------------------------------------------------------------------------------------------------------
// The platform's shapes

/// A page of members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Page<T> {
  pub items: Vec<T>,
  /// Whether anything stands past the last item.
  pub more: bool,
}

/// `GET /health`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Health {
  pub ready: bool,
  /// Null when ready.
  pub reason: Option<String>,
  pub version: String,
}

/// A failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WireError {
  /// kebab-case.
  pub code: String,
  /// An English sentence.
  pub error: String,
}

/// One event the service publishes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StreamEvent {
  pub id: i64,
  pub kind: String,
  pub at: i64,
  pub data: Value,
}

/// Which events to read.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct EventsQuery {
  #[serde(default)]
  pub after: Option<i64>,
  /// The kinds, comma-separated.
  #[serde(default)]
  pub kinds: Option<String>,
  /// The most events a page holds, 50 where none is given, at most 200.
  #[serde(default)]
  pub limit: Option<i64>,
}

/// `report.received`, `report.answered` and `report.followed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportEvent {
  pub report_id: String,
  /// The sender's user id.
  pub sender: String,
  pub title: String,
  pub kind: Kind,
}

/// `report.changed`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReportChanged {
  #[serde(flatten)]
  pub report: ReportEvent,
  /// The state it moved to.
  pub state: State,
}
