//! Application state and logic. Owns the database connection and the
//! loaded data; delegates computation to [`crate::ledger`] and
//! serialization to [`crate::csv_io`]. Drawing lives in [`crate::ui`].

use std::collections::HashMap;
use std::fs;

use chrono::Local;
use eframe::egui;
use rusqlite::{Connection, params};

use crate::csv_io;
use crate::db;
use crate::ledger;
use crate::model::{Kind, Method, Tx, Wallet, fmt8};

/// Which panel is currently shown.
#[derive(PartialEq)]
pub enum View {
    AddTransaction,
    Transactions,
    Portfolio,
    TaxLots,
    Wallets,
    Settings,
    Export,
    Import,
}

pub struct App {
    pub db: Connection,
    pub wallets: Vec<Wallet>,
    pub txs: Vec<Tx>,
    pub holdings: Vec<crate::model::Holding>,
    pub realized: Vec<crate::model::Realized>,
    pub tax_lots: Vec<crate::model::TaxLot>,
    pub method: Method,
    pub view: View,

    // new transaction form
    pub f_kind: Kind,
    pub f_wallet_idx: usize,
    pub f_coin: String,
    pub f_ts: String,
    pub f_exchange: String,
    pub f_amount: String,
    pub f_fee: String,
    pub f_price: String,
    pub f_desc: String,
    pub f_fiat_value: String,

    // price entry mode for purchases AND sells:
    // false: user knows the exchange rate (the other side is derived)
    // true:  user knows the fiat total (purchase: total paid incl. fee;
    //        sell: total received net of fee) — the rate is derived
    pub f_enter_total: bool,
    pub f_total: String,

    // transfer form
    pub t_to_wallet_idx: usize,
    pub t_asset_fee: String,

    // convert form
    pub c_to_coin: String,
    pub c_amount_to: String,
    pub c_fmv: String,

    // wallet form
    pub w_name: String,
    pub w_address: String,

    // import form
    pub import_path: String,
    pub import_report: String,

    pub status: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let database = db::open_db().expect("failed to open crypto_tracker.db");

        let stored_method: Option<String> = database
            .query_row(
                "SELECT value FROM settings WHERE key='inventory_method'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok();

        let mut app = App {
            db: database,
            wallets: Vec::new(),
            txs: Vec::new(),
            holdings: Vec::new(),
            realized: Vec::new(),
            tax_lots: Vec::new(),
            method: Method::from_db(stored_method.as_deref().unwrap_or("FIFO")),
            view: View::Portfolio,
            f_kind: Kind::Purchase,
            f_wallet_idx: 0,
            f_coin: "BTC".to_string(),
            f_ts: Local::now().format("%Y-%m-%d %H:%M").to_string(),
            f_exchange: String::new(),
            f_amount: String::new(),
            f_fee: String::new(),
            f_price: String::new(),
            f_desc: String::new(),
            f_fiat_value: String::new(),
            f_enter_total: false,
            f_total: String::new(),
            t_to_wallet_idx: 0,
            t_asset_fee: String::new(),
            c_to_coin: "XMR".to_string(),
            c_amount_to: String::new(),
            c_fmv: String::new(),
            w_name: String::new(),
            w_address: String::new(),
            import_path: String::new(),
            import_report: String::new(),
            status: String::new(),
        };
        app.load_all();
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        app
    }

