//! The reports (`docs/api.md`, 「Reports」): a sender's report sent whole with its files or not at all, the reports
//! each caller reads, one report whole, an attachment's bytes, the replies of both sides, the maker's state moves
//! and each side's read mark.

use std::path::{Path as FilePath, PathBuf};

use axum::body::Bytes;
use axum::extract::multipart::{Field, MultipartRejection};
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use super::{Answer, Served, body, bounds, path, query};
use crate::access::{Caller, caller, upstream};
use crate::app::App;
use crate::error::ApiError;
use crate::events::{ANSWERED, CHANGED, FOLLOWED, RECEIVED};
use crate::peers;
use crate::store::{self, Filter, Head, Sent, fresh_id, moment};
use crate::wire::{
  Attachment, Kind, NewReply, NewReport, Page, Person, Reply, Report, ReportChanged, ReportEvent,
  ReportSummary, ReportsQuery, Side, State as ReportState, StateWrite,
};

/// The most characters a title holds once trimmed.
const TITLE_MOST: usize = 200;
/// The most characters a report's or a reply's body holds.
const BODY_MOST: usize = 20_000;
/// The most characters an attachment's file name holds.
const FILE_NAME_MOST: usize = 200;
/// The most bytes the `report` part may run to: its title and body at their longest, each character escaped,
/// and its context, with room to spare.
const REPORT_PART_MOST: usize = 1 << 20;
/// The type of a file whose browser named none.
const UNNAMED_TYPE: &str = "application/octet-stream";
/// The pictures a browser shows and runs nothing of, answered inline; any other file is handed as a download.
const INLINE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];
/// What a file name keeps unencoded in `filename*` (RFC 8187's attr-char).
const ATTR_CHAR: &AsciiSet = &NON_ALPHANUMERIC
  .remove(b'!')
  .remove(b'#')
  .remove(b'$')
  .remove(b'&')
  .remove(b'+')
  .remove(b'-')
  .remove(b'.')
  .remove(b'^')
  .remove(b'_')
  .remove(b'`')
  .remove(b'|')
  .remove(b'~');

fn not_found(id: &str) -> ApiError {
  ApiError::not_found(format!("No report {id} stands for this reader."))
}

/// The head of a report the caller reads; another sender's, like one that does not stand, is not found.
fn readable(app: &App, caller: &Caller, id: &str) -> Answer<Head> {
  app
    .store
    .read(|connection| store::head(connection, id))?
    .filter(|head| caller.reads(&head.sender))
    .ok_or_else(|| not_found(id))
}

/// A report whole as `side` reads it, which a write has just made or read.
fn whole(app: &App, id: &str, side: Side) -> Answer<Report> {
  app
    .store
    .read(|connection| store::report(connection, id, side))?
    .ok_or_else(|| ApiError::failed(format!("the report {id} is gone from the store")))
}

/// The data every event of a report carries.
fn event_of(head: &Head) -> ReportEvent {
  ReportEvent {
    report_id: head.id.clone(),
    sender: head.sender.clone(),
    title: head.title.clone(),
    kind: head.kind,
  }
}

/// What the account service calls the writer of an act.
async fn writer_name(app: &App, caller: &Caller, user: &str) -> Answer<String> {
  let reach = peers::accounts(&app.settings).map_err(upstream)?;
  peers::name(&app.http, &reach, caller, user).await.map_err(upstream)
}

