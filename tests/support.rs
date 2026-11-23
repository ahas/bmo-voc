//! What the tests share: a stand-in of the account service on a loopback port, recording the calls it takes; the
//! service started over it in this process on a SQLite file and an attachments' folder in a scratch folder; and
//! calls of its routes as a sender or the maker in the console, or the service token alone.

use std::collections::BTreeMap;
use std::path::{Path as FilePath, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use bmo_voc::settings::Settings;
use reqwest::Method;
use serde_json::{Value, json};
use tokio::sync::oneshot;

/// The stand-in's token.
pub const PEER_TOKEN: &str = "peer-token";
/// The service token.
pub const TOKEN: &str = "service-token";
/// The console's credential.
pub const CONSOLE: &str = "console-token";
/// How many files a report carries at most, and how large one may be, in the tests' settings.
pub const MAX_FILES: usize = 3;
pub const MAX_BYTES: usize = 64;

/// A sender in the company `c1`, named `Test Sender`.
pub const SENDER: &str = "tester";
/// A sender in the company `c2` who keeps no full name, so the account service names them by their login.
pub const OTHER_SENDER: &str = "tester-two";
/// A sender the account service places in no company.
pub const LONE_SENDER: &str = "tester-alone";
/// Two platform administrators, the maker's side.
pub const MAKER: &str = "qa-admin";
pub const OTHER_MAKER: &str = "qa-admin-two";

/// One person the stand-in holds: their full name and the company it places them in.
#[derive(Clone)]
pub struct Account {
  pub full_name: String,
  pub company: Option<(String, String)>,
}

/// One call the stand-in took: its path and the `Owner-Key` and `Owner-Role` it carried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
  pub path: String,
  pub owner_key: Option<String>,
  pub owner_role: Option<String>,
}

/// The account service as the stand-in holds it, the calls it took, and whether it fails every read.
#[derive(Clone)]
pub struct Stand {
  pub accounts: Arc<Mutex<BTreeMap<String, Account>>>,
  pub calls: Arc<Mutex<Vec<Call>>>,
  pub failing: Arc<AtomicBool>,
  pub address: String,
}

impl Stand {
  /// Every call taken so far.
  pub fn calls(&self) -> Vec<Call> {
    self.calls.lock().unwrap().clone()
  }

  /// Every read fails from now, or answers again.
  pub fn fail(&self, failing: bool) {
    self.failing.store(failing, Ordering::Relaxed);
  }

  /// The company the stand-in places `user` in, named anew.
  pub fn place(&self, user: &str, company: &str, name: &str) {
    self.accounts.lock().unwrap().get_mut(user).unwrap().company =
      Some((company.to_owned(), name.to_owned()));
  }
}

type Shared = State<Stand>;

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
  headers.get(name).and_then(|value| value.to_str().ok()).map(str::to_owned)
}

/// The refusal or the failure a call gets before its answer, where it gets one, the call recorded.
fn taken(stand: &Stand, headers: &HeaderMap, path: String) -> Option<Response> {
  stand.calls.lock().unwrap().push(Call {
    path,
    owner_key: header(headers, "owner-key"),
    owner_role: header(headers, "owner-role"),
  });
  if header(headers, "authorization") != Some(format!("Bearer {PEER_TOKEN}")) {
    return Some(
      (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "code": "unauthorized", "error": "wrong token" })),
      )
        .into_response(),
    );
  }
  if stand.failing.load(Ordering::Relaxed) {
    return Some(
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "code": "failed", "error": "the store is down" })),
      )
        .into_response(),
    );
  }
  None
}

fn missing() -> Response {
  (StatusCode::NOT_FOUND, Json(json!({ "code": "not-found", "error": "none" }))).into_response()
}

async fn read_user(State(stand): Shared, headers: HeaderMap, Path(user): Path<String>) -> Response {
  if let Some(refused) = taken(&stand, &headers, format!("/users/{user}")) {
    return refused;
  }
  match stand.accounts.lock().unwrap().get(&user) {
    Some(account) => Json(json!({
      "user": user,
      "name": user,
      "fullName": account.full_name,
      "administrator": user.starts_with(MAKER),
      "senders": [],
    }))
    .into_response(),
    None => missing(),
  }
}

async fn user_unit(State(stand): Shared, headers: HeaderMap, Path(user): Path<String>) -> Response {
  if let Some(refused) = taken(&stand, &headers, format!("/users/{user}/unit")) {
    return refused;
  }
  let accounts = stand.accounts.lock().unwrap();
  let Some((id, name)) = accounts.get(&user).and_then(|account| account.company.clone()) else {
    return missing();
  };
  let company = json!({ "id": id, "name": name });
  Json(json!({ "user": user, "unit": company, "company": company, "path": [company] })).into_response()
}

