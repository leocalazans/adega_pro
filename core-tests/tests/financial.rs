use commercectrl_core_tests::{
    db::{CashCounted, Db, PaymentIn, SaleItemIn},
    financial::CashierManager,
};

#[test]
fn blind_closing_accounts_for_movements_and_non_cash_sales() {
    let path = std::env::temp_dir().join(format!(
        "commercectrl-financial-{}.db",
        uuid::Uuid::new_v4()
    ));
    let db = Db::open(&path).unwrap();
    let session = db.cash_open(10_000).unwrap().session_id.unwrap();
    db.upsert_product("FC1", "FC1", "Produto", None, 1_000, 10.0)
        .unwrap();
    db.record_sale(
        &[SaleItemIn {
            ean: "FC1".into(),
            qty: 1.0,
            price_brl_cents: 1_000,
        }],
        &PaymentIn {
            method: "cash".into(),
            amount_brl_cents: 1_000,
            extra: None,
        },
        "T1",
    )
    .unwrap();
    db.record_sale(
        &[SaleItemIn {
            ean: "FC1".into(),
            qty: 2.0,
            price_brl_cents: 1_000,
        }],
        &PaymentIn {
            method: "pix".into(),
            amount_brl_cents: 2_000,
            extra: None,
        },
        "T1",
    )
    .unwrap();
    CashierManager::sangria(&db, session, 500, Some("retirada"), Some("teste")).unwrap();
    CashierManager::suprimento(&db, session, 200, Some("troco"), Some("teste")).unwrap();
    assert_eq!(db.cash_status().unwrap().expected_brl_cents, 10_700);
    let report = CashierManager::fechamento_cego(
        &db,
        session,
        CashCounted {
            cash_brl_cents: 10_700,
            card_brl_cents: 0,
            pix_brl_cents: 2_000,
            cheque_brl_cents: 0,
        },
    )
    .unwrap();
    assert_eq!(report.expected_total, 12_700);
    assert_eq!(report.difference, 0);
    drop(db);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}
