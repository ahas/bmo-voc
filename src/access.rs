//! Who calls and on which side (`docs/api.md`, 「Who calls」; `docs/design.md`, 「Who reads」): the console for a
//! signed-in person, the maker for `administrator` and a sender for `member`, and the service token alone, a
//! reader of everything that writes nothing.

use axum::http::{HeaderMap, header};

use crate::app::App;
use crate::error::ApiError;
use crate::peers::CallFault;
use crate::wire::Side;

type Answer<T> = Result<T, ApiError>;

/// The longest `Owner-Key` that names a user id.
const LONGEST_KEY: usize = 200;

/// The account service's failure as the request that needed it is answered.
pub fn upstream(fault: CallFault) -> ApiError {
  ApiError::upstream_failed(fault.to_string())
}

/// One caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
  /// The person it acts for, the account service's user id; none for a reader of everything.
  pub user: Option<String>,
  pub side: Side,
}

impl Caller {
  /// The person a write is made as; a reader of everything is refused, since it writes nothing.
  pub fn writer(&self) -> Answer<&str> {
    self
      .user
      .as_deref()
      .ok_or_else(|| ApiError::forbidden("A reader of everything writes nothing."))
  }

  /// The `Owner-Role` the console handed for the caller's side.
  pub fn role(&self) -> &'static str {
    match self.side {
      Side::Maker => "administrator",
      Side::Sender | Side::Reader => "member",
    }
  }

  /// Whether the caller reads every report: the maker and a reader of everything.
  pub fn reads_all(&self) -> bool {
    self.side != Side::Sender
  }

  /// Whether the caller reads a report `sender` sent.
  pub fn reads(&self, sender: &str) -> bool {
    self.reads_all() || self.user.as_deref() == Some(sender)
  }
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
  headers
    .get(header::AUTHORIZATION)?
    .to_str()
    .ok()?
    .strip_prefix("Bearer ")
    .map(str::trim)
}

/// A header's text, none where it is absent or blank.
pub fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
  headers.get(name)?.to_str().ok().map(str::trim).filter(|value| !value.is_empty())
}

/// Whether the credential is the service's (true), the console's (false), or neither (401).
pub fn credential(app: &App, headers: &HeaderMap) -> Answer<bool> {
  let given = bearer(headers).ok_or_else(ApiError::unauthorized)?;
  let server = &app.settings.server.token;
  let console = &app.settings.console.token;
  if !server.is_empty() && given == server {
    Ok(true)
  } else if !console.is_empty() && given == console {
    Ok(false)
  } else {
    Err(ApiError::unauthorized())
  }
}

/// Who calls.
pub fn caller(app: &App, headers: &HeaderMap) -> Answer<Caller> {
  if credential(app, headers)? {
    return Ok(Caller {
      user: None,
      side: Side::Reader,
    });
  }
  let key = header_text(headers, "owner-key")
    .ok_or_else(|| ApiError::forbidden("The call carries no Owner-Key."))?;
  if key.chars().count() > LONGEST_KEY {
    return Err(ApiError::forbidden(format!(
      "The Owner-Key is longer than {LONGEST_KEY} characters, so it names no user."
    )));
  }
  let side = match header_text(headers, "owner-role") {
    Some("administrator") => Side::Maker,
    Some("member") => Side::Sender,
    _ => return Err(ApiError::forbidden("The call carries no Owner-Role of administrator or member.")),
  };
  Ok(Caller {
    user: Some(key.to_owned()),
    side,
  })
}
