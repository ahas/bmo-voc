//! A refusal or a failure, answered in the platform's error shape (`docs/api.md`, 「Codes」).

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::wire::WireError;

/// Every code the service answers, each with its status and what it means: the platform's common codes, then the
/// service's own.
pub const CODES: [(&str, u16, &str); 15] = [
  ("bad-request", 400, "the body, the path or the query is not the shape the route takes"),
  ("unauthorized", 401, "the token is missing or wrong"),
  ("forbidden", 403, "the token is right but the caller may not do this"),
  ("no-route", 404, "no route answers this method and path"),
  ("not-found", 404, "nothing the caller reads stands at what the path names"),
  ("conflict", 409, "the row's state does not take this act"),
  ("gone", 410, "a resume from before the events the service keeps"),
  ("failed", 500, "the service failed"),
  (
    "upstream-failed",
    502,
    "the account service failed a read a call needed; nothing of the call is written",
  ),
  ("not-ready", 503, "the service is not ready"),
  ("maker-side", 403, "a report sent by the maker"),
  ("sender-side", 403, "a state moved, or the companies read, by a sender"),
  (
    "bad-report",
    422,
    "a report's or a reply's field out of its bounds, an unknown kind or state, or a `report` part missing or not the JSON it names",
  ),
  ("too-many-files", 413, "more files than the settings' `[attachments] max_files`"),
  (
    "file-too-large",
    413,
    "a file past the settings' `[attachments] max_bytes`, or an empty one",
  ),
];

/// A refusal or a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
  pub status: u16,
  pub code: &'static str,
  pub error: String,
}

impl ApiError {
  fn of(code: &'static str, error: impl Into<String>) -> Self {
    let status = CODES
      .iter()
      .find(|(name, ..)| *name == code)
      .map(|(_, status, _)| *status)
      .unwrap_or(500);

    Self {
      status,
      code,
      error: error.into(),
    }
  }

  pub fn bad_request(error: impl Into<String>) -> Self {
    Self::of("bad-request", error)
  }

  pub fn unauthorized() -> Self {
    Self::of("unauthorized", "The bearer token is missing or wrong.")
  }

  pub fn forbidden(error: impl Into<String>) -> Self {
    Self::of("forbidden", error)
  }

  pub fn no_route() -> Self {
    Self::of("no-route", "No route answers this method and path.")
  }

  pub fn not_found(error: impl Into<String>) -> Self {
    Self::of("not-found", error)
  }

  pub fn gone(error: impl Into<String>) -> Self {
    Self::of("gone", error)
  }

  pub fn failed(error: impl Into<String>) -> Self {
    Self::of("failed", error)
  }

  pub fn upstream_failed(error: impl Into<String>) -> Self {
    Self::of("upstream-failed", error)
  }

  pub fn maker_side() -> Self {
    Self::of(
      "maker-side",
      "The maker's side sends no report; a report is a sender's word to the maker.",
    )
  }

  pub fn sender_side(error: impl Into<String>) -> Self {
    Self::of("sender-side", error)
  }

  pub fn bad_report(error: impl Into<String>) -> Self {
    Self::of("bad-report", error)
  }

  pub fn too_many_files(most: usize) -> Self {
    Self::of("too-many-files", format!("A report carries at most {most} files."))
  }

  pub fn file_too_large(error: impl Into<String>) -> Self {
    Self::of("file-too-large", error)
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = WireError {
      code: self.code.to_owned(),
      error: self.error,
    };
    let mut response = (status, axum::Json(body)).into_response();
    if status == StatusCode::SERVICE_UNAVAILABLE {
      response
        .headers_mut()
        .insert(header::RETRY_AFTER, header::HeaderValue::from_static("2"));
    }
    response
  }
}

impl From<rusqlite::Error> for ApiError {
  fn from(why: rusqlite::Error) -> Self {
    Self::failed(format!("the store: {why}"))
  }
}
