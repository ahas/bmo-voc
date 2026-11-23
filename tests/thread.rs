//! A report's thread: the maker's state moves with their history, the replies of both sides, and what each side
//! has not read, by its one mark per report.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{MAKER, OTHER_MAKER, OTHER_SENDER, SENDER, Service, refusal};

#[tokio::test]
async fn the_maker_moves_a_state_each_move_kept_and_the_standing_state_changes_nothing() {
  let service = Service::fresh().await;
  let id = service.sent(SENDER, "bug", "Moves").await;
  let state = format!("/reports/{id}/state");
  let moved = |to: &str| json!({ "state": to });

  let (status, reviewing) = service.maker(MAKER, Method::PUT, &state, Some(moved("under-review"))).await;
  assert_eq!(status, 200, "{reviewing}");
  let (_, done) = service.maker(OTHER_MAKER, Method::PUT, &state, Some(moved("done"))).await;
  let asked = service.stand.calls().len();
  let (status, again) = service.maker(MAKER, Method::PUT, &state, Some(moved("done"))).await;

  assert_eq!(status, 200);
  assert_eq!(again, done, "the state it stands in changes nothing");
  assert_eq!(service.stand.calls().len(), asked, "nor asks the account service for a name");
  let history: Vec<(&str, &Value)> = done["states"]
    .as_array()
    .unwrap()
    .iter()
    .map(|entry| (entry["state"].as_str().unwrap(), &entry["by"]))
    .collect();
  assert_eq!(
    history,
    [
      ("received", &json!({ "user": SENDER, "name": "Test Sender", "side": "sender" })),
      ("under-review", &json!({ "user": MAKER, "name": "QA Admin", "side": "maker" })),
      ("done", &json!({ "user": OTHER_MAKER, "name": "QA Admin Two", "side": "maker" })),
    ]
  );
  assert_eq!(done["state"], "done");
  assert_eq!(done["activityAt"], done["states"][2]["at"]);

  assert_eq!(
    refusal(&service.sender(SENDER, Method::PUT, &state, Some(moved("declined"))).await),
    (403, "sender-side")
  );
  assert_eq!(
    refusal(&service.maker(MAKER, Method::PUT, &state, Some(moved("closed"))).await),
    (422, "bad-report")
  );
  assert_eq!(
    refusal(
      &service
        .maker(MAKER, Method::PUT, "/reports/no-such-report/state", Some(moved("done")))
        .await
    ),
    (404, "not-found")
  );
  let (status, _) = service
    .ask(
      Method::PUT,
      &state,
      Some(crate::support::CONSOLE),
      &crate::support::console(MAKER, "administrator"),
      None,
    )
    .await;
  assert_eq!(status, 400, "a body that does not read is no state");
  service.stop().await;
}

#[tokio::test]
async fn both_sides_reply_on_one_thread_at_any_state_and_a_reply_moves_no_state() {
  let service = Service::fresh().await;
  let id = service.sent(SENDER, "suggestion", "Thread").await;
  let replies = format!("/reports/{id}/replies");
  service
    .maker(
      MAKER,
      Method::PUT,
      &format!("/reports/{id}/state"),
      Some(json!({ "state": "declined" })),
    )
    .await;

  let (status, answer) =
    service.maker(MAKER, Method::POST, &replies, Some(json!({ "body": "Not now." }))).await;
  assert_eq!(status, 201, "{answer}");
  assert_eq!(
    (&answer["side"], &answer["by"], &answer["body"]),
    (&json!("maker"), &json!({ "user": MAKER, "name": "QA Admin" }), &json!("Not now."))
  );
  let (status, follow) = service
    .sender(SENDER, Method::POST, &replies, Some(json!({ "body": "**Please** reconsider." })))
    .await;
  assert_eq!(status, 201, "{follow}");
  assert_eq!(follow["side"], "sender");

  let (_, read) = service.sender(SENDER, Method::GET, &format!("/reports/{id}"), None).await;
  assert_eq!(read["replies"], json!([answer, follow]));
  assert_eq!(read["state"], "declined");
  assert_eq!(read["states"].as_array().unwrap().len(), 2, "a reply keeps no state entry");
  assert_eq!(read["activityAt"], follow["at"]);

  for body in [json!({ "body": "  " }), json!({ "body": "b".repeat(20_001) })] {
    assert_eq!(
      refusal(&service.sender(SENDER, Method::POST, &replies, Some(body)).await),
      (422, "bad-report")
    );
  }
  assert_eq!(
    refusal(&service.sender(SENDER, Method::POST, &replies, Some(json!({ "text": "x" }))).await),
    (400, "bad-request")
  );
  service.stop().await;
}

