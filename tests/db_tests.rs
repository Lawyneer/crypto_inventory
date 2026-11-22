use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crypto_inventory::db::open_db_at;
use rusqlite::{Connection, params};

/// Unique temp-file path per test invocation, so parallel tests
/// never touch the same database.
fn temp_db(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{}/crypto_tracker_test_{}_{}.db",
        std::env::temp_dir().display(),
        tag,
        nanos
    )
}

fn cleanup(path: &str) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{path}-wal"));
    let _ = fs::remove_file(format!("{path}-shm"));
}

#[test]
fn fresh_database_gets_the_full_schema() {
    let path = temp_db("fresh");
    {
        let conn = open_db_at(&path).unwrap();
        // Insert a wallet and a transaction, read them back.
        conn.execute(
            "INSERT INTO wallets (name, address) VALUES (?1, ?2)",
            params!["Test Wallet", "bc1q..."],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transactions
             (tx_type, wallet_id, coin, ts, exchange, amount, fee, unit_price, fiat_total)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                "Purchase",
                1,
                "BTC",
                "2024-01-01 10:00",
                "TestEx",
                0.5,
                1.0,
                100.0,
                51.0
            ],
        )
        .unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
    cleanup(&path);
}

#[test]
fn old_schema_is_migrated_in_place_with_data_intact() {
    let path = temp_db("migrate");
    {
        // Build the OLD (pre-transfer/convert) schema by hand.
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE wallets (
                 id      INTEGER PRIMARY KEY AUTOINCREMENT,
                 name    TEXT NOT NULL UNIQUE,
                 address TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE settings (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE transactions (
                 id          INTEGER PRIMARY KEY AUTOINCREMENT,
                 tx_type     TEXT NOT NULL CHECK (tx_type IN ('Purchase','Sell','Spend')),
                 wallet_id   INTEGER NOT NULL REFERENCES wallets(id),
                 coin        TEXT NOT NULL,
                 ts          TEXT NOT NULL,
                 exchange    TEXT NOT NULL DEFAULT '',
                 amount      REAL NOT NULL,
                 fee         REAL NOT NULL DEFAULT 0,
                 unit_price  REAL NOT NULL DEFAULT 0,
                 fiat_total  REAL NOT NULL DEFAULT 0,
                 description TEXT NOT NULL DEFAULT '',
                 fiat_value  REAL NOT NULL DEFAULT 0
             );
             INSERT INTO wallets (name, address) VALUES ('Old Wallet', '');
             INSERT INTO transactions
                 (tx_type, wallet_id, coin, ts, exchange, amount, unit_price, fiat_total)
             VALUES ('Purchase', 1, 'BTC', '2024-01-01 10:00', 'Legacy', 1.0, 100.0, 100.0);",
        )
        .unwrap();
        conn.close().unwrap();
    }
    {
        // Opening must migrate and preserve the row.
        let conn = open_db_at(&path).unwrap();

        // New columns are queryable after migration.
        let probe = conn.prepare(
            "SELECT to_wallet_id, to_coin, asset_fee, amount_to FROM transactions LIMIT 1",
        );
        assert!(probe.is_ok(), "new columns missing after migration");
        drop(probe);

        let (ty, coin, amt): (String, String, f64) = conn
            .query_row(
                "SELECT tx_type, coin, amount FROM transactions WHERE wallet_id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(ty, "Purchase");
        assert_eq!(coin, "BTC");
        assert!((amt - 1.0).abs() < 1e-9);

        // New transaction types are now accepted by the widened constraint.
        conn.execute(
            "INSERT INTO transactions
             (tx_type, wallet_id, coin, ts, amount, asset_fee)
             VALUES ('Transfer', 1, 'BTC', '2024-02-01 10:00', 0.5, 0.001)",
            [],
        )
        .expect("Transfer should be insertable after migration");

        // The legacy scratch table is gone.
        let legacy: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE name='transactions_legacy'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        assert!(!legacy);
    }
    cleanup(&path);
}

#[test]
fn opening_twice_is_idempotent() {
    let path = temp_db("twice");
    {
        open_db_at(&path).unwrap();
    }
    {
        let conn = open_db_at(&path).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='transactions'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1); // not duplicated or renamed
    }
    cleanup(&path);
}
