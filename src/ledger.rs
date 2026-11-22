//! The inventory engine. Given a chronologically ordered set of
//! transactions, replays them to derive open tax lots, holdings and
//! realized gain/loss. Pure computation — no database, no GUI.

use std::collections::HashMap;

use crate::model::{Holding, Kind, Lot, Method, Realized, TaxLot, Tx};

/// Everything the replay produces.
pub struct LedgerOutcome {
    pub holdings: Vec<Holding>,
    pub tax_lots: Vec<TaxLot>,
    pub realized: Vec<Realized>,
    pub sorted_txs: Vec<Tx>,
}

/// Remove up to `want` coins from `lots` following the method.
/// Returns the individual lots removed (with their original unit
/// costs and acquisition dates intact).
pub fn take_lots(lots: &mut Vec<Lot>, mut want: f64, method: Method) -> Vec<Lot> {
    let mut out = Vec::new();
    while want > 1e-12 && !lots.is_empty() {
        let idx = match method {
            Method::Fifo => 0,
            Method::Lifo => lots.len() - 1,
            Method::Hifo => lots
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.unit_cost.total_cmp(&b.1.unit_cost))
                .map(|(i, _)| i)
                .unwrap_or(0),
        };
        let take = want.min(lots[idx].amount);
        let moved = Lot {
            amount: take,
            unit_cost: lots[idx].unit_cost,
            ts: lots[idx].ts.clone(),
        };
        lots[idx].amount -= take;
        want -= take;
        out.push(moved);
        if lots[idx].amount <= 1e-12 {
            lots.remove(idx);
        }
    }
    out
}

/// Aggregate version of [`take_lots`]:
/// returns (basis consumed, coins consumed).
pub fn consume_lots(lots: &mut Vec<Lot>, want: f64, method: Method) -> (f64, f64) {
    let taken = take_lots(lots, want, method);
    (
        taken.iter().map(|l| l.amount * l.unit_cost).sum(),
        taken.iter().map(|l| l.amount).sum(),
    )
}

/// Replay all transactions in chronological order and derive
/// holdings, open tax lots and realized gain/loss.
///
/// Semantics:
/// • Purchase  — creates a lot; unit cost = fiat_total / amount.
/// • Sell      — consumes lots; proceeds = net fiat received.
/// • Spend     — consumes lots; proceeds = entered fiat value.
/// • Convert   — consumes source lots (proceeds = FMV received), then
///               creates a destination lot with basis = FMV + fee.
/// • Transfer  — non-taxable: lots move to the destination wallet with
///               original basis/date; the asset-denominated network fee
///               is shaved off proportionally.
pub fn compute_ledger(
    txs: &[Tx],
    wallet_names: &HashMap<i64, String>,
    method: Method,
) -> LedgerOutcome {
    let mut sorted: Vec<Tx> = txs.to_vec();
    sorted.sort_by(|a, b| a.ts.cmp(&b.ts).then(a.id.cmp(&b.id)));

    let mut lots: HashMap<(i64, String), Vec<Lot>> = HashMap::new();
    let mut holdings = Vec::new();
    let mut realized = Vec::new();
    let mut tax_lots = Vec::new();

    for tx in &sorted {
        match tx.kind {
            Kind::Purchase => {
                let unit_cost = if tx.amount > 0.0 {
                    tx.fiat_total / tx.amount
                } else {
                    0.0
                };
                lots.entry((tx.wallet_id, tx.coin.clone()))
                    .or_default()
                    .push(Lot {
                        amount: tx.amount,
                        unit_cost,
                        ts: tx.ts.clone(),
                    });
            }
            Kind::Sell | Kind::Spend | Kind::Convert => {
                let proceeds = match tx.kind {
                    Kind::Sell => tx.fiat_total,
                    Kind::Spend | Kind::Convert => tx.fiat_value,
                    _ => unreachable!(),
                };
                let wl = lots.entry((tx.wallet_id, tx.coin.clone())).or_default();
                let (basis, taken) = consume_lots(wl, tx.amount, method);
                realized.push(Realized {
                    kind: tx.kind,
                    ts: tx.ts.clone(),
                    coin: tx.coin.clone(),
                    wallet: wallet_names.get(&tx.wallet_id).cloned().unwrap_or_default(),
                    amount: taken,
                    proceeds,
                    basis,
                    gain: proceeds - basis,
                });
                if tx.kind == Kind::Convert && tx.amount_to > 0.0 {
                    let unit_cost = (tx.fiat_value + tx.fee.max(0.0)) / tx.amount_to;
                    lots.entry((tx.wallet_id, tx.to_coin.clone()))
                        .or_default()
                        .push(Lot {
                            amount: tx.amount_to,
                            unit_cost,
                            ts: tx.ts.clone(),
                        });
                }
            }
            Kind::Transfer => {
                let wl_src = lots.entry((tx.wallet_id, tx.coin.clone())).or_default();
                let moved = take_lots(wl_src, tx.amount, method);
                let keep_frac = if tx.amount > 0.0 {
                    ((tx.amount - tx.asset_fee) / tx.amount).max(0.0)
                } else {
                    1.0
                };
                for l in moved {
                    lots.entry((tx.to_wallet_id, tx.coin.clone()))
                        .or_default()
                        .push(Lot {
                            amount: l.amount * keep_frac,
                            unit_cost: l.unit_cost,
                            ts: l.ts,
                        });
                }
            }
        }
    }

    for ((wid, coin), wl) in &lots {
        let amount: f64 = wl.iter().map(|l| l.amount).sum();
        if amount <= 1e-12 {
            continue;
        }
        let basis: f64 = wl.iter().map(|l| l.amount * l.unit_cost).sum();
        holdings.push(Holding {
            coin: coin.clone(),
            wallet: wallet_names.get(wid).cloned().unwrap_or_default(),
            amount,
            avg_cost: basis / amount,
            basis,
        });
        for l in wl {
            if l.amount > 1e-12 {
                tax_lots.push(TaxLot {
                    coin: coin.clone(),
                    wallet: wallet_names.get(wid).cloned().unwrap_or_default(),
                    ts: l.ts.clone(),
                    amount: l.amount,
                    unit_cost: l.unit_cost,
                });
            }
        }
    }
    holdings.sort_by(|a, b| a.coin.cmp(&b.coin).then(a.wallet.cmp(&b.wallet)));
    tax_lots.sort_by(|a, b| {
        a.coin
            .cmp(&b.coin)
            .then(a.wallet.cmp(&b.wallet))
            .then(a.ts.cmp(&b.ts))
    });

    LedgerOutcome {
        holdings,
        tax_lots,
        realized,
        sorted_txs: sorted,
    }
}
