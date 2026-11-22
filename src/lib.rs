//! Crypto Inventory Tracker — library crate.
//!
//! Exposes every module so integration tests (and future consumers)
//! can use them directly. The binary entry point lives in `main.rs`.

pub mod app;
pub mod csv_io;
pub mod db;
pub mod ledger;
pub mod model;
pub mod ui;
