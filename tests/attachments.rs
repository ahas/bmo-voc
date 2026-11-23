//! An attachment handed back: its bytes with its type and name, a picture a browser shows inline and any other
//! file, a picture a browser could run among them, as a download, every answer with `nosniff`.

use reqwest::Method;

use crate::support::{CONSOLE, MAKER, SENDER, Service, Upload, console, file};

#[tokio::test]
async fn a_picture_is_shown_inline_and_any_other_file_is_handed_as_a_download() {
  let service = Service::fresh().await;
  let files = [
    file("screen.png", "image/png", b"png bytes"),
    file("photo.JPEG", "IMAGE/JPEG", b"jpeg bytes"),
    file("drawing.svg", "image/svg+xml", b"<svg/>"),
    Upload {
      name: "page one \u{e9}.html",
      kind: Some("text/html"),
      bytes: b"<p>",
    },
  ];
  let (_, first) = service.send(SENDER, &crate::support::report("bug", "Pictures"), &files[..3]).await;
  let (_, second) = service.send(SENDER, &crate::support::report("bug", "A page"), &files[3..]).await;
  let attachments: Vec<(String, String)> = [&first, &second]
    .iter()
    .flat_map(|report| {
      let id = report["id"].as_str().unwrap().to_owned();
      report["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .map(move |kept| (id.clone(), kept["id"].as_str().unwrap().to_owned()))
        .collect::<Vec<_>>()
    })
    .collect();

  let expected = [
    ("image/png", "inline; filename=\"screen.png\"; filename*=UTF-8''screen.png", "png bytes"),
    (
      "image/jpeg",
      "inline; filename=\"photo.JPEG\"; filename*=UTF-8''photo.JPEG",
      "jpeg bytes",
    ),
    (
      "image/svg+xml",
      "attachment; filename=\"drawing.svg\"; filename*=UTF-8''drawing.svg",
      "<svg/>",
    ),
    (
      "text/html",
      "attachment; filename=\"page one _.html\"; filename*=UTF-8''page%20one%20%C3%A9.html",
      "<p>",
    ),
  ];
  for ((report, attachment), (kind, disposition, bytes)) in attachments.iter().zip(expected) {
    for (user, role) in [(SENDER, "member"), (MAKER, "administrator")] {
      let response = service
        .request(
          Method::GET,
          &format!("/reports/{report}/attachments/{attachment}"),
          Some(CONSOLE),
          &console(user, role),
        )
        .send()
        .await
        .unwrap();
      let header = |name: &str| response.headers()[name].to_str().unwrap().to_owned();
      assert_eq!(response.status().as_u16(), 200);
      assert_eq!(header("content-type"), kind);
      assert_eq!(header("content-disposition"), disposition);
      assert_eq!(header("x-content-type-options"), "nosniff");
      assert_eq!(response.text().await.unwrap(), bytes);
    }
  }

  let other = &second["id"].as_str().unwrap();
  let (status, _) = service
    .sender(SENDER, Method::GET, &format!("/reports/{other}/attachments/{}", attachments[0].1), None)
    .await;
  assert_eq!(status, 404, "an attachment is read on its own report alone");
  service.stop().await;
}
