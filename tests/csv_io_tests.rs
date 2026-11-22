use std::collections::HashMap;

use crypto_inventory::csv_io::{parse_csv_line, realized_csv, taxlots_csv, transactions_csv};
use crypto_inventory::model::{Kind, Realized, TaxLot, Tx};

fn sample_tx(description: &str) -> Tx {
    Tx {
        id: 1,
        kind: Kind::Purchase,
        wallet_id: 1,
        to_wallet_id: 0,
        coin: "BTC".into(),
        to_coin: String::new(),
        ts: "2024-01-01 10:00".into(),
        exchange: "Some Exchange".into(),
        amount: 0.5,
        amount_to: 0.0,
        fee: 1.0,
        asset_fee: 0.0,
        unit_price: 1000.0,
        fiat_total: 501.0,
        description: description.into(),
        fiat_value: 0.0,
    }
}

#[test]
fn parse_handles_plain_fields() {
    let cols = parse_csv_line("a,b,c");
    assert_eq!(cols, vec!["a", "b", "c"]);
}

#[test]
fn parse_handles_quoted_commas_and_escaped_quotes() {
    let cols = parse_csv_line(r#""hello, world","say ""hi""",plain"#);
    assert_eq!(cols[0], "hello, world");
    assert_eq!(cols[1], "say \"hi\"");
    assert_eq!(cols[2], "plain");
}

#[test]
fn transactions_csv_has_header_and_one_row_per_tx() {
    let names = HashMap::from([(1, "Main Wallet".to_string())]);
    let txs = vec![sample_tx("groceries, weekly"), sample_tx("plane ticket")];
    let s = transactions_csv(&txs, &names);
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 3); // header + 2
    assert!(lines[0].starts_with("date,type,coin,wallet"));
    assert!(lines[0].contains("description"));
    assert!(lines[0].split(',').count() >= 15);
}

#[test]
fn transactions_csv_round_trips_through_the_parser() {
    // A description with a comma is quoted on export and must
    // come back as ONE field on import.
    let names = HashMap::from([(1, "Main".to_string())]);
    let txs = vec![sample_tx("coffee, large")];
    let s = transactions_csv(&txs, &names);
    let data_line = s.lines().nth(1).unwrap();
    let cols = parse_csv_line(data_line);
    assert_eq!(cols.len(), 15);
    assert_eq!(cols[13], "coffee, large");
}

#[test]
fn realized_csv_serializes_each_event() {
    let realized = vec![Realized {
        kind: Kind::Sell,
        ts: "2024-02-01 10:00".into(),
        coin: "BTC".into(),
        wallet: "Main".into(),
        amount: 0.5,
        proceeds: 750.0,
        basis: 500.0,
        gain: 250.0,
    }];
    let s = realized_csv(&realized);
    assert_eq!(s.lines().count(), 2);
    assert!(s.contains("Sell"));
    assert!(s.contains("250.00"));
}

#[test]
fn taxlots_csv_serializes_each_lot() {
    let lots = vec![TaxLot {
        coin: "BTC".into(),
        wallet: "Main".into(),
        ts: "2024-01-01 10:00".into(),
        amount: 1.0,
        unit_cost: 100.0,
    }];
    let s = taxlots_csv(&lots);
    assert_eq!(s.lines().count(), 2);
    assert!(s.contains("100.00"));
}
