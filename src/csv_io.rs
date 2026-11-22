//! CSV primitives and export builders. Properly handles quoted
//! fields with embedded commas and escaped quotes, so exports
//! round-trip through [`parse_csv_line`].

use std::collections::HashMap;

use crate::model::{Realized, TaxLot, Tx, fmt2, fmt8};

/// Escape a field if it contains commas, quotes or newlines.
pub fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Join fields into one CSV row (with trailing newline).
pub fn csv_row(fields: &[&str]) -> String {
    let mut row = String::new();
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            row.push(',');
        }
        row.push_str(&csv_escape(f));
    }
    row.push('\n');
    row
}

/// Parse one CSV line, handling double-quoted fields with embedded
/// commas and escaped quotes (""). Inverse of csv_escape/csv_row.
pub fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == ',' {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out
}

/// Serialize all transactions. `names` maps wallet ids to display names.
pub fn transactions_csv(txs: &[Tx], names: &HashMap<i64, String>) -> String {
    let mut s = String::new();
    s.push_str(&csv_row(&[
        "date",
        "type",
        "coin",
        "wallet",
        "to_wallet",
        "to_coin",
        "amount",
        "amount_received",
        "exchange",
        "fee_fiat",
        "fee_asset",
        "rate",
        "fiat_total",
        "description",
        "fiat_value",
    ]));
    for t in txs {
        s.push_str(&csv_row(&[
            &t.ts,
            t.kind.label(),
            &t.coin,
            names.get(&t.wallet_id).map(String::as_str).unwrap_or("?"),
            if t.to_wallet_id != 0 {
                names
                    .get(&t.to_wallet_id)
                    .map(String::as_str)
                    .unwrap_or("?")
            } else {
                ""
            },
            &t.to_coin,
            &fmt8(t.amount),
            &fmt8(t.amount_to),
            &t.exchange,
            &fmt2(t.fee),
            &fmt8(t.asset_fee),
            &fmt2(t.unit_price),
            &fmt2(t.fiat_total),
            &t.description,
            &fmt2(t.fiat_value),
        ]));
    }
    s
}

/// Serialize realized gain/loss events.
pub fn realized_csv(realized: &[Realized]) -> String {
    let mut s = String::new();
    s.push_str(&csv_row(&[
        "type",
        "date",
        "coin",
        "wallet",
        "amount",
        "proceeds",
        "cost_basis",
        "gain_loss",
    ]));
    for r in realized {
        s.push_str(&csv_row(&[
            r.kind.label(),
            &r.ts,
            &r.coin,
            &r.wallet,
            &fmt8(r.amount),
            &fmt2(r.proceeds),
            &fmt2(r.basis),
            &fmt2(r.gain),
        ]));
    }
    s
}

/// Serialize remaining open tax lots.
pub fn taxlots_csv(lots: &[TaxLot]) -> String {
    let mut s = String::new();
    s.push_str(&csv_row(&[
        "coin",
        "wallet",
        "acquired",
        "remaining_amount",
        "unit_cost",
        "basis",
    ]));
    for l in lots {
        s.push_str(&csv_row(&[
            &l.coin,
            &l.wallet,
            &l.ts,
            &fmt8(l.amount),
            &fmt2(l.unit_cost),
            &fmt2(l.amount * l.unit_cost),
        ]));
    }
    s
}
