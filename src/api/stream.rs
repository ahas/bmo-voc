//! The event stream (`docs/api.md`, 「Events the voc unit publishes」): an event reaches the maker and a reader of
//! everything always, and a sender for their own reports alone, judged by the sender its data names.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event as Sent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures_util::{StreamExt, stream};

use super::{Answer, Served, query};
use crate::access::{Caller, caller, header_text};
use crate::error::ApiError;
use crate::wire::{EventsQuery, Page, StreamEvent};

/// How often a comment line keeps a stream open.
const KEEP_OPEN: Duration = Duration::from_secs(15);

/// Whether an event reaches a caller: every event the maker and a reader of everything, a sender's own report's
/// alone a sender.
fn reaches(caller: &Caller, event: &StreamEvent) -> bool {
  event.data["sender"].as_str().is_some_and(|sender| caller.reads(sender))
}

fn sent(event: &StreamEvent) -> Sent {
  Sent::default()
    .event(event.kind.clone())
    .id(event.id.to_string())
    .json_data(event)
    .unwrap_or_else(|_| Sent::default().comment("an event that did not render"))
}

fn gone() -> ApiError {
  ApiError::gone("The events after that number are no longer kept; read the reports again.")
}

pub(super) async fn events(
  State(served): State<Served>,
  headers: HeaderMap,
  given: Result<Query<EventsQuery>, QueryRejection>,
) -> Answer<Response> {
  let app = Arc::clone(&served.app);
  let caller = caller(&app, &headers)?;
  let given = query(given)?;
  let resumed = header_text(&headers, "last-event-id").and_then(|id| id.parse::<i64>().ok());
  let after = resumed.or(given.after);
  let kinds: Option<Vec<String>> = given
    .kinds
    .as_deref()
    .map(|kinds| kinds.split(',').map(str::trim).map(str::to_owned).collect());
  let wanted = move |event: &StreamEvent| {
    kinds.as_ref().is_none_or(|kinds| kinds.contains(&event.kind)) && reaches(&caller, event)
  };
  let paged = header_text(&headers, "accept").is_some_and(|accepted| accepted.contains("application/json"));
  if paged {
    let limit = given.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
      return Err(ApiError::bad_request("`limit` is 1 to 200."));
    }
    let (events, was_gone) = app.events.after(after.unwrap_or(app.events.first()));
    if was_gone && after.is_some() {
      return Err(gone());
    }
    let mut seen: Vec<StreamEvent> = events.into_iter().filter(|event| wanted(event)).collect();
    let more = seen.len() > limit as usize;
    seen.truncate(limit as usize);
    return Ok(Json(Page { items: seen, more }).into_response());
  }
  let live = app.events.subscribe();
  let backlog = match after {
    Some(after) => {
      let (events, was_gone) = app.events.after(after);
      if was_gone {
        return Err(gone());
      }
      events
    }
    None => Vec::new(),
  };
  let cursor = backlog.last().map(|event| event.id).or(after).unwrap_or_else(|| app.events.latest());
  let live = stream::unfold((live, cursor), |(mut live, cursor)| async move {
    loop {
      match live.recv().await {
        Ok(event) if event.id > cursor => {
          let id = event.id;
          return Some((event, (live, id)));
        }
        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
      }
    }
  });
  let events = stream::iter(backlog)
    .chain(live)
    .filter(move |event| std::future::ready(wanted(event)))
    .map(|event| Ok::<_, Infallible>(sent(&event)))
    .take_until(app.events.stopped());
  Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(KEEP_OPEN)).into_response())
}
