//! The service as it runs: its settings, the HTTP client it calls the account service with, the store it is
//! master of and the events it publishes.

use std::sync::Arc;

use crate::events::Events;
use crate::peers;
use crate::settings::Settings;
use crate::store::Store;

/// The service's state, shared by every route.
pub struct App {
  pub settings: Settings,
  pub http: reqwest::Client,
  pub store: Store,
  pub events: Events,
}

impl App {
  /// The service's state over `store`.
  pub fn new(settings: Settings, store: Store) -> Arc<Self> {
    Arc::new(Self {
      settings,
      http: peers::http(),
      store,
      events: Events::new(),
    })
  }
}
