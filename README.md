# Crypto Inventory Tracker

A privacy-focused desktop application for tracking cryptocurrency holdings,
built in Rust. All data stays on your machine in a local SQLite database —
no accounts, no cloud, no telemetry.

![CI](https://github.com/Lawyneer/crypto_inventory/actions/workflows/ci.yml/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)
![Release](https://img.shields.io/github/v/tag/Lawyneer/crypto_inventory)

## Features

- **Transaction tracking** — record purchases, sales, and spends (purchases
  of goods or services with crypto), each with the details that matter:
  exchange, fees, amounts, and rates.
- **Flexible price entry** — for buys and sells, enter either the exchange
  rate (total is derived) or the total fiat amount (rate is derived).
- **Multi-wallet support** — hold the same coin across any number of
  wallets/exchange addresses, with per-wallet balances and cost basis.
- **Wallet transfers** — move coins between wallets; non-taxable, with lots
  keeping their original cost basis and acquisition date.
- **Coin conversions** — e.g. BTC → XMR, treated as a disposal plus an
  acquisition, with realized gain computed on the disposal leg.
- **Inventory methods** — FIFO, LIFO, or HIFO (highest-price-first) lot
  consumption. Switch anytime; all realized gains recompute instantly.
- **Realized gain/loss** — per disposal event and in total.
- **Tax-lot view** — every remaining open lot with acquisition date and basis.
- **CSV import/export** — transactions, realized gains, and tax lots, with a
  native file-open dialog on import.

## Building

Requires Rust 1.85 or later (edition 2024).

```bash
git clone https://github.com/Lawyneer/crypto_inventory.git
cd crypto_inventory
cargo run
```

The first build takes a few minutes (SQLite is compiled in via the
`bundled` feature — no system installation needed). A `crypto_tracker.db`
database is created in the working directory on first launch.

Native builds are supported on Windows, macOS, and Linux. On Linux the file
dialog may require GTK development packages (`libgtk-3-dev` or equivalent).

## Tax accounting assumptions

This tool makes simplifying assumptions you should be aware of before using
its numbers for tax purposes:

- **Transfers between your own wallets are not taxable events.** Lots move
  with their original cost basis and acquisition date. The network fee
  (denominated in the coin) reduces the arriving amount but is not treated
  as a disposal.
- **Coin-to-coin conversions are taxable** (the standard treatment in the
  US): the disposal leg computes gain/loss against consumed lots, and the
  acquired coin's basis equals the fiat value received plus any exchange fee.
- **Spends record a manual fiat value** ("fair market value at time of
  spend") which serves as the disposal proceeds.
- **Fees:** purchase fees increase cost basis; sale fees reduce proceeds.
- **Disposals consume lots per wallet** — a sale in one wallet never touches
  another wallet's lots.
- Sales exceeding available lots produce a shortfall with zero basis on the
  excess; check the tax-lot view if numbers look surprising.

**This is a tracking tool, not tax advice.** Verify results against your
jurisdiction's rules before filing.

## Project layout

```
.
├── .github/workflows/ci.yml — CI: build + test on every push/PR
├── .gitignore               — excludes build output, local database, exports
├── Cargo.toml               — manifest (bin + lib targets)
├── Cargo.lock               — locked dependency versions (committed)
├── rust-toolchain.toml      — pins the Rust toolchain for builds & CI
├── LICENSE                  — MIT
├── README.md
├── CONTRIBUTING.md
├── src/
│   ├── main.rs              — binary entry point
│   ├── lib.rs               — library root
│   ├── app.rs               — application state and logic
│   ├── ui.rs                — egui panels
│   ├── model.rs             — domain types
│   ├── ledger.rs            — inventory engine (FIFO/LIFO/HIFO replay)
│   ├── db.rs                — SQLite schema and migration
│   └── csv_io.rs            — CSV parsing and export
└── tests/
    ├── model_tests.rs       — enums and formatting
    ├── ledger_tests.rs      — inventory engine and accounting rules
    ├── csv_io_tests.rs      — CSV parsing and round-trips
    └── db_tests.rs          — schema, migration, idempotency
```

The core modules — model, ledger, db, and csv_io — are pure (no GUI code)
and reusable in other projects; app and ui hold the eframe/egui application
layer. Builds and CI use the Rust toolchain pinned in rust-toolchain.toml.

## Testing

```bash
cargo test
```

26 integration tests cover the inventory engine, database migration, and
CSV round-tripping. The GUI layer is exercised manually.

## License

MIT — see [LICENSE](LICENSE).