/// A body of markdown held to its bounds: something besides blanks, at most 20,000 characters.
fn markdown(what: &str, text: &str) -> Answer<()> {
  if text.trim().is_empty() || text.chars().count() > BODY_MOST {
    return Err(ApiError::bad_report(format!("{what} is 1 to {BODY_MOST} characters, not blank.")));
  }
  Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// The reports read

pub(super) async fn list(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Query<ReportsQuery>, QueryRejection>,
) -> Answer<Json<Page<ReportSummary>>> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  let given = query(given)?;
  let (after, limit) = bounds(given.after, given.limit)?;
  // A sender reads their own reports alone, and a filter they name is answered as if they named none.
  let filter = if caller.reads_all() {
    Filter {
      sender: None,
      kind: given.kind,
      state: given.state,
      company: given.company.filter(|company| !company.is_empty()),
    }
  } else {
    Filter {
      sender: caller.user.clone(),
      ..Filter::default()
    }
  };
  let (items, more) = app
    .store
    .read(|connection| store::summaries(connection, caller.side, &filter, after, limit))?;
  Ok(Json(Page { items, more }))
}

pub(super) async fn read(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Path<String>, PathRejection>,
) -> Answer<Json<Report>> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  let id = path(given)?;
  let head = readable(app, &caller, &id)?;
  Ok(Json(whole(app, &head.id, caller.side)?))
}

// ---------------------------------------------------------------------------------------------------------
// A report sent

/// One file of a report's form, read whole.
struct Upload {
  name: String,
  kind: String,
  bytes: Bytes,
}

fn unreadable_form(why: impl std::fmt::Display) -> ApiError {
  ApiError::bad_request(format!("The form does not read: {why}"))
}

/// A part's bytes, read as they come and refused by `refusal` once they run past `most`.
async fn part_bytes(field: &mut Field<'_>, most: usize, refusal: impl Fn() -> ApiError) -> Answer<Bytes> {
  let mut bytes = Vec::new();
  while let Some(chunk) = field.chunk().await.map_err(unreadable_form)? {
    if bytes.len() + chunk.len() > most {
      return Err(refusal());
    }
    bytes.extend_from_slice(&chunk);
  }
  Ok(Bytes::from(bytes))
}

/// A form's `report` part and its files, each file held to the settings' bounds as it is read.
async fn form(app: &App, mut multipart: Multipart) -> Answer<(Option<Bytes>, Vec<Upload>)> {
  let bounds = &app.settings.attachments;
  let mut report = None;
  let mut files = Vec::new();
  while let Some(mut field) = multipart.next_field().await.map_err(unreadable_form)? {
    match field.name().unwrap_or_default() {
      "report" => {
        if report.is_some() {
          return Err(ApiError::bad_report("The form carries one `report` part, not two."));
        }
        let refusal = || ApiError::bad_report("The `report` part is longer than any report runs to.");
        report = Some(part_bytes(&mut field, REPORT_PART_MOST, refusal).await?);
      }
      "files" => {
        if files.len() >= bounds.max_files {
          return Err(ApiError::too_many_files(bounds.max_files));
        }
        let name = field.file_name().unwrap_or_default().to_owned();
        if name.is_empty() || name.chars().count() > FILE_NAME_MOST {
          return Err(ApiError::bad_report(format!("A file's name is 1 to {FILE_NAME_MOST} characters.")));
        }
        let kind = field
          .content_type()
          .map(str::trim)
          .filter(|kind| !kind.is_empty())
          .unwrap_or(UNNAMED_TYPE)
          .to_owned();
        let most = bounds.max_bytes;
        let refusal =
          || ApiError::file_too_large(format!("{name} is past the {most} bytes a file may hold."));
        let bytes = part_bytes(&mut field, most, refusal).await?;
        if bytes.is_empty() {
          return Err(ApiError::file_too_large(format!("{name} is empty; a file holds 1 byte or more.")));
        }
        files.push(Upload { name, kind, bytes });
      }
      other => {
        return Err(ApiError::bad_request(format!(
          "The form takes a `report` part and `files` parts, not `{other}`."
        )));
      }
    }
  }
  Ok((report, files))
}

