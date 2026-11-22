//! All egui drawing: the navigation bar, every panel, and the
//! `eframe::App` implementation that drives the frame loop.

use std::collections::HashMap;

use eframe::egui;
use rusqlite::params;

use crate::app::{App, View};
use crate::model::{Kind, Method, fmt2, fmt8};

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("nav").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("₿ Crypto Inventory Tracker");
                ui.separator();
                for (v, label) in [
                    (View::Portfolio, "Portfolio"),
                    (View::Transactions, "Transactions"),
                    (View::TaxLots, "Tax Lots"),
                    (View::AddTransaction, "＋ Add Transaction"),
                    (View::Wallets, "Wallets"),
                    (View::Export, "Export"),
                    (View::Import, "Import"),
                    (View::Settings, "Settings"),
                ] {
                    if ui.selectable_label(self.view == v, label).clicked() {
                        self.view = v;
                    }
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if !self.status.is_empty() {
                ui.label(egui::RichText::new(&self.status).weak());
            }
            ui.label(format!("Inventory method: {}", self.method.label()));
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::AddTransaction => self.ui_add(ui),
            View::Transactions => self.ui_transactions(ui),
            View::Portfolio => self.ui_portfolio(ui),
            View::TaxLots => self.ui_taxlots(ui),
            View::Wallets => self.ui_wallets(ui),
            View::Export => self.ui_export(ui),
            View::Import => self.ui_import(ui),
            View::Settings => self.ui_settings(ui),
        });
    }
}

/// Wallet picker combo box, as a free function so callers can pass
/// a wallet-index field without borrow conflicts.
fn wallet_combo(ui: &mut egui::Ui, wallets: &[crate::model::Wallet], label: &str, idx: &mut usize) {
    if *idx >= wallets.len() {
        *idx = 0;
    }
    ui.label(label);
    egui::ComboBox::from_id_source(label)
        .selected_text(if wallets.is_empty() {
            "(none)".to_string()
        } else {
            format!(
                "{}{}",
                wallets[*idx].name,
                if wallets[*idx].address.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", wallets[*idx].address)
                }
            )
        })
        .show_ui(ui, |ui| {
            for (i, w) in wallets.iter().enumerate() {
                ui.selectable_value(idx, i, &w.name);
            }
        });
    ui.end_row();
}

