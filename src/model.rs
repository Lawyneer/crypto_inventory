//! Domain types shared by the whole app: transactions, wallets,
//! holdings, tax lots, realized-gain records, and the two core
//! enums (transaction kind and inventory method).

/// Inventory method: the order in which cost-basis lots are consumed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    Fifo,
    Lifo,
    Hifo,
}

impl Method {
    pub fn label(self) -> &'static str {
        match self {
            Method::Fifo => "FIFO (First-In, First-Out)",
            Method::Lifo => "LIFO (Last-In, First-Out)",
            Method::Hifo => "HIFO (Highest Price First)",
        }
    }
    pub fn from_db(s: &str) -> Method {
        match s {
            "LIFO" => Method::Lifo,
            "HIFO" => Method::Hifo,
            _ => Method::Fifo,
        }
    }
    pub fn db_key(self) -> &'static str {
        match self {
            Method::Fifo => "FIFO",
            Method::Lifo => "LIFO",
            Method::Hifo => "HIFO",
        }
    }
    pub const ALL: [Method; 3] = [Method::Fifo, Method::Lifo, Method::Hifo];
}

/// What kind of transaction a record is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Purchase,
    Sell,
    Spend,
    Transfer,
    Convert,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Purchase => "Purchase",
            Kind::Sell => "Sell",
            Kind::Spend => "Spend",
            Kind::Transfer => "Transfer",
            Kind::Convert => "Convert",
        }
    }
    pub fn from_db(s: &str) -> Kind {
        match s {
            "Sell" => Kind::Sell,
            "Spend" => Kind::Spend,
            "Transfer" => Kind::Transfer,
            "Convert" => Kind::Convert,
            _ => Kind::Purchase,
        }
    }
    pub fn db_key(self) -> &'static str {
        match self {
            Kind::Purchase => "Purchase",
            Kind::Sell => "Sell",
            Kind::Spend => "Spend",
            Kind::Transfer => "Transfer",
            Kind::Convert => "Convert",
        }
    }
    pub const ALL: [Kind; 5] = [
        Kind::Purchase,
        Kind::Sell,
        Kind::Spend,
        Kind::Transfer,
        Kind::Convert,
    ];
}

#[derive(Clone)]
pub struct Wallet {
    pub id: i64,
    pub name: String,
    pub address: String,
}

#[derive(Clone)]
pub struct Tx {
    pub id: i64,
    pub kind: Kind,
    pub wallet_id: i64,      // source wallet (transfer: from-wallet)
    pub to_wallet_id: i64,   // transfer: destination wallet; 0 otherwise
    pub coin: String,        // source coin
    pub to_coin: String,     // convert: coin received; '' otherwise
    pub ts: String,          // "YYYY-MM-DD HH:MM" local time
    pub exchange: String,    // purchases/sells/converts
    pub amount: f64,         // crypto units given up (source side)
    pub amount_to: f64,      // convert: crypto units received
    pub fee: f64,            // fiat fee (purchase/sell/convert)
    pub asset_fee: f64,      // transfer: network fee denominated in the coin
    pub unit_price: f64,     // fiat per coin (purchase/sell; excludes fee)
    pub fiat_total: f64,     // purchase: total paid incl. fee; sell: net received
    pub description: String, // spends
    pub fiat_value: f64,     // spend/convert: fiat fair-market value (proceeds)
}

#[derive(Clone)]
pub struct Lot {
    pub amount: f64,
    pub unit_cost: f64, // fiat per coin
    pub ts: String,     // acquisition date of this lot
}

#[derive(Clone)]
pub struct Holding {
    pub coin: String,
    pub wallet: String,
    pub amount: f64,
    pub basis: f64,
    pub avg_cost: f64,
}

#[derive(Clone)]
pub struct TaxLot {
    pub coin: String,
    pub wallet: String,
    pub ts: String,
    pub amount: f64,
    pub unit_cost: f64,
}

#[derive(Clone)]
pub struct Realized {
    pub kind: Kind,
    pub ts: String,
    pub coin: String,
    pub wallet: String,
    pub amount: f64,
    pub proceeds: f64,
    pub basis: f64,
    pub gain: f64,
}

/// Format a fiat amount with two decimals.
pub fn fmt2(v: f64) -> String {
    format!("{:.2}", v)
}

/// Format a crypto amount, trimming pointless trailing zeros.
pub fn fmt8(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.8}", v)
    }
}
