//! The store's schema: the numbered migrations applied in order as the store opens, each once, and a file holding
//! more than the binary knows refused.

use rusqlite::Connection;

use bmo_voc::store::Store;

use crate::support::scratch;

const MIGRATIONS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/migrations");

/// How many migrations the file holds, by its `user_version`.
fn held(path: &std::path::Path) -> i64 {
  Connection::open(path)
    .unwrap()
    .query_row("PRAGMA user_version", [], |row| row.get(0))
    .unwrap()
}

#[test]
fn every_migration_is_applied_once_and_a_newer_file_is_refused() {
  let written = std::fs::read_dir(MIGRATIONS).unwrap().count() as i64;
  let path = scratch().join("voc.db");

  drop(Store::open(&path).unwrap());
  assert_eq!(held(&path), written);
  drop(Store::open(&path).expect("a store holding every migration opens again, applying none twice"));
  assert_eq!(held(&path), written);

  Connection::open(&path)
    .unwrap()
    .execute_batch(&format!("PRAGMA user_version = {}", written + 1))
    .unwrap();
  assert!(
    Store::open(&path).is_err(),
    "a store holding a migration this binary does not know is refused"
  );
}
