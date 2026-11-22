# Contributing

Thanks for considering a contribution!

## Getting started

1. Fork and clone the repo.
2. Build: `cargo run` (Rust 1.85+ required).
3. Make your change.
4. Run the test suite: `cargo test` — all 26 tests must pass.
5. Keep formatting consistent: `cargo fmt` and `cargo clippy` should be clean.

## Guidelines

- **Keep the core pure.** `model.rs`, `ledger.rs`, `db.rs`, and `csv_io.rs`
  must stay free of GUI code so they remain reusable and easily testable.
  GUI code belongs in `ui.rs`; orchestration in `app.rs`.
- **New accounting behavior needs tests.** Any change to lot consumption,
  fee handling, or transfer/conversion semantics should come with a test
  in `tests/ledger_tests.rs` demonstrating the rule.
- **Document tax-affecting assumptions.** If your change alters how cost
  basis or gains are computed, update the "Tax accounting assumptions"
  section in the README.
- Screen-reader accessibility and translations are welcome; please open an
  issue first to discuss larger UI changes.