/// The stand-in, listening on a fresh loopback port.
pub async fn stand() -> Stand {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let account = |full_name: &str, company: Option<(&str, &str)>| Account {
    full_name: full_name.to_owned(),
    company: company.map(|(id, name)| (id.to_owned(), name.to_owned())),
  };
  let stand = Stand {
    accounts: Arc::new(Mutex::new(BTreeMap::from([
      (SENDER.to_owned(), account("Test Sender", Some(("c1", "Company One")))),
      (OTHER_SENDER.to_owned(), account("", Some(("c2", "Company Two")))),
      (LONE_SENDER.to_owned(), account("Lone Sender", None)),
      (MAKER.to_owned(), account("QA Admin", None)),
      (OTHER_MAKER.to_owned(), account("QA Admin Two", None)),
    ]))),
    calls: Arc::new(Mutex::new(Vec::new())),
    failing: Arc::new(AtomicBool::new(false)),
    address: format!("http://{}", listener.local_addr().unwrap()),
  };
  let router = Router::new()
    .route("/users/{user}", get(read_user))
    .route("/users/{user}/unit", get(user_unit))
    .with_state(stand.clone());
  tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
  stand
}

/// A fresh folder of its own under the system's scratch folder.
pub fn scratch() -> PathBuf {
  static COUNT: AtomicU64 = AtomicU64::new(0);
  let folder = std::env::temp_dir().join(format!(
    "bmo-voc-test-{}-{}",
    std::process::id(),
    COUNT.fetch_add(1, Ordering::Relaxed)
  ));
  let _ = std::fs::remove_dir_all(&folder);
  std::fs::create_dir_all(&folder).unwrap();
  folder
}

/// Settings over the stand-in, with the store, the attachments and the console bundle in `folder`.
pub fn settings(stand: &Stand, folder: &FilePath) -> Settings {
  let local = format!(
    "[server]\nlisten = \"127.0.0.1:0\"\ntoken = \"{TOKEN}\"\ndata_root = \"{root}\"\n\
     [console]\ntoken = \"{CONSOLE}\"\nfolder = \"{bundle}\"\n\
     [store]\npath = \"{store}\"\nfiles = \"{files}\"\n\
     [attachments]\nmax_files = {MAX_FILES}\nmax_bytes = {MAX_BYTES}\n\
     [accounts]\naddress = \"{peer}\"\ntoken = \"{PEER_TOKEN}\"\n",
    root = folder.join("data").display(),
    bundle = folder.join("bundle").display(),
    store = store_file(folder).display(),
    files = files_folder(folder).display(),
    peer = stand.address,
  );
  std::fs::write(folder.join("settings.local.toml"), local).unwrap();
  Settings::read(folder, Some(Default::default())).unwrap()
}

/// The SQLite file of the service started in `folder`.
pub fn store_file(folder: &FilePath) -> PathBuf {
  folder.join("data/voc/voc.db")
}

/// The attachments' folder of the service started in `folder`.
pub fn files_folder(folder: &FilePath) -> PathBuf {
  folder.join("data/voc/files")
}

/// One file of a report's form: its name, its type where the browser names one, and its bytes.
pub struct Upload<'a> {
  pub name: &'a str,
  pub kind: Option<&'a str>,
  pub bytes: &'a [u8],
}

pub fn file<'a>(name: &'a str, kind: &'a str, bytes: &'a [u8]) -> Upload<'a> {
  Upload {
    name,
    kind: Some(kind),
    bytes,
  }
}

/// A report's `report` part as a sender's screen writes it.
pub fn report(kind: &str, title: &str) -> Value {
  json!({
    "kind": kind,
    "title": title,
    "body": "The save button does nothing.\n\n- open the form\n- press save",
    "context": { "path": "/calendar/month", "consoleVersion": "0.1.0-rev.1", "browser": "Test Browser 1.0" },
  })
}

const BOUNDARY: &str = "voc-test-boundary";

/// A multipart form of a `report` part, where one is given as text, and a `files` part for each upload.
fn form(report: Option<&str>, files: &[Upload]) -> Vec<u8> {
  let mut form = Vec::new();
  if let Some(report) = report {
    form.extend(
      format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"report\"\r\n\
         Content-Type: application/json\r\n\r\n{report}\r\n"
      )
      .as_bytes(),
    );
  }
  for upload in files {
    form.extend(
      format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"{}\"\r\n",
        upload.name
      )
      .as_bytes(),
    );
    if let Some(kind) = upload.kind {
      form.extend(format!("Content-Type: {kind}\r\n").as_bytes());
    }
    form.extend(b"\r\n");
    form.extend(upload.bytes);
    form.extend(b"\r\n");
  }
  form.extend(format!("--{BOUNDARY}--\r\n").as_bytes());
  form
}

