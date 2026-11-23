//! The unit's own store (`docs/design.md`, 「The store」): one SQLite file of the reports, their attachments'
//! names, types and sizes, their state histories, their replies and each side's read mark. Its schema is moved
//! by the numbered migrations under `migrations/`, applied in order as the store opens, each once, the file's
//! `user_version` counting those applied. A write is one transaction: a route reads, judges and writes inside
//! `transact`, so a report and its arrival entry, or a reply and the report's moments, land together.

use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::types::Value as Sql;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};

use crate::error::ApiError;
use crate::wire::{
  Attachment, Company, Context, Kind, Person, Reply, Report, ReportSummary, Side, State, StateEntry, Writer,
};

type Answer<T> = Result<T, ApiError>;

/// The migrations, in the order they apply; the file's `user_version` is how many of them it holds.
const MIGRATIONS: [&str; 1] = [include_str!("migrations/0001_reports.sql")];

/// The moment now, in epoch milliseconds.
pub fn now_ms() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|since| since.as_millis() as i64)
    .unwrap_or(0)
}

/// The moment an act is kept at: now, and always after every act this process kept before, so a read mark set
/// and an act made in one millisecond still stand in the order they were made.
pub fn moment() -> i64 {
  static LAST: AtomicI64 = AtomicI64::new(0);
  let mut last = LAST.load(Ordering::Relaxed);
  loop {
    let next = now_ms().max(last + 1);
    match LAST.compare_exchange_weak(last, next, Ordering::Relaxed, Ordering::Relaxed) {
      Ok(_) => return next,
      Err(standing) => last = standing,
    }
  }
}

/// A fresh id: 32 hexadecimal digits no other id of this process takes.
pub fn fresh_id() -> String {
  use std::collections::hash_map::RandomState;
  use std::hash::{BuildHasher, Hasher};
  use std::sync::atomic::AtomicU64;

  static COUNT: AtomicU64 = AtomicU64::new(0);
  let count = COUNT.fetch_add(1, Ordering::Relaxed);
  let half = |salt: u64| {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(salt);
    hasher.write_i64(now_ms());
    hasher.write_u64(count);
    hasher.finish()
  };
  format!("{:016x}{:016x}", half(1), half(2))
}

/// The SQLite file, shared by every route.
#[derive(Clone)]
pub struct Store {
  connection: Arc<Mutex<Connection>>,
}

impl Store {
  /// The store at `path`, its folder made and its migrations applied where they are not.
  pub fn open(path: &Path) -> Result<Self, String> {
    let at = |why: rusqlite::Error| format!("{}: {why}", path.display());
    if let Some(folder) = path.parent().filter(|folder| !folder.as_os_str().is_empty()) {
      std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    }
    let mut connection = Connection::open(path).map_err(at)?;
    connection
      .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
      .map_err(at)?;
    migrate(&mut connection).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(Self {
      connection: Arc::new(Mutex::new(connection)),
    })
  }

  /// A read over the store.
  pub fn read<T>(&self, work: impl FnOnce(&Connection) -> Answer<T>) -> Answer<T> {
    let guard = self.connection.lock().map_err(|_| ApiError::failed("the store's lock is poisoned"))?;
    work(&guard)
  }

  /// One transaction: `work` reads, judges and writes, and what it wrote lands only where it answers a success.
  pub fn transact<T>(&self, work: impl FnOnce(&Transaction) -> Answer<T>) -> Answer<T> {
    let mut guard = self.connection.lock().map_err(|_| ApiError::failed("the store's lock is poisoned"))?;
    let transaction = guard.transaction()?;
    let done = work(&transaction)?;
    transaction.commit()?;
    Ok(done)
  }
}

/// Every migration the file does not hold yet applied in order, each in a transaction of its own with the count
/// it raises `user_version` to. A file holding more than this binary knows is refused, since its schema is not
/// the one read here.
fn migrate(connection: &mut Connection) -> Result<(), String> {
  let held: i64 = connection
    .query_row("PRAGMA user_version", [], |row| row.get(0))
    .map_err(|why| why.to_string())?;
  let known = MIGRATIONS.len() as i64;
  if held > known {
    return Err(format!("the store holds {held} migrations, and this binary knows {known}"));
  }
  for (index, migration) in MIGRATIONS.iter().enumerate().skip(held as usize) {
    let number = index + 1;
    let transaction = connection.transaction().map_err(|why| why.to_string())?;
    transaction
      .execute_batch(migration)
      .and_then(|()| transaction.execute_batch(&format!("PRAGMA user_version = {number}")))
      .and_then(|()| transaction.commit())
      .map_err(|why| format!("migration {number}: {why}"))?;
  }
  Ok(())
}

