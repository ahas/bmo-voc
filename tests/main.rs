//! The crate's one test harness (bmo-chat's docs/conventions.md, 「Rust builds」): each file of tests a module
//! of it, sharing the process.

mod support;

mod attachments;
mod console;
mod events;
mod openapi;
mod reports;
mod repository;
mod store;
mod thread;
