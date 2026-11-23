//! A report sent and read: sent whole with its files and the names the account service gave, refused whole for
//! any of its fields or files, read by its own sender alone and by the maker and a reader of everything, the
//! maker's inbox filtered and paged, and the companies the reports came from.

use reqwest::Method;
use serde_json::{Value, json};

use crate::support::{
  Call, LONE_SENDER, MAKER, MAX_BYTES, MAX_FILES, OTHER_SENDER, SENDER, Service, TOKEN, Upload, file,
  refusal, report,
};

/// A form sent and the refusal it gets: its `report` part, its files, and the status and code.
type Case<'a> = (Option<String>, Vec<Upload<'a>>, (u16, &'static str));

/// The ids of a page's reports, in order.
fn ids(page: &Value) -> Vec<String> {
  page["items"]
    .as_array()
    .unwrap()
    .iter()
    .map(|item| item["id"].as_str().unwrap().to_owned())
    .collect()
}

#[tokio::test]
async fn a_report_is_sent_whole_with_its_files_and_the_names_it_was_signed_with() {
  let service = Service::fresh().await;
  let mut written = report("bug", "  The save button does nothing  ");
  written["context"]["browser"] = Value::from("");
  let picture = [1u8; MAX_BYTES];
  let (status, sent) = service
    .send(
      SENDER,
      &written,
      &[
        file("screen.png", "image/png", &picture),
        Upload {
          name: "notes.log",
          kind: None,
          bytes: b"line one",
        },
      ],
    )
    .await;

  assert_eq!(status, 201, "{sent}");
  assert_eq!(sent["kind"], "bug");
  assert_eq!(sent["title"], "The save button does nothing", "the title is kept trimmed");
  assert_eq!(sent["body"], written["body"]);
  assert_eq!(
    sent["context"],
    json!({ "path": "/attendance/month", "consoleVersion": "0.1.0-rev.1", "browser": "" })
  );
  assert_eq!(sent["state"], "received");
  assert_eq!(sent["sender"], json!({ "user": SENDER, "name": "Test Sender" }));
  assert_eq!(sent["company"], json!({ "id": "c1", "name": "Company One" }));
  assert_eq!(sent["activityAt"], sent["sentAt"]);
  assert_eq!(sent["unread"], false, "nothing of the maker's stands for the sender to read");
  assert_eq!(sent["replies"], json!([]));
  assert_eq!(
    sent["states"],
    json!([{
      "state": "received",
      "by": { "user": SENDER, "name": "Test Sender", "side": "sender" },
      "at": sent["sentAt"],
    }])
  );
  let attachments = sent["attachments"].as_array().unwrap();
  let shapes: Vec<(&str, &str, i64)> = attachments
    .iter()
    .map(|kept| {
      (
        kept["name"].as_str().unwrap(),
        kept["type"].as_str().unwrap(),
        kept["size"].as_i64().unwrap(),
      )
    })
    .collect();
  assert_eq!(
    shapes,
    [
      ("screen.png", "image/png", MAX_BYTES as i64),
      ("notes.log", "application/octet-stream", 8)
    ]
  );
  let mut kept: Vec<String> =
    attachments.iter().map(|kept| kept["id"].as_str().unwrap().to_owned()).collect();
  kept.sort();
  assert_eq!(service.kept_files(), kept, "each file's bytes stand under its attachment's id");

  let member = |path: &str| Call {
    path: path.to_owned(),
    owner_key: Some(SENDER.to_owned()),
    owner_role: Some("member".to_owned()),
  };
  assert_eq!(
    service.stand.calls(),
    [member("/users/tester"), member("/users/tester/unit")],
    "the name and the company are read with the caller's own headers"
  );

  let (status, read) = service
    .sender(SENDER, Method::GET, &format!("/reports/{}", sent["id"].as_str().unwrap()), None)
    .await;
  assert_eq!(status, 200);
  assert_eq!(read, sent, "the report reads as it was sent");
  service.stop().await;
}

#[tokio::test]
async fn a_sender_the_account_service_names_by_login_alone_or_places_nowhere_is_kept_so() {
  let service = Service::fresh().await;
  let (_, nameless) = service.send(OTHER_SENDER, &report("suggestion", "Dark mode"), &[]).await;
  let (_, alone) = service.send(LONE_SENDER, &report("bug", "A crash"), &[]).await;

  assert_eq!(nameless["sender"], json!({ "user": OTHER_SENDER, "name": OTHER_SENDER }));
  assert_eq!(nameless["kind"], "suggestion");
  assert_eq!(alone["sender"]["name"], "Lone Sender");
  assert_eq!(alone["company"], Value::Null);
  service.stop().await;
}

#[tokio::test]
async fn a_report_refused_for_a_field_or_a_file_keeps_nothing() {
  let service = Service::fresh().await;
  let small = b"bytes";
  let large = [0u8; MAX_BYTES + 1];
  let valid = report("bug", "A title").to_string();
  let with = |field: &str, value: Value| {
    let mut written = report("bug", "A title");
    written[field] = value;
    written.to_string()
  };
  let many: Vec<_> = (0..=MAX_FILES).map(|_| file("a.png", "image/png", small)).collect();
  let long_name = "n".repeat(201);
  let cases: Vec<Case> = vec![
    (Some(with("title", Value::from("   "))), vec![], (422, "bad-report")),
    (Some(with("title", Value::from("t".repeat(201)))), vec![], (422, "bad-report")),
    (Some(with("body", Value::from(" \n "))), vec![], (422, "bad-report")),
    (Some(with("body", Value::from("b".repeat(20_001)))), vec![], (422, "bad-report")),
    (Some(with("kind", Value::from("praise"))), vec![], (422, "bad-report")),
    (Some("not json".to_owned()), vec![], (422, "bad-report")),
    (None, vec![file("a.png", "image/png", small)], (422, "bad-report")),
    (Some(valid.clone()), vec![file(&long_name, "text/plain", small)], (422, "bad-report")),
    (Some(valid.clone()), many, (413, "too-many-files")),
    (
      Some(valid.clone()),
      vec![file("a.png", "image/png", small), file("b.png", "image/png", &large)],
      (413, "file-too-large"),
    ),
    (
      Some(valid.clone()),
      vec![file("a.png", "image/png", small), file("empty.txt", "text/plain", b"")],
      (413, "file-too-large"),
    ),
  ];
  for (index, (part, files, refused)) in cases.into_iter().enumerate() {
    let answer = service.send_form(SENDER, "member", part.as_deref(), &files).await;
    assert_eq!(refusal(&answer), refused, "case {index}: {}", answer.1);
  }
  assert_eq!(
    refusal(&service.send_form(MAKER, "administrator", Some(&valid), &[]).await),
    (403, "maker-side")
  );
  assert_eq!(
    refusal(&service.post_form(Some(TOKEN), &[], Some(&valid), &[]).await),
    (403, "forbidden"),
    "a reader of everything sends nothing"
  );

  assert!(service.stand.calls().is_empty(), "a report refused asks the account service nothing");
  assert!(service.kept_files().is_empty(), "no file of a refused report is kept");
  let (_, inbox) = service.maker(MAKER, Method::GET, "/reports", None).await;
  assert_eq!(inbox["items"], json!([]));
  service.stop().await;
}

#[tokio::test]
async fn a_sender_reads_their_own_reports_alone_and_another_s_is_not_found() {
  let service = Service::fresh().await;
  let theirs = service.sent(SENDER, "bug", "Mine").await;
  let (_, with_file) = service
    .send(SENDER, &report("bug", "With a file"), &[file("a.png", "image/png", b"png")])
    .await;
  let attachment = with_file["attachments"][0]["id"].as_str().unwrap();
  let with_file = with_file["id"].as_str().unwrap();
  let own = service.sent(OTHER_SENDER, "suggestion", "Theirs").await;

  let (_, listed) = service.sender(OTHER_SENDER, Method::GET, "/reports", None).await;
  assert_eq!(ids(&listed), [own.as_str()]);
  for (method, path, body) in [
    (Method::GET, format!("/reports/{theirs}"), None),
    (Method::GET, format!("/reports/{with_file}/attachments/{attachment}"), None),
    (Method::POST, format!("/reports/{theirs}/replies"), Some(json!({ "body": "Me too" }))),
    (Method::PUT, format!("/reports/{theirs}/seen"), None),
    (Method::GET, "/reports/no-such-report".to_owned(), None),
  ] {
    let answer = service.sender(OTHER_SENDER, method, &path, body).await;
    assert_eq!(refusal(&answer), (404, "not-found"), "{path}");
  }
  let (status, read) = service.sender(OTHER_SENDER, Method::GET, &format!("/reports/{own}"), None).await;
  assert_eq!((status, read["title"].as_str()), (200, Some("Theirs")));
  service.stop().await;
}

#[tokio::test]
async fn the_maker_reads_every_report_newest_activity_first_filtered_and_paged() {
  let service = Service::fresh().await;
  let first = service.sent(SENDER, "bug", "First").await;
  let second = service.sent(OTHER_SENDER, "suggestion", "Second").await;
  let third = service.sent(LONE_SENDER, "bug", "Third").await;
  let page = |query: &'static str| {
    let service = &service;
    async move {
      let (status, page) = service.maker(MAKER, Method::GET, &format!("/reports{query}"), None).await;
      assert_eq!(status, 200, "{page}");
      page
    }
  };

  assert_eq!(ids(&page("").await), [third.as_str(), second.as_str(), first.as_str()]);
  let (status, _) = service
    .maker(
      MAKER,
      Method::POST,
      &format!("/reports/{first}/replies"),
      Some(json!({ "body": "Looking" })),
    )
    .await;
  assert_eq!(status, 201);
  let all = page("").await;
  assert_eq!(
    ids(&all),
    [first.as_str(), third.as_str(), second.as_str()],
    "a reply is the report's latest act"
  );
  assert_eq!(all["items"][0]["replies"], 1);
  assert_eq!(all["items"][0]["company"], json!({ "id": "c1", "name": "Company One" }));
  assert_eq!(all["items"][0]["sender"], json!({ "user": SENDER, "name": "Test Sender" }));
  assert_eq!(all["more"], false);

  assert_eq!(ids(&page("?kind=bug").await), [first.as_str(), third.as_str()]);
  assert_eq!(ids(&page("?kind=suggestion").await), [second.as_str()]);
  assert_eq!(ids(&page("?company=c2").await), [second.as_str()]);
  service
    .maker(
      MAKER,
      Method::PUT,
      &format!("/reports/{third}/state"),
      Some(json!({ "state": "declined" })),
    )
    .await;
  assert_eq!(ids(&page("?state=declined").await), [third.as_str()]);
  assert_eq!(ids(&page("?state=received&kind=bug").await), [first.as_str()]);

  let head = page("?limit=2").await;
  assert_eq!((ids(&head).len(), &head["more"]), (2, &Value::Bool(true)));
  let tail = page("?limit=2&after=2").await;
  assert_eq!((ids(&tail), &tail["more"]), (vec![second.clone()], &Value::Bool(false)));
  for wrong in ["?kind=praise", "?state=open", "?limit=0", "?limit=201", "?after=-1"] {
    let answer = service.maker(MAKER, Method::GET, &format!("/reports{wrong}"), None).await;
    assert_eq!(refusal(&answer), (400, "bad-request"), "{wrong}");
  }

  let (_, own) = service.sender(SENDER, Method::GET, "/reports?kind=suggestion&company=c2", None).await;
  assert_eq!(ids(&own), [first], "a sender's filters are answered as if they named none");
  let (_, everything) = service.everything(Method::GET, "/reports?kind=suggestion", None).await;
  assert_eq!(ids(&everything), [second.as_str()]);
  assert_eq!(everything["items"][0]["unread"], false, "a reader of everything has nothing unread");
  service.stop().await;
}

