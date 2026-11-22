use crypto_inventory::model::{Kind, Method, fmt2, fmt8};

#[test]
fn method_round_trips_through_db_keys() {
    for m in Method::ALL {
        assert_eq!(Method::from_db(m.db_key()), m);
    }
}

#[test]
fn method_unknown_key_defaults_to_fifo() {
    assert_eq!(Method::from_db("NONSENSE"), Method::Fifo);
}

#[test]
fn kind_round_trips_through_db_keys() {
    for k in Kind::ALL {
        assert_eq!(Kind::from_db(k.db_key()), k);
    }
}

#[test]
fn kind_unknown_key_defaults_to_purchase() {
    assert_eq!(Kind::from_db("???"), Kind::Purchase);
}

#[test]
fn fmt_formats_fiat_with_two_decimals() {
    assert_eq!(fmt2(1234.567), "1234.57");
    assert_eq!(fmt2(0.0), "0.00");
}

#[test]
fn fmt_formats_crypto_amounts_compactly() {
    assert_eq!(fmt8(2.0), "2");
    assert_eq!(fmt8(0.12345678), "0.12345678");
}
