#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crypto_inventory::app::App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([980.0, 680.0])
            .with_title("Crypto Inventory Tracker"),
        ..Default::default()
    };
    eframe::run_native(
        "Crypto Inventory Tracker",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