/// What the `report` part says, held to its bounds: its kind, its trimmed title, its body and its context.
fn written(part: Option<Bytes>) -> Answer<(Kind, NewReport)> {
  let part = part.ok_or_else(|| ApiError::bad_report("The form carries no `report` part."))?;
  let mut report: NewReport = serde_json::from_slice(&part)
    .map_err(|why| ApiError::bad_report(format!("The `report` part does not read: {why}")))?;
  let kind = Kind::read(&report.kind)
    .ok_or_else(|| ApiError::bad_report("A report's kind is `bug` or `suggestion`."))?;
  report.title = report.title.trim().to_owned();
  let title = report.title.chars().count();
  if title == 0 || title > TITLE_MOST {
    return Err(ApiError::bad_report(format!("A title is 1 to {TITLE_MOST} characters once trimmed.")));
  }
  markdown("A report's body", &report.body)?;
  Ok((kind, report))
}

/// The files written into the folder under their ids; where one cannot be, those written are taken away again.
async fn keep_files(folder: &FilePath, files: &[(Attachment, Bytes)]) -> Answer<()> {
  let mut written: Vec<PathBuf> = Vec::new();
  for (attachment, bytes) in files {
    let path = folder.join(&attachment.id);
    if let Err(why) = tokio::fs::write(&path, bytes).await {
      drop_files(&written).await;
      return Err(ApiError::failed(format!("the attachment {} could not be kept: {why}", attachment.name)));
    }
    written.push(path);
  }
  Ok(())
}

async fn drop_files(paths: &[PathBuf]) {
  for path in paths {
    let _ = tokio::fs::remove_file(path).await;
  }
}

pub(super) async fn send(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Multipart, MultipartRejection>,
) -> Answer<Response> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  let user = caller.writer()?.to_owned();
  if caller.side == Side::Maker {
    return Err(ApiError::maker_side());
  }
  let multipart = given.map_err(|why| ApiError::bad_request(why.body_text()))?;
  let (part, uploads) = form(app, multipart).await?;
  let (kind, report) = written(part)?;

  let reach = peers::accounts(&app.settings).map_err(upstream)?;
  let name = peers::name(&app.http, &reach, &caller, &user).await.map_err(upstream)?;
  let company = peers::company(&app.http, &reach, &caller, &user).await.map_err(upstream)?;

  let files: Vec<(Attachment, Bytes)> = uploads
    .into_iter()
    .map(|upload| {
      let attachment = Attachment {
        id: fresh_id(),
        name: upload.name,
        kind: upload.kind,
        size: upload.bytes.len() as i64,
      };
      (attachment, upload.bytes)
    })
    .collect();
  let sent = Sent {
    id: fresh_id(),
    kind,
    title: report.title,
    body: report.body,
    context: report.context,
    sender: Person { user, name },
    company,
    attachments: files.iter().map(|(attachment, _)| attachment.clone()).collect(),
    at: moment(),
  };
  let folder = &app.settings.store.files;
  keep_files(folder, &files).await?;
  if let Err(why) = app.store.transact(|transaction| store::insert_report(transaction, &sent)) {
    let paths: Vec<PathBuf> = files.iter().map(|(attachment, _)| folder.join(&attachment.id)).collect();
    drop_files(&paths).await;
    return Err(why);
  }
  app.events.publish(
    RECEIVED,
    ReportEvent {
      report_id: sent.id.clone(),
      sender: sent.sender.user.clone(),
      title: sent.title.clone(),
      kind,
    },
  );
  Ok((StatusCode::CREATED, Json(whole(app, &sent.id, caller.side)?)).into_response())
}

// ---------------------------------------------------------------------------------------------------------
// An attachment's bytes