#[tokio::test]
async fn the_companies_are_named_as_their_latest_report_names_them_to_the_maker_alone() {
  let service = Service::fresh().await;
  service.sent(OTHER_SENDER, "bug", "One").await;
  service.sent(SENDER, "bug", "Two").await;
  service.sent(LONE_SENDER, "bug", "Three").await;
  service.stand.place(OTHER_SENDER, "c2", "Another Name");
  service.sent(OTHER_SENDER, "bug", "Four").await;

  let expected = json!({
    "items": [{ "id": "c2", "name": "Another Name" }, { "id": "c1", "name": "Company One" }],
    "more": false,
  });
  assert_eq!(service.maker(MAKER, Method::GET, "/companies", None).await, (200, expected.clone()));
  assert_eq!(service.everything(Method::GET, "/companies", None).await, (200, expected));
  assert_eq!(
    refusal(&service.sender(SENDER, Method::GET, "/companies", None).await),
    (403, "sender-side")
  );
  service.stop().await;
}

#[tokio::test]
async fn a_reader_of_everything_reads_every_report_and_writes_nothing() {
  let service = Service::fresh().await;
  let id = service.sent(SENDER, "bug", "Read me").await;

  assert_eq!(
    service.everything(Method::GET, "/me", None).await,
    (
      200,
      json!({ "user": null, "side": "reader", "limits": { "maxFiles": MAX_FILES, "maxBytes": MAX_BYTES } })
    )
  );
  let (status, read) = service.everything(Method::GET, &format!("/reports/{id}"), None).await;
  assert_eq!((status, read["unread"].as_bool()), (200, Some(false)));
  for (method, path, body) in [
    (Method::POST, format!("/reports/{id}/replies"), Some(json!({ "body": "Hello" }))),
    (Method::PUT, format!("/reports/{id}/state"), Some(json!({ "state": "done" }))),
    (Method::PUT, format!("/reports/{id}/seen"), None),
  ] {
    assert_eq!(refusal(&service.everything(method, &path, body).await), (403, "forbidden"), "{path}");
  }
  let (_, read) = service.everything(Method::GET, &format!("/reports/{id}"), None).await;
  assert_eq!(
    (read["state"].as_str(), read["replies"].as_array().map(Vec::len)),
    (Some("received"), Some(0))
  );
  service.stop().await;
}

