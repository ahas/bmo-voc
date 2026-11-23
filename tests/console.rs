//! The console's reach: the service's own check of a caller (401, 403) and the side it reads them on, the health
//! behind the token, the manifest its face serves read through bmo-ui's own reading of the contract on the release
//! the client vendors, carrying the voc declaration, a 404 where no bundle is built or its layer is missing, and how
//! a console probe reaches the service. How the face guards its manifest, words, module and stylesheet is
//! bmo-console-face's tests' and the deployment's console probe's, and is not checked again here.

use std::path::{Path, PathBuf};

use bmo_console_face::read_manifest;
use bmo_voc::console::{DECLARATION, reach};
use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{CONSOLE, Service, TOKEN, scratch, settings, stand};

#[tokio::test]
async fn every_route_is_guarded_and_the_role_names_the_side() {
  let service = Service::fresh().await;
  let get = async |bearer: Option<&str>, headers: &[(&str, &str)]| {
    let (status, answer) = service.ask(Method::GET, "/me", bearer, headers, None).await;
    (status, answer["code"].as_str().unwrap_or("").to_owned())
  };

  assert_eq!(get(None, &[]).await, (401, "unauthorized".to_owned()));
  assert_eq!(get(Some("wrong"), &[]).await, (401, "unauthorized".to_owned()));
  assert_eq!(get(Some(CONSOLE), &[("Owner-Role", "member")]).await, (403, "forbidden".to_owned()));
  assert_eq!(get(Some(CONSOLE), &[("Owner-Key", "tester")]).await, (403, "forbidden".to_owned()));
  assert_eq!(
    get(Some(CONSOLE), &[("Owner-Key", "tester"), ("Owner-Role", "guest")]).await,
    (403, "forbidden".to_owned())
  );
  let long = "k".repeat(201);
  assert_eq!(get(Some(CONSOLE), &[("Owner-Key", &long), ("Owner-Role", "member")]).await.0, 403);

  let me = async |role: &str| {
    service
      .ask(
        Method::GET,
        "/me",
        Some(CONSOLE),
        &[("Owner-Key", "tester"), ("Owner-Role", role)],
        None,
      )
      .await
  };
  let sides = async |role: &str| {
    let (status, answer) = me(role).await;
    (status, answer["user"].clone(), answer["side"].clone())
  };
  assert_eq!(sides("member").await, (200, json!("tester"), json!("sender")));
  assert_eq!(sides("administrator").await, (200, json!("tester"), json!("maker")));

  let (status, refusal) = service.sender("tester", Method::GET, "/nowhere/at/all", None).await;
  assert_eq!((status, refusal["code"].as_str()), (404, Some("no-route")));
  assert_eq!(service.ask(Method::GET, "/health", None, &[], None).await.0, 401);
  let (status, health) = service.everything(Method::GET, "/health", None).await;
  assert_eq!(status, 200);
  assert_eq!(health, json!({ "ready": true, "reason": null, "version": env!("CARGO_PKG_VERSION") }));
  service.stop().await;
}

#[tokio::test]
async fn no_face_is_answered_where_no_bundle_is_built_or_its_layer_is_missing() {
  let service = Service::fresh().await;
  let manifest = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!((manifest.0, manifest.1["code"].as_str()), (404, Some("no-route")));
  service.stop().await;

  let folder = scratch();
  build(&folder, None);
  let service = Service::start(&stand().await, folder).await;
  assert_eq!(service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await.0, 404);
  service.stop().await;
}

#[tokio::test]
async fn the_served_manifest_is_read_by_the_contract_on_the_vendored_release() {
  let layer = installed_layer();
  let folder = scratch();
  build(&folder, Some(&layer));
  let service = Service::start(&stand().await, folder).await;

  let (status, served) = service.ask(Method::GET, "/console-manifest", Some(CONSOLE), &[], None).await;
  assert_eq!(status, 200, "{served}");
  let read = read_manifest(&served, &vendored_release()).unwrap_or_else(|why| panic!("{why}"));
  assert_eq!(read["layer"], layer.as_str());
  let declared: Value = serde_json::from_str(DECLARATION).unwrap();
  for (field, value) in declared.as_object().unwrap() {
    assert_eq!(&read[field], value, "{field}");
  }
  service.stop().await;
}

#[tokio::test]
async fn a_console_probe_reaches_the_service_by_its_settings() {
  let folder = scratch();
  let settings = settings(&stand().await, &folder);
  let reached = reach(&settings);
  assert_eq!(reached["address"], settings.base_address());
  assert_eq!(reached["consoleToken"], CONSOLE);
  assert_eq!(reached["token"], TOKEN);
}

/// The module and the stylesheet a test bundle holds.
const MODULE: &str = "export default {};\n";
const STYLESHEET: &str = ".voc {}\n";

/// A built bundle in the folder the service reads, its layer file written where one is given.
fn build(folder: &Path, layer: Option<&str>) {
  let bundle = folder.join("bundle");
  std::fs::create_dir_all(&bundle).unwrap();
  std::fs::write(bundle.join("bundle.js"), MODULE).unwrap();
  std::fs::write(bundle.join("bundle.css"), STYLESHEET).unwrap();
  let words = json!({ "en": { "voc": { "title": "Feedback" } }, "ko": { "voc": { "title": "Feedback" } } });
  std::fs::write(bundle.join("words.json"), words.to_string()).unwrap();
  if let Some(layer) = layer {
    std::fs::write(bundle.join("layer.json"), json!({ "layer": layer }).to_string()).unwrap();
  }
}

/// The client's folder.
const CLIENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/client");

/// The layer the bundle is built on: the release of bmo-ui the client vendors, as its file is named
/// (`bmo-ui-<version>.tgz`), which is what the bundle build writes into its layer file. It is read from the
/// file's name, since the suite runs where the client's packages are not installed.
fn installed_layer() -> String {
  let release = vendored_release();
  let name = release.file_name().and_then(|name| name.to_str()).unwrap_or_default();
  name
    .strip_prefix("bmo-ui-")
    .and_then(|rest| rest.strip_suffix(".tgz"))
    .expect("the vendored release is named bmo-ui-<version>.tgz")
    .to_owned()
}

/// The bmo-ui release file the client vendors, as its package manifest names it.
fn vendored_release() -> PathBuf {
  let manifest: Value =
    serde_json::from_slice(&std::fs::read(Path::new(CLIENT).join("package.json")).unwrap()).unwrap();
  let taken = manifest["devDependencies"]["bmo-ui"].as_str().unwrap();
  Path::new(CLIENT).join(taken.strip_prefix("file:").expect("bmo-ui is taken as a file"))
}
