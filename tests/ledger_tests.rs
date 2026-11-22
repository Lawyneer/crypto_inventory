use std::collections::HashMap;

use crypto_inventory::ledger::compute_ledger;
use crypto_inventory::model::{Kind, Method, Tx};

fn names() -> HashMap<i64, String> {
    HashMap::from([(1, "A".to_string()), (2, "B".to_string())])
}

/// Test Tx builder with sensible zero-defaults for unused fields.
fn tx(id: i64, kind: Kind, wallet: i64, coin: &str, ts: &str, amount: f64, fiat: f64) -> Tx {
    Tx {
        id,
        kind,
        wallet_id: wallet,
        to_wallet_id: 0,
        coin: coin.into(),
        to_coin: String::new(),
        ts: ts.into(),
        exchange: String::new(),
        amount,
        amount_to: 0.0,
        fee: 0.0,
        asset_fee: 0.0,
        unit_price: 0.0,
        fiat_total: fiat,
        description: String::new(),
        fiat_value: fiat,
    }
}

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn purchase_creates_holding_and_tax_lot() {
    let txs = vec![tx(
        1,
        Kind::Purchase,
        1,
        "BTC",
        "2024-01-01 10:00",
        0.5,
        1500.0,
    )];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    assert_eq!(out.holdings.len(), 1);
    approx(out.holdings[0].amount, 0.5);
    approx(out.holdings[0].basis, 1500.0);
    approx(out.holdings[0].avg_cost, 3000.0);
    assert_eq!(out.tax_lots.len(), 1);
    assert_eq!(out.tax_lots[0].ts, "2024-01-01 10:00");
}

#[test]
fn fifo_consumes_oldest_lot_first() {
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        tx(2, Kind::Purchase, 1, "BTC", "2024-02-01 10:00", 1.0, 200.0),
        tx(3, Kind::Sell, 1, "BTC", "2024-03-01 10:00", 1.0, 250.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    assert_eq!(out.realized.len(), 1);
    approx(out.realized[0].basis, 100.0); // oldest lot
    approx(out.realized[0].gain, 150.0);
    approx(out.holdings[0].basis, 200.0); // newer lot remains
}

#[test]
fn lifo_consumes_newest_lot_first() {
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        tx(2, Kind::Purchase, 1, "BTC", "2024-02-01 10:00", 1.0, 200.0),
        tx(3, Kind::Sell, 1, "BTC", "2024-03-01 10:00", 1.0, 250.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Lifo);
    approx(out.realized[0].basis, 200.0); // newest lot
    approx(out.realized[0].gain, 50.0);
}

#[test]
fn hifo_consumes_most_expensive_lot_first() {
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        tx(2, Kind::Purchase, 1, "BTC", "2024-02-01 10:00", 1.0, 300.0),
        tx(3, Kind::Purchase, 1, "BTC", "2024-03-01 10:00", 1.0, 200.0),
        tx(4, Kind::Sell, 1, "BTC", "2024-04-01 10:00", 1.0, 250.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Hifo);
    approx(out.realized[0].basis, 300.0); // highest unit cost
    approx(out.realized[0].gain, -50.0);
}

#[test]
fn partial_consumption_splits_a_lot() {
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 2.0, 200.0),
        tx(2, Kind::Sell, 1, "BTC", "2024-02-01 10:00", 0.5, 75.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    approx(out.realized[0].basis, 50.0); // 0.5 of the 100/coin lot
    approx(out.holdings[0].amount, 1.5);
    approx(out.holdings[0].basis, 150.0);
}

#[test]
fn selling_more_than_held_reports_shortfall_without_panicking() {
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        tx(2, Kind::Sell, 1, "BTC", "2024-02-01 10:00", 3.0, 900.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    assert_eq!(out.holdings.len(), 0);
    approx(out.realized[0].amount, 1.0); // only what existed was consumed
    approx(out.realized[0].gain, 800.0); // shortfall has zero basis
}

#[test]
fn transfer_moves_lots_with_original_basis_and_date() {
    let mut t = tx(2, Kind::Transfer, 1, "BTC", "2024-02-01 10:00", 0.5, 0.0);
    t.to_wallet_id = 2;
    t.asset_fee = 0.001;
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        t,
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);

    // Source keeps half, destination receives amount less network fee.
    let a = out.holdings.iter().find(|h| h.wallet == "A").unwrap();
    let b = out.holdings.iter().find(|h| h.wallet == "B").unwrap();
    approx(a.amount, 0.5);
    approx(a.basis, 50.0);
    approx(b.amount, 0.499);
    approx(b.basis, 49.9);

    // Destination lot keeps the ORIGINAL acquisition date (holding period).
    let b_lot = out.tax_lots.iter().find(|l| l.wallet == "B").unwrap();
    assert_eq!(b_lot.ts, "2024-01-01 10:00");
    // Transfers are not taxable events.
    assert!(out.realized.is_empty());
}

#[test]
fn convert_is_a_disposal_plus_an_acquisition() {
    let mut t = tx(2, Kind::Convert, 1, "BTC", "2024-02-01 10:00", 0.5, 60.0);
    t.to_coin = "XMR".into();
    t.amount_to = 3.0;
    t.fee = 1.0; // fiat exchange fee
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        t,
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);

    // Disposal: 0.5 BTC with basis 50, proceeds = FMV 60 → gain 10.
    assert_eq!(out.realized.len(), 1);
    approx(out.realized[0].gain, 10.0);

    // Acquisition: XMR basis = FMV + fee = 61 across 3 coins.
    let xmr = out.holdings.iter().find(|h| h.coin == "XMR").unwrap();
    approx(xmr.amount, 3.0);
    approx(xmr.basis, 61.0);
    approx(xmr.avg_cost, 61.0 / 3.0);
}

#[test]
fn spend_uses_entered_fiat_value_as_proceeds() {
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        tx(2, Kind::Spend, 1, "BTC", "2024-02-01 10:00", 0.1, 90.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    approx(out.realized[0].proceeds, 90.0);
    approx(out.realized[0].basis, 10.0);
    approx(out.realized[0].gain, 80.0);
}

#[test]
fn transactions_are_replayed_in_chronological_order() {
    // Entered out of order; a sale before the purchase that funds it
    // would find no lots if replayed naively in insertion order.
    let txs = vec![
        tx(1, Kind::Sell, 1, "BTC", "2024-03-01 10:00", 1.0, 250.0),
        tx(2, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    approx(out.realized[0].basis, 100.0);
    approx(out.realized[0].gain, 150.0);
}

#[test]
fn lots_are_isolated_per_wallet() {
    // A sale in wallet A must not touch wallet B's coins.
    let txs = vec![
        tx(1, Kind::Purchase, 1, "BTC", "2024-01-01 10:00", 1.0, 100.0),
        tx(2, Kind::Purchase, 2, "BTC", "2024-01-02 10:00", 2.0, 500.0),
        tx(3, Kind::Sell, 1, "BTC", "2024-02-01 10:00", 1.0, 250.0),
    ];
    let out = compute_ledger(&txs, &names(), Method::Fifo);
    let b = out.holdings.iter().find(|h| h.wallet == "B").unwrap();
    approx(b.amount, 2.0);
    approx(b.basis, 500.0);
}