impl App {
    fn ui_add(&mut self, ui: &mut egui::Ui) {
        ui.heading("Add Transaction");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Type:");
            for k in Kind::ALL {
                ui.selectable_value(&mut self.f_kind, k, k.label());
            }
        });
        ui.add_space(6.0);

        egui::Grid::new("addgrid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                if self.wallets.is_empty() {
                    ui.label("Wallet:");
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        "No wallets yet — add one in the Wallets tab.",
                    );
                    return;
                }
                match self.f_kind {
                    Kind::Transfer => {
                        let mut from = self.f_wallet_idx;
                        let mut to = self.t_to_wallet_idx;
                        wallet_combo(ui, &self.wallets, "From wallet:", &mut from);
                        wallet_combo(ui, &self.wallets, "To wallet:", &mut to);
                        self.f_wallet_idx = from;
                        self.t_to_wallet_idx = to;

                        ui.label("Coin (e.g. BTC):");
                        ui.text_edit_singleline(&mut self.f_coin);
                        ui.end_row();

                        ui.label("Date/time (YYYY-MM-DD HH:MM):");
                        ui.text_edit_singleline(&mut self.f_ts);
                        ui.end_row();

                        ui.label("Amount sent:");
                        ui.text_edit_singleline(&mut self.f_amount);
                        ui.end_row();

                        ui.label("Network fee (in the coin itself):");
                        ui.text_edit_singleline(&mut self.t_asset_fee);
                        ui.end_row();
                    }
                    Kind::Convert => {
                        wallet_combo(ui, &self.wallets, "Wallet:", &mut self.f_wallet_idx);

                        ui.label("From coin (e.g. BTC):");
                        ui.text_edit_singleline(&mut self.f_coin);
                        ui.end_row();

                        ui.label("To coin (e.g. XMR):");
                        ui.text_edit_singleline(&mut self.c_to_coin);
                        ui.end_row();

                        ui.label("Date/time (YYYY-MM-DD HH:MM):");
                        ui.text_edit_singleline(&mut self.f_ts);
                        ui.end_row();

                        ui.label("Exchange (optional):");
                        ui.text_edit_singleline(&mut self.f_exchange);
                        ui.end_row();

                        ui.label("Amount of from-coin given up:");
                        ui.text_edit_singleline(&mut self.f_amount);
                        ui.end_row();

                        ui.label("Amount of to-coin received:");
                        ui.text_edit_singleline(&mut self.c_amount_to);
                        ui.end_row();

                        ui.label("Fiat value of to-coin received:");
                        ui.text_edit_singleline(&mut self.c_fmv);
                        ui.end_row();

                        ui.label("Exchange fee (fiat, optional):");
                        ui.text_edit_singleline(&mut self.f_fee);
                        ui.end_row();
                    }
                    _ => {
                        wallet_combo(ui, &self.wallets, "Wallet:", &mut self.f_wallet_idx);

                        ui.label("Coin (e.g. BTC):");
                        ui.text_edit_singleline(&mut self.f_coin);
                        ui.end_row();

                        ui.label("Date/time (YYYY-MM-DD HH:MM):");
                        ui.text_edit_singleline(&mut self.f_ts);
                        ui.end_row();

                        match self.f_kind {
                            Kind::Purchase => {
                                ui.label("Exchange:");
                                ui.text_edit_singleline(&mut self.f_exchange);
                                ui.end_row();
                                ui.label("Amount of crypto bought:");
                                ui.text_edit_singleline(&mut self.f_amount);
                                ui.end_row();
                                ui.label("Price entry:");
                                ui.horizontal(|ui| {
                                    ui.radio_value(
                                        &mut self.f_enter_total,
                                        false,
                                        "I know the exchange rate",
                                    );
                                    ui.radio_value(
                                        &mut self.f_enter_total,
                                        true,
                                        "I know the total fiat paid",
                                    );
                                });
                                ui.end_row();
                                if self.f_enter_total {
                                    ui.label("Total fiat paid (incl. fee):");
                                    ui.text_edit_singleline(&mut self.f_total);
                                    ui.end_row();
                                } else {
                                    ui.label("Exchange rate (fiat per coin):");
                                    ui.text_edit_singleline(&mut self.f_price);
                                    ui.end_row();
                                }
                                ui.label("Exchange fee (fiat):");
                                ui.text_edit_singleline(&mut self.f_fee);
                                ui.end_row();
                            }
                            Kind::Sell => {
                                ui.label("Exchange:");
                                ui.text_edit_singleline(&mut self.f_exchange);
                                ui.end_row();
                                ui.label("Amount of crypto sold:");
                                ui.text_edit_singleline(&mut self.f_amount);
                                ui.end_row();
                                ui.label("Price entry:");
                                ui.horizontal(|ui| {
                                    ui.radio_value(
                                        &mut self.f_enter_total,
                                        false,
                                        "I know the exchange rate",
                                    );
                                    ui.radio_value(
                                        &mut self.f_enter_total,
                                        true,
                                        "I know the total fiat received",
                                    );
                                });
                                ui.end_row();
                                if self.f_enter_total {
                                    ui.label("Total fiat received (net of fee):");
                                    ui.text_edit_singleline(&mut self.f_total);
                                    ui.end_row();
                                } else {
                                    ui.label("Exchange rate (fiat per coin):");
                                    ui.text_edit_singleline(&mut self.f_price);
                                    ui.end_row();
                                }
                                ui.label("Exchange fee (fiat):");
                                ui.text_edit_singleline(&mut self.f_fee);
                                ui.end_row();
                            }
                            Kind::Spend => {
                                ui.label("Item or service purchased:");
                                ui.text_edit_singleline(&mut self.f_desc);
                                ui.end_row();
                                ui.label("Amount of crypto paid:");
                                ui.text_edit_singleline(&mut self.f_amount);
                                ui.end_row();
                                ui.label("Fiat value of crypto spent (proceeds):");
                                ui.text_edit_singleline(&mut self.f_fiat_value);
                                ui.end_row();
                            }
                            Kind::Transfer | Kind::Convert => unreachable!(),
                        }
                    }
                }
            });

        ui.add_space(8.0);
        // live preview / derived counterpart values
        let amount_parsed = self.f_amount.trim().parse::<f64>();
        let fee_parsed = self.f_fee.trim().parse::<f64>().unwrap_or(0.0);
        match self.f_kind {
            Kind::Purchase => {
                if self.f_enter_total {
                    if let (Ok(a), Ok(t)) = (amount_parsed, self.f_total.trim().parse::<f64>()) {
                        if a > 0.0 {
                            ui.label(format!(
                                "Implied exchange rate: {}",
                                fmt2((t - fee_parsed).max(0.0) / a)
                            ));
                        }
                    }
                } else if let (Ok(a), Ok(p)) = (amount_parsed, self.f_price.trim().parse::<f64>()) {
                    ui.label(format!("Total paid: {}", fmt2(a * p + fee_parsed)));
                }
            }
            Kind::Sell => {
                if self.f_enter_total {
                    if let (Ok(a), Ok(t)) = (amount_parsed, self.f_total.trim().parse::<f64>()) {
                        if a > 0.0 {
                            ui.label(format!(
                                "Implied exchange rate: {}",
                                fmt2((t + fee_parsed) / a)
                            ));
                        }
                    }
                } else if let (Ok(a), Ok(p)) = (amount_parsed, self.f_price.trim().parse::<f64>()) {
                    ui.label(format!(
                        "Total received (net of fee): {}",
                        fmt2((a * p - fee_parsed).max(0.0))
                    ));
                }
            }
            Kind::Transfer => {
                if let (Ok(a), Ok(f)) = (amount_parsed, self.t_asset_fee.trim().parse::<f64>()) {
                    ui.label(format!(
                        "Arriving at destination: {}",
                        fmt8((a - f).max(0.0))
                    ));
                }
            }
            Kind::Convert => {
                if let (Ok(at), Ok(v)) = (
                    self.c_amount_to.trim().parse::<f64>(),
                    self.c_fmv.trim().parse::<f64>(),
                ) {
                    if at > 0.0 {
                        ui.label(format!(
                            "Effective rate: {} fiat per to-coin (basis incl. fee)",
                            fmt2((v + fee_parsed) / at)
                        ));
                    }
                }
            }
            Kind::Spend => {}
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Save transaction").clicked() {
                self.add_tx();
            }
            if ui.button("Clear fields").clicked() {
                self.clear_form();
                self.status = "Form cleared.".into();
            }
        });
    }

    fn ui_transactions(&mut self, ui: &mut egui::Ui) {
        ui.heading("Transactions");
        ui.separator();
        if self.txs.is_empty() {
            ui.weak("No transactions yet.");
            return;
        }
        let names: HashMap<i64, String> = self.wallet_names();
        let txs = self.txs.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("txtable")
                .striped(true)
                .num_columns(7)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Date");
                    ui.strong("Type");
                    ui.strong("Coin");
                    ui.strong("Wallet");
                    ui.strong("Amount");
                    ui.strong("Details");
                    ui.strong("");
                    ui.end_row();
                    for t in &txs {
                        ui.label(&t.ts);
                        ui.label(t.kind.label());
                        ui.label(if t.to_coin.is_empty() {
                            t.coin.clone()
                        } else {
                            format!("{} → {}", t.coin, t.to_coin)
                        });
                        let wallet_label = names
                            .get(&t.wallet_id)
                            .cloned()
                            .unwrap_or_else(|| "?".into());
                        if t.to_wallet_id != 0 {
                            ui.label(format!(
                                "{} → {}",
                                wallet_label,
                                names
                                    .get(&t.to_wallet_id)
                                    .map(String::as_str)
                                    .unwrap_or("?")
                            ));
                        } else {
                            ui.label(wallet_label);
                        }
                        ui.label(fmt8(t.amount));
                        let details = match t.kind {
                            Kind::Purchase => format!(
                                "{} @ {} = {} (fee {})",
                                t.exchange,
                                fmt2(t.unit_price),
                                fmt2(t.fiat_total),
                                fmt2(t.fee)
                            ),
                            Kind::Sell => format!(
                                "{} @ {} = {} net (fee {})",
                                t.exchange,
                                fmt2(t.unit_price),
                                fmt2(t.fiat_total),
                                fmt2(t.fee)
                            ),
                            Kind::Spend => {
                                format!("{} — fiat value {}", t.description, fmt2(t.fiat_value))
                            }
                            Kind::Transfer => {
                                format!("network fee {} {}", fmt8(t.asset_fee), t.coin)
                            }
                            Kind::Convert => format!(
                                "received {} {}, fiat value {} (fee {})",
                                fmt8(t.amount_to),
                                t.to_coin,
                                fmt2(t.fiat_value),
                                fmt2(t.fee)
                            ),
                        };
                        ui.label(details);
                        let id = t.id;
                        if ui.button("Delete").clicked() {
                            self.delete_tx(id);
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn ui_portfolio(&mut self, ui: &mut egui::Ui) {
        ui.heading("Portfolio — Holdings");
        ui.separator();
        if self.holdings.is_empty() {
            ui.weak("No holdings. Record purchases (and wallets) first.");
        } else {
            egui::Grid::new("holdings")
                .striped(true)
                .num_columns(5)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Coin");
                    ui.strong("Wallet");
                    ui.strong("Amount");
                    ui.strong("Avg cost / coin");
                    ui.strong("Cost basis");
                    ui.end_row();
                    for h in &self.holdings {
                        ui.label(&h.coin);
                        ui.label(&h.wallet);
                        ui.label(fmt8(h.amount));
                        ui.label(fmt2(h.avg_cost));
                        ui.label(fmt2(h.basis));
                        ui.end_row();
                    }
                    let basis: f64 = self.holdings.iter().map(|h| h.basis).sum();
                    ui.label("");
                    ui.label("");
                    ui.strong("Total cost basis");
                    ui.label("");
                    ui.strong(fmt2(basis));
                    ui.end_row();
                });
        }

        ui.add_space(14.0);
        ui.heading("Realized Gain / Loss");
        ui.separator();
        if self.realized.is_empty() {
            ui.weak("No sales, spends or conversions recorded yet.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("realized")
                    .striped(true)
                    .num_columns(8)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        ui.strong("Type");
                        ui.strong("Date");
                        ui.strong("Coin");
                        ui.strong("Wallet");
                        ui.strong("Amount");
                        ui.strong("Proceeds");
                        ui.strong("Cost basis");
                        ui.strong("Gain/Loss");
                        ui.end_row();
                        let mut total_gain = 0.0;
                        for r in &self.realized {
                            ui.label(r.kind.label());
                            ui.label(&r.ts);
                            ui.label(&r.coin);
                            ui.label(&r.wallet);
                            ui.label(fmt8(r.amount));
                            ui.label(fmt2(r.proceeds));
                            ui.label(fmt2(r.basis));
                            let txt = if r.gain >= 0.0 {
                                egui::RichText::new(format!("+{}", fmt2(r.gain)))
                                    .color(egui::Color32::LIGHT_GREEN)
                            } else {
                                egui::RichText::new(fmt2(r.gain)).color(egui::Color32::LIGHT_RED)
                            };
                            ui.label(txt);
                            ui.end_row();
                            total_gain += r.gain;
                        }
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.strong(
                            egui::RichText::new(if total_gain >= 0.0 {
                                format!("Total: +{}", fmt2(total_gain))
                            } else {
                                format!("Total: {}", fmt2(total_gain))
                            })
                            .color(if total_gain >= 0.0 {
                                egui::Color32::LIGHT_GREEN
                            } else {
                                egui::Color32::LIGHT_RED
                            }),
                        );
                        ui.end_row();
                    });
            });
        }
    }

    fn ui_taxlots(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tax Lots — Remaining (Open) Lots");
        ui.separator();
        ui.weak(format!(
            "Consumption order: {}. Basis is kept per lot; transfers carry each lot's original basis and acquisition date to the destination wallet.",
            self.method.label()
        ));
        ui.add_space(6.0);
        if self.tax_lots.is_empty() {
            ui.weak("No open lots.");
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("taxlots")
                .striped(true)
                .num_columns(6)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Coin");
                    ui.strong("Wallet");
                    ui.strong("Acquired");
                    ui.strong("Remaining amount");
                    ui.strong("Unit cost");
                    ui.strong("Basis");
                    ui.end_row();
                    let mut total_basis = 0.0;
                    for l in &self.tax_lots {
                        ui.label(&l.coin);
                        ui.label(&l.wallet);
                        ui.label(&l.ts);
                        ui.label(fmt8(l.amount));
                        ui.label(fmt2(l.unit_cost));
                        let b = l.amount * l.unit_cost;
                        total_basis += b;
                        ui.label(fmt2(b));
                        ui.end_row();
                    }
                    ui.label("");
                    ui.label("");
                    ui.label("");
                    ui.label("");
                    ui.strong("Total basis");
                    ui.strong(fmt2(total_basis));
                    ui.end_row();
                });
        });
    }

    fn ui_export(&mut self, ui: &mut egui::Ui) {
        ui.heading("CSV Export");
        ui.separator();
        ui.label("Files are written to the app's working directory with a timestamp in the name.");
        ui.add_space(8.0);
        if ui.button("Export transactions (.csv)").clicked() {
            self.export_csv("transactions");
        }
        if ui.button("Export realized gains (.csv)").clicked() {
            self.export_csv("realized");
        }
        if ui.button("Export tax lots (.csv)").clicked() {
            self.export_csv("taxlots");
        }
        ui.add_space(10.0);
        ui.collapsing("Column notes", |ui| {
            ui.label("• Transactions: one row per record; transfer/convert columns are blank for other types.");
            ui.label("• Realized gains: one row per disposal event (sells, spends, conversions).");
            ui.label("• Tax lots: one row per remaining open lot, basis included.");
        });
    }

    fn ui_import(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import Transactions (CSV)");
        ui.separator();
        ui.label(
            "Imports the CSV format produced by the Export tab (15 columns, header row expected).",
        );
        ui.label("Wallets must already exist — rows referencing unknown wallets are skipped and listed below.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Browse…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSV files", &["csv"])
                    .set_title("Select transactions CSV to import")
                    .pick_file()
                {
                    self.import_path = path.display().to_string();
                }
            }
            if self.import_path.is_empty() {
                ui.weak("No file selected");
            } else {
                ui.monospace(&self.import_path);
            }
        });

        ui.add_space(4.0);
        egui::Grid::new("importgrid")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Path to CSV file (editable):");
                ui.text_edit_singleline(&mut self.import_path);
                ui.end_row();
            });

        ui.add_space(6.0);
        if ui.button("Import").clicked() {
            self.import_csv();
        }
        if !self.import_report.is_empty() {
            ui.add_space(10.0);
            ui.separator();
            ui.label(&self.import_report);
        }
    }

    fn ui_wallets(&mut self, ui: &mut egui::Ui) {
        ui.heading("Wallets");
        ui.separator();
        egui::Grid::new("wallets")
            .striped(true)
            .num_columns(4)
            .spacing([14.0, 4.0])
            .show(ui, |ui| {
                ui.strong("Name");
                ui.strong("Address");
                ui.strong("Coins held");
                ui.strong("");
                ui.end_row();
                for w in self.wallets.clone() {
                    ui.label(&w.name);
                    ui.monospace(&w.address);
                    let coins: Vec<&str> = self
                        .holdings
                        .iter()
                        .filter(|h| {
                            self.wallets
                                .iter()
                                .find(|x| x.name == h.wallet)
                                .map(|x| x.id == w.id)
                                .unwrap_or(false)
                        })
                        .map(|h| h.coin.as_str())
                        .collect();
                    ui.label(if coins.is_empty() {
                        "—".to_string()
                    } else {
                        coins.join(", ")
                    });
                    if ui.button("Delete").clicked() {
                        if self
                            .txs
                            .iter()
                            .any(|t| t.wallet_id == w.id || t.to_wallet_id == w.id)
                        {
                            self.status = "Cannot delete: wallet has transactions.".into();
                        } else {
                            let _ = self
                                .db
                                .execute("DELETE FROM wallets WHERE id=?1", params![w.id]);
                            self.load_all();
                        }
                    }
                    ui.end_row();
                }
            });

        ui.add_space(14.0);
        ui.heading("Add wallet");
        egui::Grid::new("addwallet")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.w_name);
                ui.end_row();
                ui.label("Address (optional):");
                ui.text_edit_singleline(&mut self.w_address);
                ui.end_row();
            });
        if ui.button("Add wallet").clicked() {
            self.add_wallet();
        }
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();
        ui.label(
            "Inventory method (order in which cost-basis lots are consumed by sells, spends, conversions and outgoing transfers):",
        );
        ui.add_space(4.0);
        let mut chosen = self.method;
        for m in Method::ALL {
            ui.radio_value(&mut chosen, m, m.label());
        }
        ui.add_space(6.0);
        if chosen != self.method || ui.button("Apply method").clicked() {
            self.set_method(chosen);
        }
        ui.add_space(12.0);
        ui.collapsing("Notes", |ui| {
            ui.label("• Changing the method does not change holdings or balances, only which lots are consumed (and therefore realized gain/loss).");
            ui.label("• Disposals consume lots only from the wallet the transaction happened in.");
            ui.label("• Transfers between wallets are non-taxable: lots keep their original cost basis and acquisition date; the network fee is shaved off proportionally and is not treated as a disposal.");
            ui.label("• Conversions (e.g. BTC → XMR) are treated as a disposal plus an acquisition: proceeds = fiat value of the coin received, and the new lot's basis = that value plus the exchange fee.");
            ui.label("• Deleting a transaction replays the whole ledger, so lots and gains stay consistent.");
            ui.label("• Spends require you to enter the fiat value of the crypto at spend time; that value is used as proceeds for gain calculation.");
        });
    }
}
