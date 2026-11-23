//! The OpenAPI document: the committed one is what the code generates, and it states exactly the routes the service
//! answers and the events docs/api.md lists.

use std::collections::BTreeSet;

use bmo_voc::openapi::{openapi, openapi_json};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The routes the service answers, by method and path.
const ROUTES: [&str; 14] = [
  "get /health",
  "get /me",
  "get /reports",
  "post /reports",
  "get /reports/{reportId}",
  "get /reports/{reportId}/attachments/{attachmentId}",
  "post /reports/{reportId}/replies",
  "put /reports/{reportId}/state",
  "put /reports/{reportId}/seen",
  "get /companies",
  "get /unread",
  "get /events",
  "get /console-manifest",
  "get /console/{file}",
];

#[test]
fn the_committed_document_is_the_generated_one() {
  let committed = std::fs::read_to_string(format!("{ROOT}/docs/openapi.json")).unwrap_or_default();
  assert!(
    committed == openapi_json(),
    "docs/openapi.json differs from what the code generates; write it again with `bmo-voc openapi`"
  );
}

#[test]
fn the_document_states_every_route_and_nothing_else() {
  let document = openapi();
  let stated: BTreeSet<String> = document["paths"]
    .as_object()
    .unwrap()
    .iter()
    .flat_map(|(path, methods)| {
      methods.as_object().unwrap().keys().map(move |method| format!("{method} {path}"))
    })
    .collect();
  assert_eq!(stated, ROUTES.iter().map(|route| (*route).to_owned()).collect());
  assert_eq!(document["info"]["version"], env!("CARGO_PKG_VERSION"));
  assert_eq!(
    document["x-events"],
    serde_json::json!(["report.received", "report.answered", "report.followed", "report.changed"])
  );
}