    pub fn load_all(&mut self) {
        self.wallets = self
            .db
            .prepare("SELECT id, name, address FROM wallets ORDER BY name")
            .and_then(|mut s| {
                s.query_map([], |r| {
                    Ok(Wallet {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        address: r.get(2)?,
                    })
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        self.txs = self
            .db
            .prepare(
                "SELECT id, tx_type, wallet_id, to_wallet_id, coin, to_coin, ts,
                        exchange, amount, amount_to, fee, asset_fee, unit_price,
                        fiat_total, description, fiat_value
                 FROM transactions",
            )
            .and_then(|mut s| {
                s.query_map([], |r| {
                    Ok(Tx {
                        id: r.get(0)?,
                        kind: Kind::from_db(&r.get::<_, String>(1)?),
                        wallet_id: r.get(2)?,
                        to_wallet_id: r.get(3)?,
                        coin: r.get(4)?,
                        to_coin: r.get(5)?,
                        ts: r.get(6)?,
                        exchange: r.get(7)?,
                        amount: r.get(8)?,
                        amount_to: r.get(9)?,
                        fee: r.get(10)?,
                        asset_fee: r.get(11)?,
                        unit_price: r.get(12)?,
                        fiat_total: r.get(13)?,
                        description: r.get(14)?,
                        fiat_value: r.get(15)?,
                    })
                })
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();
        self.rebuild();
    }

    /// Recompute lots, holdings and realized gains by replaying
    /// all transactions in chronological order.
    pub fn rebuild(&mut self) {
        let names: HashMap<i64, String> = self
            .wallets
            .iter()
            .map(|w| (w.id, w.name.clone()))
            .collect();
        let outcome = ledger::compute_ledger(&self.txs, &names, self.method);
        self.holdings = outcome.holdings;
        self.tax_lots = outcome.tax_lots;
        self.realized = outcome.realized;
        self.txs = outcome.sorted_txs;
    }

    pub fn clear_form(&mut self) {
        for f in [
            &mut self.f_amount,
            &mut self.f_fee,
            &mut self.f_price,
            &mut self.f_total,
            &mut self.f_desc,
            &mut self.f_fiat_value,
            &mut self.f_exchange,
            &mut self.t_asset_fee,
            &mut self.c_amount_to,
            &mut self.c_fmv,
        ] {
            f.clear();
        }
        self.f_ts = Local::now().format("%Y-%m-%d %H:%M").to_string();
    }

    pub fn add_tx(&mut self) {
        if self.wallets.is_empty() {
            self.status = "Add a wallet first.".into();
            return;
        }
        match self.f_kind {
            Kind::Transfer => {
                self.insert_transfer();
                return;
            }
            Kind::Convert => {
                self.insert_convert();
                return;
            }
            _ => {}
        }

        let wallet_id = self.wallets[self.f_wallet_idx.min(self.wallets.len() - 1)].id;
        let parse = |s: &str| s.trim().parse::<f64>().ok();
        let amount = parse(&self.f_amount).unwrap_or(0.0);
        let fee = parse(&self.f_fee).unwrap_or(0.0);
        let price = parse(&self.f_price).unwrap_or(0.0);
        let total_input = parse(&self.f_total).unwrap_or(0.0);
        let fiat_value = parse(&self.f_fiat_value).unwrap_or(0.0);
        let coin = self.f_coin.trim().to_uppercase();
        let ts = self.f_ts.trim().to_string();

        if coin.is_empty() || ts.is_empty() || amount <= 0.0 {
            self.status = "Coin, date and a positive amount are required.".into();
            return;
        }

        let (exchange, unit_price, fiat_total, description, fv) = match self.f_kind {
            Kind::Purchase => {
                let fee_v = fee.max(0.0);
                if self.f_enter_total {
                    // Total fiat paid, assumed to INCLUDE the fee.
                    if total_input <= 0.0 {
                        self.status = "Enter the total fiat paid for this purchase.".into();
                        return;
                    }
                    let net = (total_input - fee_v).max(0.0);
                    (
                        self.f_exchange.trim().to_string(),
                        net / amount,
                        total_input,
                        String::new(),
                        0.0,
                    )
                } else {
                    if price <= 0.0 {
                        self.status = "Exchange rate (fiat per coin) is required.".into();
                        return;
                    }
                    (
                        self.f_exchange.trim().to_string(),
                        price,
                        amount * price + fee_v,
                        String::new(),
                        0.0,
                    )
                }
            }
            Kind::Sell => {
                let fee_v = fee.max(0.0);
                if self.f_enter_total {
                    // Total fiat received, assumed to be NET of the fee.
                    if total_input <= 0.0 {
                        self.status = "Enter the total fiat received for this sale.".into();
                        return;
                    }
                    (
                        self.f_exchange.trim().to_string(),
                        (total_input + fee_v) / amount,
                        total_input,
                        String::new(),
                        0.0,
                    )
                } else {
                    if price <= 0.0 {
                        self.status = "Exchange rate (fiat per coin) is required.".into();
                        return;
                    }
                    (
                        self.f_exchange.trim().to_string(),
                        price,
                        amount * price - fee_v,
                        String::new(),
                        0.0,
                    )
                }
            }
            Kind::Spend => {
                if fiat_value <= 0.0 {
                    self.status =
                        "For a spend, enter the fiat value of the crypto spent (proceeds).".into();
                    return;
                }
                (
                    String::new(),
                    0.0,
                    0.0,
                    self.f_desc.trim().to_string(),
                    fiat_value,
                )
            }
            Kind::Transfer | Kind::Convert => unreachable!(),
        };

        if self.f_kind == Kind::Spend && description.is_empty() {
            self.status = "Describe what was purchased.".into();
            return;
        }

        let r = self.db.execute(
            "INSERT INTO transactions
             (tx_type, wallet_id, coin, ts, exchange, amount, fee, unit_price,
              fiat_total, description, fiat_value)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                self.f_kind.db_key(),
                wallet_id,
                coin,
                ts,
                exchange,
                amount,
                fee.max(0.0),
                unit_price,
                fiat_total,
                description,
                fv
            ],
        );
        match r {
            Ok(_) => {
                self.status = "Transaction saved.".into();
                self.clear_form();
                self.load_all();
            }
            Err(e) => self.status = format!("DB error: {e}"),
        }
    }

    fn insert_transfer(&mut self) {
        if self.wallets.len() < 2 {
            self.status = "A transfer needs two different wallets.".into();
            return;
        }
        let from_idx = self.f_wallet_idx.min(self.wallets.len() - 1);
        let mut to_idx = self.t_to_wallet_idx.min(self.wallets.len() - 1);
        if to_idx == from_idx {
            to_idx = if from_idx == 0 { 1 } else { 0 };
        }
        let from_id = self.wallets[from_idx].id;
        let to_id = self.wallets[to_idx].id;
        let parse = |s: &str| s.trim().parse::<f64>().ok();
        let amount = parse(&self.f_amount).unwrap_or(0.0);
        let asset_fee = parse(&self.t_asset_fee).unwrap_or(0.0);
        let coin = self.f_coin.trim().to_uppercase();
        let ts = self.f_ts.trim().to_string();

        if from_id == to_id {
            self.status = "Source and destination wallets must differ.".into();
            return;
        }
        if coin.is_empty() || ts.is_empty() || amount <= 0.0 {
            self.status = "Coin, date and a positive amount are required.".into();
            return;
        }
        if !(0.0..amount).contains(&asset_fee) {
            self.status = "Network fee must be between 0 and the transferred amount.".into();
            return;
        }

        let r = self.db.execute(
            "INSERT INTO transactions
             (tx_type, wallet_id, to_wallet_id, coin, ts, amount, asset_fee)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                Kind::Transfer.db_key(),
                from_id,
                to_id,
                coin,
                ts,
                amount,
                asset_fee
            ],
        );
        match r {
            Ok(_) => {
                self.status = "Transfer saved.".into();
                self.clear_form();
                self.load_all();
            }
            Err(e) => self.status = format!("DB error: {e}"),
        }
    }

    fn insert_convert(&mut self) {
        let wallet_id = self.wallets[self.f_wallet_idx.min(self.wallets.len() - 1)].id;
        let parse = |s: &str| s.trim().parse::<f64>().ok();
        let amount = parse(&self.f_amount).unwrap_or(0.0);
        let amount_to = parse(&self.c_amount_to).unwrap_or(0.0);
        let fmv = parse(&self.c_fmv).unwrap_or(0.0);
        let fee = parse(&self.f_fee).unwrap_or(0.0);
        let coin = self.f_coin.trim().to_uppercase();
        let to_coin = self.c_to_coin.trim().to_uppercase();
        let ts = self.f_ts.trim().to_string();

        if coin.is_empty() || to_coin.is_empty() || coin == to_coin {
            self.status = "From-coin and to-coin must be two different coins.".into();
            return;
        }
        if ts.is_empty() || amount <= 0.0 || amount_to <= 0.0 || fmv <= 0.0 {
            self.status =
                "Date, amount given up, amount received and fiat value are all required.".into();
            return;
        }

        let r = self.db.execute(
            "INSERT INTO transactions
             (tx_type, wallet_id, coin, to_coin, ts, exchange, amount, amount_to,
              fee, fiat_value)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                Kind::Convert.db_key(),
                wallet_id,
                coin,
                to_coin,
                ts,
                self.f_exchange.trim(),
                amount,
                amount_to,
                fee.max(0.0),
                fmv
            ],
        );
        match r {
            Ok(_) => {
                self.status = "Conversion saved.".into();
                self.clear_form();
                self.load_all();
            }
            Err(e) => self.status = format!("DB error: {e}"),
        }
    }

    pub fn delete_tx(&mut self, id: i64) {
        if self
            .db
            .execute("DELETE FROM transactions WHERE id=?1", params![id])
            .is_ok()
        {
            self.load_all();
            self.status = "Transaction deleted (ledger recomputed).".into();
        }
    }

    pub fn add_wallet(&mut self) {
        let name = self.w_name.trim().to_string();
        if name.is_empty() {
            self.status = "Wallet name is required.".into();
            return;
        }
        match self.db.execute(
            "INSERT INTO wallets (name, address) VALUES (?1, ?2)",
            params![name, self.w_address.trim()],
        ) {
            Ok(_) => {
                self.w_name.clear();
                self.w_address.clear();
                self.status = "Wallet added.".into();
                self.load_all();
            }
            Err(e) => self.status = format!("DB error: {e}"),
        }
    }

    pub fn set_method(&mut self, m: Method) {
        self.method = m;
        let _ = self.db.execute(
            "INSERT INTO settings (key, value) VALUES ('inventory_method', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![m.db_key()],
        );
        self.rebuild();
        self.status = "Inventory method applied. Ledger recomputed.".into();
    }

    // ---------- CSV export ----------

    pub fn export_csv(&mut self, which: &str) {
        let stamp = Local::now().format("%Y%m%d_%H%M%S");
        let names: HashMap<i64, String> = self
            .wallets
            .iter()
            .map(|w| (w.id, w.name.clone()))
            .collect();
        let (fname, content) = match which {
            "transactions" => (
                format!("transactions_{stamp}.csv"),
                csv_io::transactions_csv(&self.txs, &names),
            ),
            "realized" => (
                format!("realized_gains_{stamp}.csv"),
                csv_io::realized_csv(&self.realized),
            ),
            _ => (
                format!("tax_lots_{stamp}.csv"),
                csv_io::taxlots_csv(&self.tax_lots),
            ),
        };
        match fs::write(&fname, content) {
            Ok(_) => self.status = format!("Exported to {fname} (app's working directory)."),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }

    // ---------- CSV import ----------

    pub fn import_csv(&mut self) {
        self.import_report.clear();
        let path = self.import_path.trim().to_string();
        if path.is_empty() {
            self.status = "Select a CSV file first.".into();
            return;
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("Could not read file: {e}");
                return;
            }
        };

        // Wallet name -> id, resolved from current wallets.
        let mut wallet_ids: HashMap<String, i64> = HashMap::new();
        for w in &self.wallets {
            wallet_ids.insert(w.name.clone(), w.id);
        }

        let mut inserted = 0usize;
        let mut skipped: Vec<String> = Vec::new();
        for (lineno, line) in content.lines().enumerate() {
            if lineno == 0 || line.trim().is_empty() {
                continue; // header row / blanks
            }
            let cols = csv_io::parse_csv_line(line);
            if cols.len() < 15 {
                skipped.push(format!("line {}: wrong column count", lineno + 1));
                continue;
            }
            let trim = |s: &String| s.trim().to_string();
            let kind = match cols[1].trim() {
                "Purchase" => Kind::Purchase,
                "Sell" => Kind::Sell,
                "Spend" => Kind::Spend,
                "Transfer" => Kind::Transfer,
                "Convert" => Kind::Convert,
                other => {
                    skipped.push(format!("line {}: unknown type '{other}'", lineno + 1));
                    continue;
                }
            };
            let coin = trim(&cols[2]).to_uppercase();
            let wallet_id = match wallet_ids.get(cols[3].trim()) {
                Some(id) => *id,
                None => {
                    skipped.push(format!(
                        "line {}: unknown wallet '{}'",
                        lineno + 1,
                        cols[3].trim()
                    ));
                    continue;
                }
            };
            let to_wallet_id = if cols[4].trim().is_empty() {
                0
            } else {
                match wallet_ids.get(cols[4].trim()) {
                    Some(id) => *id,
                    None => {
                        skipped.push(format!(
                            "line {}: unknown to-wallet '{}'",
                            lineno + 1,
                            cols[4].trim()
                        ));
                        continue;
                    }
                }
            };
            let num = |i: usize| cols[i].trim().parse::<f64>().unwrap_or(0.0);
            let amount = num(6);
            let ts = trim(&cols[0]);
            if coin.is_empty() || amount <= 0.0 || ts.is_empty() {
                skipped.push(format!("line {}: missing date, coin or amount", lineno + 1));
                continue;
            }

            let res = self.db.execute(
                "INSERT INTO transactions
                 (tx_type, wallet_id, to_wallet_id, coin, to_coin, ts, exchange,
                  amount, amount_to, fee, asset_fee, unit_price, fiat_total,
                  description, fiat_value)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
                params![
                    kind.db_key(),
                    wallet_id,
                    to_wallet_id,
                    coin,
                    trim(&cols[5]),
                    ts,
                    trim(&cols[8]),
                    amount,
                    num(7),
                    num(9),
                    num(10),
                    num(11),
                    num(12),
                    trim(&cols[13]),
                    num(14),
                ],
            );
            match res {
                Ok(_) => inserted += 1,
                Err(e) => skipped.push(format!("line {}: DB error {e}", lineno + 1)),
            }
        }
        self.load_all();
        let mut report = format!("Imported {inserted} transaction(s) from {path}.");
        if !skipped.is_empty() {
            report.push_str("\nSkipped rows:");
            for s in &skipped {
                report.push_str(&format!("\n  • {s}"));
            }
        }
        self.import_report = report;
        self.status = format!(
            "Import finished: {inserted} added, {} skipped.",
            skipped.len()
        );
    }

    /// Convenience helper used by several panels.
    pub fn wallet_names(&self) -> HashMap<i64, String> {
        self.wallets
            .iter()
            .map(|w| (w.id, w.name.clone()))
            .collect()
    }

    /// Currently unused numerics formatter re-export guard.
    #[allow(dead_code)]
    fn _fmt8_guard(v: f64) -> String {
        fmt8(v)
    }
}
