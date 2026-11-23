//! The events the voc unit publishes (`docs/api.md`, 「Events the voc unit publishes」): kept since the start, at
//! most the last 1000, numbered upward from the start's moment in milliseconds times 1000, and handed to every
//! open stream as they are published.

use std::collections::VecDeque;
use std::sync::Mutex;

use serde::Serialize;
use tokio::sync::{broadcast, watch};

use crate::store::now_ms;
use crate::wire::StreamEvent;

/// How many events are kept.
pub const KEPT: usize = 1000;

/// A report sent by a sender.
pub const RECEIVED: &str = "report.received";
/// A reply of the maker's.
pub const ANSWERED: &str = "report.answered";
/// A reply of the sender's.
pub const FOLLOWED: &str = "report.followed";
/// A state moved by the maker.
pub const CHANGED: &str = "report.changed";

/// The kinds the unit publishes.
pub const KINDS: [&str; 4] = [RECEIVED, ANSWERED, FOLLOWED, CHANGED];

/// The kept events and the live readers.
pub struct Events {
  kept: Mutex<(i64, VecDeque<StreamEvent>)>,
  first: i64,
  live: broadcast::Sender<StreamEvent>,
  stop: watch::Sender<bool>,
}

impl Default for Events {
  fn default() -> Self {
    Self::new()
  }
}

impl Events {
  pub fn new() -> Self {
    let first = now_ms() * 1000;
    let (live, _) = broadcast::channel(KEPT);
    let (stop, _) = watch::channel(false);
    Self {
      kept: Mutex::new((first, VecDeque::new())),
      first,
      live,
      stop,
    }
  }

  /// The number the first event of this run would take, less one: a resume from before it is gone.
  pub fn first(&self) -> i64 {
    self.first
  }

  /// One event published to every reader it reaches.
  pub fn publish(&self, kind: &str, data: impl Serialize) {
    let Ok(mut kept) = self.kept.lock() else {
      return;
    };
    kept.0 += 1;
    let event = StreamEvent {
      id: kept.0,
      kind: kind.to_owned(),
      at: now_ms(),
      data: serde_json::to_value(data).unwrap_or_default(),
    };
    kept.1.push_back(event.clone());
    while kept.1.len() > KEPT {
      kept.1.pop_front();
    }
    let _ = self.live.send(event);
  }

  /// The events kept after `after`, and whether `after` stands before the window (and so is gone).
  pub fn after(&self, after: i64) -> (Vec<StreamEvent>, bool) {
    let Ok(kept) = self.kept.lock() else {
      return (Vec::new(), false);
    };
    let oldest = kept.1.front().map(|event| event.id).unwrap_or(kept.0 + 1);
    let gone = after < self.first || after + 1 < oldest;
    (kept.1.iter().filter(|event| event.id > after).cloned().collect(), gone)
  }

  /// The last number given.
  pub fn latest(&self) -> i64 {
    self.kept.lock().map(|kept| kept.0).unwrap_or(self.first)
  }

  /// A reader of the events published from now.
  pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
    self.live.subscribe()
  }

  /// Ends every open stream.
  pub fn stop(&self) {
    let _ = self.stop.send(true);
  }

  /// Resolves once the streams are ended.
  pub fn stopped(&self) -> impl Future<Output = ()> + Send + 'static {
    let mut stop = self.stop.subscribe();
    async move {
      let _ = stop.wait_for(|stopped| *stopped).await;
    }
  }
}
