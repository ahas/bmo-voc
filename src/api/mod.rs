//! The routes (`docs/api.md`): who calls, the reports, their attachments, replies, states and read marks, the
//! companies, the unread count, the event stream, the console face and the health.

mod reports;
mod stream;

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::DefaultBodyLimit;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Json;
use axum::routing::{get, post, put};
use bmo_console_face::ConsoleFace;
use serde::de::DeserializeOwned;

use crate::access::{caller, credential};
use crate::app::App;
use crate::error::ApiError;
use crate::store::{self, Filter};
use crate::wire::{Company, Health, Limits, Me, Page, Side, Unread};

type Answer<T> = Result<T, ApiError>;

/// The most rows a page holds where none is asked, and the most it ever holds.
const PAGE: i64 = 50;
const MOST: i64 = 200;

/// The service as the routes see it: its state and its console face, where one stands.
#[derive(Clone)]
pub struct Served {
  pub app: Arc<App>,
  pub face: Option<ConsoleFace>,
}

/// Every route, the unknown ones answered `no-route`: the health, the screens' ones under the console face's
/// guard, and the face's own, where a face stands (`docs/api.md`, 「Console」).
pub fn router(served: Served) -> Router {
  let screens = screens();
  let mut routes = Router::new().route("/health", get(health));
  match &served.face {
    Some(face) => routes = routes.merge(face.guard_screens(screens)).merge(face.router()),
    None => routes = routes.merge(screens),
  }
  routes
    .fallback(|| async { ApiError::no_route() })
    .method_not_allowed_fallback(|| async { ApiError::no_route() })
    .with_state(served)
}

/// The routes that act for a reader, which the screens call. A report's form is bounded by its own part's and
/// files' bounds as it is read, so the server's default bound on a body does not stand over it.
fn screens() -> Router<Served> {
  Router::new()
    .route("/me", get(me))
    .route("/reports", get(reports::list).post(reports::send).layer(DefaultBodyLimit::disable()))
    .route("/reports/{reportId}", get(reports::read))
    .route("/reports/{reportId}/attachments/{attachmentId}", get(reports::attachment))
    .route("/reports/{reportId}/replies", post(reports::reply))
    .route("/reports/{reportId}/state", put(reports::move_state))
    .route("/reports/{reportId}/seen", put(reports::seen))
    .route("/companies", get(companies))
    .route("/unread", get(unread))
    .route("/events", get(stream::events))
}

pub(crate) fn query<T: DeserializeOwned>(given: Result<Query<T>, QueryRejection>) -> Answer<T> {
  given.map(|Query(query)| query).map_err(|why| ApiError::bad_request(why.body_text()))
}

pub(crate) fn path<T: DeserializeOwned>(given: Result<Path<T>, PathRejection>) -> Answer<T> {
  given.map(|Path(part)| part).map_err(|why| ApiError::bad_request(why.body_text()))
}

pub(crate) fn body<T: DeserializeOwned>(bytes: &Bytes) -> Answer<T> {
  serde_json::from_slice(bytes)
    .map_err(|why| ApiError::bad_request(format!("The body does not read: {why}")))
}

/// A page's bounds read: the rows read so far, and at most how many, 50 where none is asked, never past 200.
pub(crate) fn bounds(after: Option<i64>, limit: Option<i64>) -> Answer<(i64, i64)> {
  let after = after.unwrap_or(0);
  let limit = limit.unwrap_or(PAGE);
  if after < 0 || !(1..=MOST).contains(&limit) {
    return Err(ApiError::bad_request("`after` is 0 or more and `limit` 1 to 200."));
  }
  Ok((after, limit))
}

// ---------------------------------------------------------------------------------------------------------
// Health

async fn health(State(served): State<Served>, headers: HeaderMap) -> Answer<Json<Health>> {
  credential(&served.app, &headers)?;
  let files = &served.app.settings.store.files;
  let reason =
    (!files.is_dir()).then(|| format!("the attachments' folder {} does not stand", files.display()));
  Ok(Json(Health {
    ready: reason.is_none(),
    reason,
    version: crate::VERSION.to_owned(),
  }))
}

// ---------------------------------------------------------------------------------------------------------
// The reader, the companies and the unread count

async fn me(State(served): State<Served>, headers: HeaderMap) -> Answer<Json<Me>> {
  let caller = caller(&served.app, &headers)?;
  let bounds = &served.app.settings.attachments;
  Ok(Json(Me {
    user: caller.user,
    side: caller.side,
    limits: Limits {
      max_files: bounds.max_files,
      max_bytes: bounds.max_bytes,
    },
  }))
}

async fn companies(State(served): State<Served>, headers: HeaderMap) -> Answer<Json<Page<Company>>> {
  let caller = caller(&served.app, &headers)?;
  if caller.side == Side::Sender {
    return Err(ApiError::sender_side("The companies are read by the maker's side alone."));
  }
  let items = served.app.store.read(store::companies)?;
  Ok(Json(Page { items, more: false }))
}

async fn unread(State(served): State<Served>, headers: HeaderMap) -> Answer<Json<Unread>> {
  let caller = caller(&served.app, &headers)?;
  let filter = Filter {
    sender: if caller.reads_all() { None } else { caller.user.clone() },
    ..Filter::default()
  };
  let count = served
    .app
    .store
    .read(|connection| store::unread_count(connection, caller.side, &filter))?;
  Ok(Json(Unread { count }))
}