// --- What a side has not read ------------------------------------------------------------------------------

/// Whether a report row is unread by `side` (`docs/design.md`, 「What each side has not read」): the maker's
/// arrival or a reply of the sender's after the maker's mark, a reply of the maker's or a state change after the
/// sender's; nothing for a reader of everything.
fn unread_by(side: Side) -> &'static str {
  match side {
    Side::Maker => "(maker_seen_at IS NULL OR maker_activity_at > maker_seen_at)",
    Side::Sender => {
      "(sender_activity_at IS NOT NULL AND (sender_seen_at IS NULL OR sender_activity_at > sender_seen_at))"
    }
    Side::Reader => "0",
  }
}

/// The reports a reader reads: every one, or a sender's own alone.
#[derive(Clone, Debug, Default)]
pub struct Filter {
  /// The sender whose reports alone are read; none for every report.
  pub sender: Option<String>,
  pub kind: Option<Kind>,
  pub state: Option<State>,
  pub company: Option<String>,
}

impl Filter {
  fn clause(&self) -> (String, Vec<Sql>) {
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    let mut add = |condition: &str, value: String| {
      values.push(Sql::Text(value));
      conditions.push(format!("{condition} = ?{}", values.len()));
    };
    if let Some(sender) = &self.sender {
      add("sender_user", sender.clone());
    }
    if let Some(kind) = self.kind {
      add("kind", kind.word());
    }
    if let Some(state) = self.state {
      add("state", state.word());
    }
    if let Some(company) = &self.company {
      add("company_id", company.clone());
    }
    let clause = if conditions.is_empty() {
      String::new()
    } else {
      format!("WHERE {}", conditions.join(" AND "))
    };
    (clause, values)
  }
}

// --- Reports -----------------------------------------------------------------------------------------------

/// The columns of a report's summary, `unread` judged for a side.
fn summary_columns(side: Side) -> String {
  format!(
    "id, kind, title, state, sender_user, sender_name, company_id, company_name, sent_at, activity_at, \
     (SELECT COUNT(*) FROM replies WHERE replies.report_id = reports.id) AS reply_count, {} AS unread",
    unread_by(side)
  )
}

fn summary_row(row: &Row) -> rusqlite::Result<ReportSummary> {
  let company_id: Option<String> = row.get("company_id")?;
  let company_name: Option<String> = row.get("company_name")?;
  Ok(ReportSummary {
    id: row.get("id")?,
    kind: Kind::of(&row.get::<_, String>("kind")?),
    title: row.get("title")?,
    state: State::of(&row.get::<_, String>("state")?),
    sender: Person {
      user: row.get("sender_user")?,
      name: row.get("sender_name")?,
    },
    company: company_id.map(|id| Company {
      id,
      name: company_name.unwrap_or_default(),
    }),
    sent_at: row.get("sent_at")?,
    activity_at: row.get("activity_at")?,
    replies: row.get("reply_count")?,
    unread: row.get("unread")?,
  })
}

/// A page of the reports `filter` names, newest activity first, `after` read already and at most `limit`, and
/// whether more stand past it.
pub fn summaries(
  connection: &Connection,
  side: Side,
  filter: &Filter,
  after: i64,
  limit: i64,
) -> Answer<(Vec<ReportSummary>, bool)> {
  let (clause, mut values) = filter.clause();
  values.push(Sql::Integer(limit + 1));
  values.push(Sql::Integer(after));
  let count = values.len();
  let sql = format!(
    "SELECT {} FROM reports {clause} ORDER BY activity_at DESC, id DESC LIMIT ?{} OFFSET ?{count}",
    summary_columns(side),
    count - 1,
  );
  let mut statement = connection.prepare(&sql)?;
  let mut rows: Vec<ReportSummary> =
    statement.query_map(params_from_iter(values), summary_row)?.collect::<Result<_, _>>()?;
  let more = rows.len() as i64 > limit;
  rows.truncate(limit as usize);
  Ok((rows, more))
}

/// How many of the reports `filter` names are unread by `side`.
pub fn unread_count(connection: &Connection, side: Side, filter: &Filter) -> Answer<i64> {
  let (clause, values) = filter.clause();
  let joined = if clause.is_empty() { "WHERE" } else { "AND" };
  let sql = format!("SELECT COUNT(*) FROM reports {clause} {joined} {}", unread_by(side));
  Ok(connection.query_row(&sql, params_from_iter(values), |row| row.get(0))?)
}

