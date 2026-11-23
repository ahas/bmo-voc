//! The unit the voc unit calls (`docs/api.md`, 「Calls the voc unit makes」): the account service, read for a
//! writer's name and a sender's company, found afresh for every call from the settings or the data root's
//! `env/web.toml`. A call carries the caller's `Owner-Key` and `Owner-Role` as the console handed them and waits
//! for its answer; nothing read is kept but what a report keeps of it.

use std::path::Path;

use serde_json::Value;

use crate::access::Caller;
use crate::settings::Settings;
use crate::wire::Company;

/// The account service as a call reaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
  pub address: String,
  pub token: String,
}

/// Why a call came to nothing.
#[derive(Clone, Debug)]
pub enum CallFault {
  /// The account service answered a failure: its status and the body's sentence.
  Answered { status: u16, error: String },
  /// The account service could not be found or its answer not read.
  Broken(String),
}

impl std::fmt::Display for CallFault {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Answered { status, error } => write!(formatter, "the account service: {error} ({status})"),
      Self::Broken(why) => formatter.write_str(why),
    }
  }
}

/// An address under a listen or a setting, `127.0.0.1:13127` read as `http://127.0.0.1:13127`.
pub fn address(listen: &str) -> String {
  let listen = listen.trim().trim_end_matches('/');
  if listen.contains("://") {
    listen.to_owned()
  } else if let Some(port) = listen.strip_prefix("0.0.0.0:") {
    format!("http://127.0.0.1:{port}")
  } else {
    format!("http://{listen}")
  }
}

fn toml_file(path: &Path) -> toml::Table {
  std::fs::read_to_string(path)
    .ok()
    .and_then(|text| text.parse().ok())
    .unwrap_or_default()
}

/// The account service: the settings' where they name its address, else `[server] listen` and `token` of the
/// data root's `env/web.toml`, the settings' token standing over the file's where it names one.
pub fn accounts(settings: &Settings) -> Result<Reach, CallFault> {
  let named = &settings.accounts;
  let file = toml_file(&settings.data_root().join("env/web.toml"));
  let server = file.get("server").and_then(|server| server.as_table());
  let text = |key: &str| {
    server
      .and_then(|server| server.get(key))
      .and_then(|value| value.as_str())
      .map(str::to_owned)
  };
  let given = |value: &str| (!value.is_empty()).then(|| value.to_owned());
  let address = given(&named.address)
    .map(|named| self::address(&named))
    .or_else(|| text("listen").map(|listen| self::address(&listen)));
  let token = given(&named.token).or_else(|| text("token"));
  match (address, token) {
    (Some(address), Some(token)) => Ok(Reach { address, token }),
    _ => Err(CallFault::Broken("the account service's address or token is set nowhere".to_owned())),
  }
}

/// The one HTTP client every call shares, never through a proxy and never cut by a clock.
pub fn http() -> reqwest::Client {
  reqwest::Client::builder().no_proxy().build().unwrap_or_default()
}

fn misshapen() -> CallFault {
  CallFault::Broken(
    "the account service answered a body that is not the shape its document states".to_owned(),
  )
}

/// The account service's address with path segments each encoded as one segment.
fn url(reach: &Reach, segments: &[&str]) -> Result<reqwest::Url, CallFault> {
  let mut url = reqwest::Url::parse(&reach.address)
    .map_err(|why| CallFault::Broken(format!("{} is no address: {why}", reach.address)))?;
  url
    .path_segments_mut()
    .map_err(|()| CallFault::Broken(format!("{} takes no path", reach.address)))?
    .pop_if_empty()
    .extend(segments);
  Ok(url)
}

/// A read of `segments` made for `caller`, carrying the account service's token and the caller's `Owner-Key` and
/// `Owner-Role`: the body of a success, none for a 404, else the fault it came to.
async fn read(
  http: &reqwest::Client,
  reach: &Reach,
  caller: &Caller,
  segments: &[&str],
) -> Result<Option<Value>, CallFault> {
  let mut request = http.get(url(reach, segments)?).bearer_auth(&reach.token);
  if let Some(user) = &caller.user {
    request = request.header("Owner-Key", user).header("Owner-Role", caller.role());
  }
  let response = request
    .send()
    .await
    .map_err(|why| CallFault::Broken(format!("the account service could not be reached: {why}")))?;
  let status = response.status();
  if status.is_success() {
    return response
      .json()
      .await
      .map(Some)
      .map_err(|why| CallFault::Broken(format!("the account service's answer does not read: {why}")));
  }
  if status == reqwest::StatusCode::NOT_FOUND {
    return Ok(None);
  }
  let body: Value = response.json().await.unwrap_or(Value::Null);
  Err(CallFault::Answered {
    status: status.as_u16(),
    error: body["error"]
      .as_str()
      .map(str::to_owned)
      .unwrap_or_else(|| format!("it answered {status}")),
  })
}

/// An id as the account service writes it, a number or a string, as a string.
fn id_text(value: &Value) -> Option<String> {
  match value {
    Value::String(text) if !text.is_empty() => Some(text.clone()),
    Value::Number(number) => Some(number.to_string()),
    _ => None,
  }
}

/// What the account service calls `user`: their full name, else their login where no full name stands. A user it
/// does not hold is a fault like any other failure, since a writer always stands.
pub async fn name(
  http: &reqwest::Client,
  reach: &Reach,
  caller: &Caller,
  user: &str,
) -> Result<String, CallFault> {
  let answer = read(http, reach, caller, &["users", user]).await?.ok_or(CallFault::Answered {
    status: 404,
    error: format!("no user {user} stands"),
  })?;
  [&answer["fullName"], &answer["name"]]
    .into_iter()
    .filter_map(Value::as_str)
    .map(str::trim)
    .find(|name| !name.is_empty())
    .map(str::to_owned)
    .ok_or_else(misshapen)
}

/// The company the account service places `user` in; none where it answers 404 or places them in none.
pub async fn company(
  http: &reqwest::Client,
  reach: &Reach,
  caller: &Caller,
  user: &str,
) -> Result<Option<Company>, CallFault> {
  let Some(answer) = read(http, reach, caller, &["users", user, "unit"]).await? else {
    return Ok(None);
  };
  let company = &answer["company"];
  if company.is_null() {
    return Ok(None);
  }
  let id = id_text(&company["id"]).ok_or_else(misshapen)?;
  let name = company["name"].as_str().ok_or_else(misshapen)?.to_owned();
  Ok(Some(Company { id, name }))
}
