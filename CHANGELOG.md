# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-22

### Added

- Transaction tracking for purchases, sales, and spends (goods/services
  bought with crypto), with exchange, fee, amount, and rate details.
- Flexible price entry for purchases and sales: enter either the exchange
  rate (total derived) or the total fiat amount (rate derived).
- Multi-wallet support: track the same coin across any number of wallets,
  with per-wallet balances and cost basis.
- Wallet-to-wallet transfers, treated as non-taxable: lots keep their
  original cost basis and acquisition date; asset-denominated network fees
  are deducted proportionally.
- Coin-to-coin conversions (e.g. BTC → XMR), treated as a disposal plus an
  acquisition with realized gain computed on the disposal leg.
- Inventory methods: FIFO, LIFO, and HIFO, switchable at any time with
  instant recomputation of all realized gains.
- Realized gain/loss reporting per disposal event and in total.
- Tax-lot view showing every remaining open lot with acquisition date and
  cost basis.
- CSV export of transactions, realized gains, and tax lots.
- CSV import of transactions with a native file-open dialog and per-row
  error reporting.
- Clear-fields button on the transaction entry form.
- Automatic in-place migration of databases created by pre-release schemas.
- 26-test integration suite covering the inventory engine, database
  migration, and CSV round-tripping; CI via GitHub Actions.

[1.0.0]: https://github.com/Lawyneer/crypto_inventory/releases/tag/v1.0.0