/// The service running over the stand-in.
pub struct Service {
  pub address: String,
  pub folder: PathBuf,
  pub stand: Stand,
  stop: Option<oneshot::Sender<()>>,
  ended: Option<tokio::task::JoinHandle<Result<(), String>>>,
}

impl Service {
  /// The service over a fresh stand-in in a fresh folder.
  pub async fn fresh() -> Self {
    Self::start(&stand().await, scratch()).await
  }

  /// The service started over `stand` with its store, attachments and console bundle in `folder`. Its listener is
  /// bound before it is served, so a call made at once waits in the listener's queue.
  pub async fn start(stand: &Stand, folder: PathBuf) -> Self {
    let settings = settings(stand, &folder);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = oneshot::channel();
    let ended = tokio::spawn(bmo_voc::serve_on(settings, listener, async move {
      let _ = stopped.await;
    }));
    Self {
      address,
      folder,
      stand: stand.clone(),
      stop: Some(stop),
      ended: Some(ended),
    }
  }

  /// Stops the service and waits for it to end.
  pub async fn stop(self) {
    let Self { stop, ended, .. } = self;
    if let Some(stop) = stop {
      let _ = stop.send(());
    }
    if let Some(ended) = ended {
      ended.await.unwrap().unwrap();
    }
  }

  /// A request with the given bearer and headers, not yet sent.
  pub fn request(
    &self,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    headers: &[(&str, &str)],
  ) -> reqwest::RequestBuilder {
    let mut request = reqwest::Client::builder()
      .no_proxy()
      .build()
      .unwrap()
      .request(method, format!("{}{path}", self.address));
    if let Some(bearer) = bearer {
      request = request.bearer_auth(bearer);
    }
    for (name, value) in headers {
      request = request.header(*name, *value);
    }
    request
  }

  /// A request sent: its status and its body read as JSON, null where it is none.
  pub async fn answer(request: reqwest::RequestBuilder) -> (u16, Value) {
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
  }

  /// One call with the given bearer and headers.
  pub async fn ask(
    &self,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    headers: &[(&str, &str)],
    body: Option<Value>,
  ) -> (u16, Value) {
    let mut request = self.request(method, path, bearer, headers);
    if let Some(body) = body {
      request = request.json(&body);
    }
    Self::answer(request).await
  }

  /// A sender's call through the console.
  pub async fn sender(&self, user: &str, method: Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self.ask(method, path, Some(CONSOLE), &console(user, "member"), body).await
  }

  /// A platform administrator's call through the console.
  pub async fn maker(&self, user: &str, method: Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self.ask(method, path, Some(CONSOLE), &console(user, "administrator"), body).await
  }

  /// A call with the service token alone.
  pub async fn everything(&self, method: Method, path: &str, body: Option<Value>) -> (u16, Value) {
    self.ask(method, path, Some(TOKEN), &[], body).await
  }

  /// A report's form sent through the console as `user` in `role`, its `report` part given as text.
  pub async fn send_form(
    &self,
    user: &str,
    role: &str,
    report: Option<&str>,
    files: &[Upload<'_>],
  ) -> (u16, Value) {
    self.post_form(Some(CONSOLE), &console(user, role), report, files).await
  }

  /// A report's form sent with the given bearer and headers, its `report` part given as text.
  pub async fn post_form(
    &self,
    bearer: Option<&str>,
    headers: &[(&str, &str)],
    report: Option<&str>,
    files: &[Upload<'_>],
  ) -> (u16, Value) {
    let request = self
      .request(Method::POST, "/reports", bearer, headers)
      .header("Content-Type", format!("multipart/form-data; boundary={BOUNDARY}"))
      .body(form(report, files));
    Self::answer(request).await
  }

  /// A report sent by a sender with its files.
  pub async fn send(&self, user: &str, report: &Value, files: &[Upload<'_>]) -> (u16, Value) {
    self.send_form(user, "member", Some(&report.to_string()), files).await
  }

  /// A report sent by a sender with no files, its id.
  pub async fn sent(&self, user: &str, kind: &str, title: &str) -> String {
    let (status, sent) = self.send(user, &report(kind, title), &[]).await;
    assert_eq!(status, 201, "{sent}");
    sent["id"].as_str().unwrap().to_owned()
  }

  /// The files the attachments' folder holds.
  pub fn kept_files(&self) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(files_folder(&self.folder))
      .unwrap()
      .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
      .collect();
    names.sort();
    names
  }
}

/// The headers a console call for `user` in `role` carries.
pub fn console<'a>(user: &'a str, role: &'a str) -> [(&'a str, &'a str); 2] {
  [("Owner-Key", user), ("Owner-Role", role)]
}

/// The `code` of a refusal, with its status.
pub fn refusal(answer: &(u16, Value)) -> (u16, &str) {
  (answer.0, answer.1["code"].as_str().unwrap_or_default())
}