/// The unread count a side reads.
async fn count(service: &Service, user: &str, maker: bool) -> i64 {
  let (status, unread) = if maker {
    service.maker(user, Method::GET, "/unread", None).await
  } else {
    service.sender(user, Method::GET, "/unread", None).await
  };
  assert_eq!(status, 200, "{unread}");
  unread["count"].as_i64().unwrap()
}

/// Whether a report reads unread to a sender, in their list and whole.
async fn unread_to_sender(service: &Service, id: &str) -> bool {
  let (_, page) = service.sender(SENDER, Method::GET, "/reports", None).await;
  let (_, whole) = service.sender(SENDER, Method::GET, &format!("/reports/{id}"), None).await;
  let listed =
    page["items"].as_array().unwrap().iter().find(|item| item["id"] == id).unwrap()["unread"].clone();
  assert_eq!(listed, whole["unread"]);
  listed.as_bool().unwrap()
}

#[tokio::test]
async fn each_side_reads_as_unread_what_the_other_side_did_since_its_mark() {
  let service = Service::fresh().await;
  let id = service.sent(SENDER, "bug", "Unread").await;
  service.sent(OTHER_SENDER, "bug", "Another's").await;
  let seen = format!("/reports/{id}/seen");
  let replies = format!("/reports/{id}/replies");

  assert_eq!((count(&service, MAKER, true).await, count(&service, SENDER, false).await), (2, 0));
  assert!(!unread_to_sender(&service, &id).await);

  let (status, nothing) = service.maker(MAKER, Method::PUT, &seen, None).await;
  assert_eq!((status, nothing), (204, Value::Null));
  assert_eq!(
    count(&service, OTHER_MAKER, true).await,
    1,
    "one administrator's mark is the maker's whole"
  );
  let (_, inbox) = service.maker(OTHER_MAKER, Method::GET, "/reports", None).await;
  let marks: Vec<(&str, bool)> = inbox["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| (item["title"].as_str().unwrap(), item["unread"].as_bool().unwrap()))
    .collect();
  assert_eq!(marks, [("Another's", true), ("Unread", false)]);

  service
    .maker(MAKER, Method::POST, &replies, Some(json!({ "body": "Which page?" })))
    .await;
  assert_eq!(count(&service, MAKER, true).await, 1, "the maker's own reply is no news to the maker");
  assert_eq!(count(&service, SENDER, false).await, 1);
  assert!(unread_to_sender(&service, &id).await);
  service.sender(SENDER, Method::PUT, &seen, None).await;
  assert_eq!(count(&service, SENDER, false).await, 0);

  service
    .maker(
      MAKER,
      Method::PUT,
      &format!("/reports/{id}/state"),
      Some(json!({ "state": "under-review" })),
    )
    .await;
  assert_eq!(count(&service, SENDER, false).await, 1, "a state change is news to the sender");
  service.sender(SENDER, Method::PUT, &seen, None).await;

  service
    .sender(SENDER, Method::POST, &replies, Some(json!({ "body": "The month page." })))
    .await;
  assert_eq!(count(&service, MAKER, true).await, 2, "the sender's reply is news to the maker");
  assert_eq!(
    count(&service, SENDER, false).await,
    0,
    "the sender's own reply is no news to the sender"
  );
  assert_eq!(count(&service, OTHER_SENDER, false).await, 0, "a sender counts their own reports alone");
  assert_eq!(service.everything(Method::GET, "/unread", None).await, (200, json!({ "count": 0 })));
  service.stop().await;
}