#[tokio::test]
async fn a_failing_account_service_fails_each_write_that_needs_it_and_nothing_is_written() {
  let service = Service::fresh().await;
  let id = service.sent(SENDER, "bug", "Standing").await;
  service.stand.fail(true);

  let (_, page) = service
    .ask(Method::GET, "/events", Some(TOKEN), &[("accept", "application/json")], None)
    .await;
  let published = page["items"].as_array().unwrap().len();
  let answers = [
    service
      .send(SENDER, &report("bug", "Lost"), &[file("a.png", "image/png", b"png")])
      .await,
    service
      .sender(SENDER, Method::POST, &format!("/reports/{id}/replies"), Some(json!({ "body": "More" })))
      .await,
    service
      .maker(MAKER, Method::POST, &format!("/reports/{id}/replies"), Some(json!({ "body": "Seen" })))
      .await,
    service
      .maker(MAKER, Method::PUT, &format!("/reports/{id}/state"), Some(json!({ "state": "done" })))
      .await,
  ];
  for answer in &answers {
    assert_eq!(refusal(answer), (502, "upstream-failed"), "{}", answer.1);
  }

  let (_, inbox) = service.maker(MAKER, Method::GET, "/reports", None).await;
  assert_eq!(ids(&inbox), [id.as_str()]);
  let (_, read) = service.maker(MAKER, Method::GET, &format!("/reports/{id}"), None).await;
  assert_eq!(
    (read["state"].as_str(), read["replies"].as_array().map(Vec::len)),
    (Some("received"), Some(0))
  );
  assert!(service.kept_files().is_empty(), "the refused report's file is not kept");
  let (_, page) = service
    .ask(Method::GET, "/events", Some(TOKEN), &[("accept", "application/json")], None)
    .await;
  assert_eq!(page["items"].as_array().unwrap().len(), published, "nothing written publishes nothing");
  service.stop().await;
}