/// What a route judges a report by before it reads or writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Head {
  pub id: String,
  pub sender: String,
  pub title: String,
  pub kind: Kind,
  pub state: State,
}

/// The head of a report, where it stands.
pub fn head(connection: &Connection, id: &str) -> Answer<Option<Head>> {
  Ok(
    connection
      .query_row(
        "SELECT id, sender_user, title, kind, state FROM reports WHERE id = ?1",
        params![id],
        |row| {
          Ok(Head {
            id: row.get(0)?,
            sender: row.get(1)?,
            title: row.get(2)?,
            kind: Kind::of(&row.get::<_, String>(3)?),
            state: State::of(&row.get::<_, String>(4)?),
          })
        },
      )
      .optional()?,
  )
}

/// A report whole, `unread` judged for `side`, where it stands.
pub fn report(connection: &Connection, id: &str, side: Side) -> Answer<Option<Report>> {
  let sql = format!(
    "SELECT {}, body, context_path, context_console_version, context_browser FROM reports WHERE id = ?1",
    summary_columns(side)
  );
  let found = connection
    .query_row(&sql, params![id], |row| {
      Ok((
        summary_row(row)?,
        row.get::<_, String>("body")?,
        Context {
          path: row.get("context_path")?,
          console_version: row.get("context_console_version")?,
          browser: row.get("context_browser")?,
        },
      ))
    })
    .optional()?;
  let Some((summary, body, context)) = found else {
    return Ok(None);
  };
  Ok(Some(Report {
    attachments: attachments(connection, id)?,
    states: states(connection, id)?,
    replies: replies(connection, id)?,
    id: summary.id,
    kind: summary.kind,
    title: summary.title,
    state: summary.state,
    sender: summary.sender,
    company: summary.company,
    sent_at: summary.sent_at,
    activity_at: summary.activity_at,
    unread: summary.unread,
    body,
    context,
  }))
}

fn attachment_row(row: &Row) -> rusqlite::Result<Attachment> {
  Ok(Attachment {
    id: row.get("id")?,
    name: row.get("name")?,
    kind: row.get("type")?,
    size: row.get("size")?,
  })
}

fn attachments(connection: &Connection, report: &str) -> Answer<Vec<Attachment>> {
  let mut statement =
    connection.prepare("SELECT * FROM attachments WHERE report_id = ?1 ORDER BY position")?;
  Ok(statement.query_map(params![report], attachment_row)?.collect::<Result<_, _>>()?)
}

/// One attachment of a report, where it stands on that report.
pub fn attachment(connection: &Connection, report: &str, id: &str) -> Answer<Option<Attachment>> {
  Ok(
    connection
      .query_row(
        "SELECT * FROM attachments WHERE report_id = ?1 AND id = ?2",
        params![report, id],
        attachment_row,
      )
      .optional()?,
  )
}

fn states(connection: &Connection, report: &str) -> Answer<Vec<StateEntry>> {
  let mut statement = connection.prepare("SELECT * FROM states WHERE report_id = ?1 ORDER BY id")?;
  let rows = statement.query_map(params![report], |row| {
    Ok(StateEntry {
      state: State::of(&row.get::<_, String>("state")?),
      by: Writer {
        user: row.get("by_user")?,
        name: row.get("by_name")?,
        side: Side::of(&row.get::<_, String>("side")?),
      },
      at: row.get("at")?,
    })
  })?;
  Ok(rows.collect::<Result<_, _>>()?)
}

fn replies(connection: &Connection, report: &str) -> Answer<Vec<Reply>> {
  let mut statement = connection.prepare("SELECT * FROM replies WHERE report_id = ?1 ORDER BY seq")?;
  let rows = statement.query_map(params![report], |row| {
    Ok(Reply {
      id: row.get("id")?,
      side: Side::of(&row.get::<_, String>("side")?),
      by: Person {
        user: row.get("by_user")?,
        name: row.get("by_name")?,
      },
      body: row.get("body")?,
      at: row.get("at")?,
    })
  })?;
  Ok(rows.collect::<Result<_, _>>()?)
}

/// A report as it is sent: what the sender wrote, who they are and where they stood, and its files' names.
#[derive(Clone, Debug)]
pub struct Sent {
  pub id: String,
  pub kind: Kind,
  pub title: String,
  pub body: String,
  pub context: Context,
  pub sender: Person,
  pub company: Option<Company>,
  pub attachments: Vec<Attachment>,
  pub at: i64,
}