/// The `Content-Disposition` an attachment is handed with: inline for a picture a browser shows, else as a file
/// to save, its name in ASCII for an old reader and whole in `filename*`.
fn disposition(inline: bool, name: &str) -> HeaderValue {
  let plain: String = name
    .chars()
    .map(|character| match character {
      ' '..='~' if character != '"' && character != '\\' => character,
      _ => '_',
    })
    .collect();
  let encoded = utf8_percent_encode(name, ATTR_CHAR);
  let how = if inline { "inline" } else { "attachment" };
  HeaderValue::from_str(&format!("{how}; filename=\"{plain}\"; filename*=UTF-8''{encoded}"))
    .unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

pub(super) async fn attachment(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Path<(String, String)>, PathRejection>,
) -> Answer<Response> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  let (id, attachment_id) = path(given)?;
  let head = readable(app, &caller, &id)?;
  let attachment = app
    .store
    .read(|connection| store::attachment(connection, &head.id, &attachment_id))?
    .ok_or_else(|| {
      ApiError::not_found(format!("No attachment {attachment_id} stands on the report {id}."))
    })?;
  let bytes = tokio::fs::read(app.settings.store.files.join(&attachment.id)).await.map_err(|why| {
    ApiError::failed(format!("the attachment {} could not be read: {why}", attachment.id))
  })?;
  let essence = attachment.kind.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
  let inline = INLINE_TYPES.contains(&essence.as_str());
  let kind = if inline {
    HeaderValue::from_str(&essence)
  } else {
    HeaderValue::from_str(&attachment.kind)
  }
  .unwrap_or_else(|_| HeaderValue::from_static(UNNAMED_TYPE));
  Ok(
    (
      [
        (header::CONTENT_TYPE, kind),
        (header::CONTENT_DISPOSITION, disposition(inline, &attachment.name)),
        (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
      ],
      bytes,
    )
      .into_response(),
  )
}

// ---------------------------------------------------------------------------------------------------------
// Replies, states and read marks

pub(super) async fn reply(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Path<String>, PathRejection>,
  bytes: Bytes,
) -> Answer<Response> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  let user = caller.writer()?.to_owned();
  let id = path(given)?;
  let written: NewReply = body(&bytes)?;
  markdown("A reply's body", &written.body)?;
  let head = readable(app, &caller, &id)?;
  let name = writer_name(app, &caller, &user).await?;
  let reply = Reply {
    id: fresh_id(),
    side: caller.side,
    by: Person { user, name },
    body: written.body,
    at: moment(),
  };
  app.store.transact(|transaction| store::insert_reply(transaction, &head.id, &reply))?;
  let kind = if caller.side == Side::Maker { ANSWERED } else { FOLLOWED };
  app.events.publish(kind, event_of(&head));
  Ok((StatusCode::CREATED, Json(reply)).into_response())
}

pub(super) async fn move_state(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Path<String>, PathRejection>,
  bytes: Bytes,
) -> Answer<Json<Report>> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  let user = caller.writer()?.to_owned();
  if caller.side == Side::Sender {
    return Err(ApiError::sender_side("A report's state is moved by the maker's side alone."));
  }
  let id = path(given)?;
  let written: StateWrite = body(&bytes)?;
  let state = ReportState::read(&written.state)
    .ok_or_else(|| ApiError::bad_report("A state is `received`, `under-review`, `done` or `declined`."))?;
  let head = readable(app, &caller, &id)?;
  // The state it stands in already changes nothing, so no writer's name is asked for.
  if head.state != state {
    let name = writer_name(app, &caller, &user).await?;
    let by = Person { user, name };
    let moved = app
      .store
      .transact(|transaction| store::move_state(transaction, &head.id, state, &by, moment()))?;
    if moved {
      app.events.publish(
        CHANGED,
        ReportChanged {
          report: event_of(&head),
          state,
        },
      );
    }
  }
  Ok(Json(whole(app, &head.id, caller.side)?))
}

pub(super) async fn seen(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Path<String>, PathRejection>,
) -> Answer<StatusCode> {
  let app = &served.app;
  let caller = caller(app, &headers)?;
  caller.writer()?;
  let id = path(given)?;
  let head = readable(app, &caller, &id)?;
  app
    .store
    .read(|connection| store::mark_seen(connection, &head.id, caller.side, moment()))?;
  Ok(StatusCode::NO_CONTENT)
}
