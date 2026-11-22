//! Database layer: connection bootstrap, schema creation and
//! in-place migration from older versions of the schema.

use rusqlite::Connection;

pub const CREATE_TRANSACTIONS: &str = "
    CREATE TABLE IF NOT EXISTS transactions (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        tx_type      TEXT NOT NULL CHECK (tx_type IN ('Purchase','Sell','Spend','Transfer','Convert')),
        wallet_id    INTEGER NOT NULL REFERENCES wallets(id),
        to_wallet_id INTEGER NOT NULL DEFAULT 0,
        coin         TEXT NOT NULL,
        to_coin      TEXT NOT NULL DEFAULT '',
        ts           TEXT NOT NULL,
        exchange     TEXT NOT NULL DEFAULT '',
        amount       REAL NOT NULL,
        amount_to    REAL NOT NULL DEFAULT 0,
        fee          REAL NOT NULL DEFAULT 0,
        asset_fee    REAL NOT NULL DEFAULT 0,
        unit_price   REAL NOT NULL DEFAULT 0,
        fiat_total   REAL NOT NULL DEFAULT 0,
        description  TEXT NOT NULL DEFAULT '',
        fiat_value   REAL NOT NULL DEFAULT 0
    );
";

/// Open the tracker database at the default location.
pub fn open_db() -> rusqlite::Result<Connection> {
    open_db_at("crypto_tracker.db")
}

/// Open (creating or migrating as needed) the tracker database
/// at an explicit path. Used by tests to isolate databases.
pub fn open_db_at(path: &str) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS wallets (
             id      INTEGER PRIMARY KEY AUTOINCREMENT,
             name    TEXT NOT NULL UNIQUE,
             address TEXT NOT NULL DEFAULT ''
         );
         CREATE TABLE IF NOT EXISTS settings (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );",
    )?;

    let table_exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='transactions'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        conn.execute_batch(CREATE_TRANSACTIONS)?;
    } else {
        // Older schema (Purchase/Sell/Spend only) lacks the new columns.
        let has_new_cols = conn
            .prepare("SELECT to_wallet_id, to_coin, asset_fee, amount_to FROM transactions LIMIT 0")
            .is_ok();
        if !has_new_cols {
            conn.execute_batch(&format!(
                "ALTER TABLE transactions RENAME TO transactions_legacy;
                 {CREATE_TRANSACTIONS}
                 INSERT INTO transactions
                    (tx_type, wallet_id, coin, ts, exchange, amount, fee,
                     unit_price, fiat_total, description, fiat_value)
                 SELECT tx_type, wallet_id, coin, ts, exchange, amount, fee,
                        unit_price, fiat_total, description, fiat_value
                 FROM transactions_legacy;
                 DROP TABLE transactions_legacy;"
            ))?;
        }
    }
    Ok(conn)
}
