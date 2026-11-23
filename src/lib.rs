//! The voc service: the customers' voice box, the bug reports and suggestions people send the platform's maker
//! and the maker's answers, its own master over a SQLite store and a folder of attachments (`docs/design.md`,
//! `docs/api.md`).

pub mod access;
pub mod api;
pub mod app;
pub mod console;
pub mod error;
pub mod events;
pub mod openapi;
pub mod peers;
pub mod settings;
pub mod store;
pub mod wire;

use std::sync::Arc;

use tokio::net::TcpListener;

use crate::api::{Served, router};
use crate::app::App;
use crate::settings::Settings;
use crate::store::Store;

/// Opens the store and answers the routes on the settings' address until the process is told to stop, the open
/// event streams ended at once.
pub async fn serve(settings: Settings) -> Result<(), String> {
  let listener = TcpListener::bind(settings.server.listen)
    .await
    .map_err(|why| format!("{} could not be listened on: {why}", settings.server.listen))?;
  serve_on(settings, listener, stopped()).await
}

/// The service on a listener already bound, until `stop` resolves: the store opened, the attachments' folder made
/// where it does not stand, and the routes.
pub async fn serve_on(
  settings: Settings,
  listener: TcpListener,
  stop: impl Future<Output = ()> + Send + 'static,
) -> Result<(), String> {
  let store = Store::open(&settings.store.path)?;
  let files = &settings.store.files;
  std::fs::create_dir_all(files).map_err(|why| format!("{}: {why}", files.display()))?;
  let app = App::new(settings, store);
  // An empty console credential reaches nothing, so no face stands to be reached.
  let face = if app.settings.console.token.is_empty() {
    eprintln!("bmo-voc: the console face is not served: no console credential is set");
    None
  } else {
    match console::face(&app.settings.console.folder, &app.settings.console.token) {
      Ok(face) => Some(face),
      Err(why) => {
        eprintln!("bmo-voc: the console face is not served: {why}");
        None
      }
    }
  };
  let stopping = Arc::clone(&app);
  axum::serve(listener, router(Served { app, face }))
    .with_graceful_shutdown(async move {
      stop.await;
      stopping.events.stop();
    })
    .await
    .map_err(|why| why.to_string())
}

/// Resolves once the process is told to stop: an interrupt, or the service manager's terminate.
async fn stopped() {
  let interrupt = async {
    let _ = tokio::signal::ctrl_c().await;
  };
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => {
        signal.recv().await;
      }
      Err(_) => std::future::pending().await,
    }
  };
  tokio::select! {
    () = interrupt => {}
    () = terminate => {}
  }
}
