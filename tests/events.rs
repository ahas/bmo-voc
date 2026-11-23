//! The events read as a page: every one to the maker and a reader of everything, a sender's own reports' alone to
//! a sender, each kind with its data, narrowed by kind and resumed after a number, and a resume from before the
//! service's start gone.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{CONSOLE, MAKER, OTHER_SENDER, SENDER, Service, TOKEN, console, refusal};

/// A page of events as a caller reads it.
async fn page(service: &Service, query: &str, bearer: &str, headers: &[(&str, &str)]) -> (u16, Value) {
  let mut headers = headers.to_vec();
  headers.push(("accept", "application/json"));
  service.ask(Method::GET, &format!("/events{query}"), Some(bearer), &headers, None).await
}

/// The kinds a caller reads, in order.
async fn kinds(service: &Service, bearer: &str, headers: &[(&str, &str)]) -> Vec<String> {
  let (status, read) = page(service, "", bearer, headers).await;
  assert_eq!(status, 200, "{read}");
  read["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|event| event["kind"].as_str().unwrap().to_owned())
    .collect()
}

#[tokio::test]
async fn an_event_reaches_the_maker_always_and_a_sender_for_their_own_reports_alone() {
  let service = Service::fresh().await;
  let own = service.sent(SENDER, "bug", "Own").await;
  let other = service.sent(OTHER_SENDER, "suggestion", "Other").await;
  let reply = |id: &str| format!("/reports/{id}/replies");
  service
    .maker(MAKER, Method::POST, &reply(&own), Some(json!({ "body": "Answer" })))
    .await;
  service
    .sender(SENDER, Method::POST, &reply(&own), Some(json!({ "body": "Follow" })))
    .await;
  service
    .maker(MAKER, Method::PUT, &format!("/reports/{other}/state"), Some(json!({ "state": "done" })))
    .await;

  let every = [
    "report.received",
    "report.received",
    "report.answered",
    "report.followed",
    "report.changed",
  ];
  assert_eq!(kinds(&service, CONSOLE, &console(MAKER, "administrator")).await, every);
  assert_eq!(kinds(&service, TOKEN, &[]).await, every);
  assert_eq!(
    kinds(&service, CONSOLE, &console(SENDER, "member")).await,
    ["report.received", "report.answered", "report.followed"]
  );
  assert_eq!(
    kinds(&service, CONSOLE, &console(OTHER_SENDER, "member")).await,
    ["report.received", "report.changed"]
  );

  let (_, changed) = page(&service, "?kinds=report.changed", TOKEN, &[]).await;
  assert_eq!(
    changed["items"][0]["data"],
    json!({ "reportId": other, "sender": OTHER_SENDER, "title": "Other", "kind": "suggestion", "state": "done" })
  );
  let (_, received) = page(&service, "?kinds=report.received,report.answered&limit=1", TOKEN, &[]).await;
  assert_eq!(
    received["items"][0]["data"],
    json!({ "reportId": own, "sender": SENDER, "title": "Own", "kind": "bug" })
  );
  assert_eq!(received["more"], true);
  let first = received["items"][0]["id"].as_i64().unwrap();
  let (_, resumed) = page(&service, &format!("?after={first}"), TOKEN, &[]).await;
  assert_eq!(resumed["items"].as_array().unwrap().len(), 4);
  service.stop().await;
}

#[tokio::test]
async fn a_resume_from_before_the_start_is_gone_on_the_page_and_the_stream() {
  let service = Service::fresh().await;
  assert_eq!(refusal(&page(&service, "?after=0", TOKEN, &[]).await), (410, "gone"));
  let (status, _) = service.ask(Method::GET, "/events", Some(TOKEN), &[("Last-Event-ID", "0")], None).await;
  assert_eq!(status, 410);
  service.stop().await;
}
