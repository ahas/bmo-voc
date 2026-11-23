//! The console face the service serves (`docs/api.md`, 「Console」), through bmo-chat's `bmo-console-face`: the
//! voc unit's declaration of the manifest fields it decides, over the bundle build's folder, guarded by the
//! console's credential.

use std::path::Path;

use bmo_console_face::{BundleFolder, ConsoleFace, ConsoleFaceError};
use serde_json::{Value, json};

use crate::settings::Settings;

/// The manifest fields the voc unit decides, as bmo-ui's console plugin contract names them.
pub const DECLARATION: &str = include_str!("console.json");

// What the bundle's build writes into its folder.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

/// The face over the bundle built into `folder`, guarded by `credential`, or why it cannot be served.
pub fn face(folder: &Path, credential: &str) -> Result<ConsoleFace, ConsoleFaceError> {
  let bundle = BundleFolder::new(folder, MODULE_FILE, WORDS_FILE).with_stylesheet(STYLESHEET_FILE);
  ConsoleFace::new(DECLARATION, &bundle, credential)
}

/// How a console reaches the service, as a deployment's console probe asks for it: the base address the settings
/// name, the console's credential and the service token, which the console's routes refuse.
pub fn reach(settings: &Settings) -> Value {
  json!({
    "address": settings.base_address(),
    "consoleToken": settings.console.token,
    "token": settings.server.token,
  })
}