/// A report written with its attachments' rows and its arrival entry, by the sender.
pub fn insert_report(transaction: &Transaction, sent: &Sent) -> Answer<()> {
  let received = State::Received.word();
  transaction.execute(
    "INSERT INTO reports (id, kind, title, body, state, sender_user, sender_name, company_id, company_name, \
     context_path, context_console_version, context_browser, sent_at, activity_at, maker_activity_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?13)",
    params![
      sent.id,
      sent.kind.word(),
      sent.title,
      sent.body,
      received,
      sent.sender.user,
      sent.sender.name,
      sent.company.as_ref().map(|company| &company.id),
      sent.company.as_ref().map(|company| &company.name),
      sent.context.path,
      sent.context.console_version,
      sent.context.browser,
      sent.at,
    ],
  )?;
  for (position, attachment) in sent.attachments.iter().enumerate() {
    transaction.execute(
      "INSERT INTO attachments (id, report_id, position, name, type, size) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![
        attachment.id,
        sent.id,
        position as i64,
        attachment.name,
        attachment.kind,
        attachment.size
      ],
    )?;
  }
  transaction.execute(
    "INSERT INTO states (report_id, state, by_user, by_name, side, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![sent.id, received, sent.sender.user, sent.sender.name, Side::Sender.word(), sent.at],
  )?;
  Ok(())
}

/// The report's moments moved for an act of `side` at `at`: its activity, and the other side's moment to read.
fn acted(transaction: &Transaction, report: &str, side: Side, at: i64) -> Answer<()> {
  let other = match side {
    Side::Maker => "sender_activity_at",
    Side::Sender | Side::Reader => "maker_activity_at",
  };
  transaction.execute(
    &format!("UPDATE reports SET activity_at = ?2, {other} = ?2 WHERE id = ?1"),
    params![report, at],
  )?;
  Ok(())
}

/// A reply written on a report by `by` of `side`, the report's moments moved with it.
pub fn insert_reply(transaction: &Transaction, report: &str, reply: &Reply) -> Answer<()> {
  transaction.execute(
    "INSERT INTO replies (id, report_id, side, by_user, by_name, body, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    params![reply.id, report, reply.side.word(), reply.by.user, reply.by.name, reply.body, reply.at],
  )?;
  acted(transaction, report, reply.side, reply.at)
}

/// A report moved to `state` by the maker `by`, its entry kept and its moments moved; false, with nothing
/// written, where it stands in that state already.
pub fn move_state(
  transaction: &Transaction,
  report: &str,
  state: State,
  by: &Person,
  at: i64,
) -> Answer<bool> {
  let changed = transaction.execute(
    "UPDATE reports SET state = ?2 WHERE id = ?1 AND state <> ?2",
    params![report, state.word()],
  )?;
  if changed == 0 {
    return Ok(false);
  }
  transaction.execute(
    "INSERT INTO states (report_id, state, by_user, by_name, side, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![report, state.word(), by.user, by.name, Side::Maker.word(), at],
  )?;
  acted(transaction, report, Side::Maker, at)?;
  Ok(true)
}

/// A side's read mark on a report set to `at`.
pub fn mark_seen(connection: &Connection, report: &str, side: Side, at: i64) -> Answer<()> {
  let mark = match side {
    Side::Maker => "maker_seen_at",
    Side::Sender => "sender_seen_at",
    Side::Reader => return Ok(()),
  };
  connection.execute(&format!("UPDATE reports SET {mark} = ?2 WHERE id = ?1"), params![report, at])?;
  Ok(())
}

/// The companies the reports were sent from, each named as the latest report from it names it, by name.
pub fn companies(connection: &Connection) -> Answer<Vec<Company>> {
  let mut statement = connection.prepare(
    "SELECT company_id, company_name FROM reports AS latest \
     WHERE company_id IS NOT NULL AND NOT EXISTS ( \
       SELECT 1 FROM reports AS later WHERE later.company_id = latest.company_id \
       AND (later.sent_at > latest.sent_at OR (later.sent_at = latest.sent_at AND later.id > latest.id))) \
     ORDER BY company_name, company_id",
  )?;
  let rows = statement.query_map([], |row| {
    Ok(Company {
      id: row.get(0)?,
      name: row.get(1)?,
    })
  })?;
  Ok(rows.collect::<Result<_, _>>()?)
}
