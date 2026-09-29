use crate::db::{CashCounted, CashMovement, Db, FechamentoReport};

pub struct CashierManager;

impl CashierManager {
    pub fn sangria(
        db: &Db,
        session_id: i64,
        amount_brl_cents: i64,
        description: Option<&str>,
        operator: Option<&str>,
    ) -> Result<CashMovement, String> {
        if amount_brl_cents <= 0 {
            return Err("Valor da sangria deve ser maior que zero".into());
        }

        let status = db.cash_status().map_err(|e| e.to_string())?;
        if !status.open || status.session_id != Some(session_id) {
            return Err("Caixa não está aberto nesta sessão".into());
        }

        let expected = status.expected_brl_cents;
        if amount_brl_cents > expected {
            return Err(format!(
                "Sangria ({:.2}) excede o valor esperado em caixa ({:.2})",
                amount_brl_cents as f64 / 100.0,
                expected as f64 / 100.0
            ));
        }

        let id = db
            .record_cash_movement(
                session_id,
                "sangria",
                amount_brl_cents,
                description,
                operator,
            )
            .map_err(|e| e.to_string())?;

        log::info!(
            "Sangria: R$ {:.2} na sessão {session_id}",
            amount_brl_cents as f64 / 100.0
        );

        Ok(CashMovement {
            id,
            session_id,
            movement_type: "sangria".into(),
            amount_brl_cents,
            description: description.map(|s| s.into()),
            operator: operator.map(|s| s.into()),
            created_at: chrono::Utc::now().timestamp(),
        })
    }

    pub fn suprimento(
        db: &Db,
        session_id: i64,
        amount_brl_cents: i64,
        description: Option<&str>,
        operator: Option<&str>,
    ) -> Result<CashMovement, String> {
        if amount_brl_cents <= 0 {
            return Err("Valor do suprimento deve ser maior que zero".into());
        }

        let status = db.cash_status().map_err(|e| e.to_string())?;
        if !status.open || status.session_id != Some(session_id) {
            return Err("Caixa não está aberto nesta sessão".into());
        }

        let id = db
            .record_cash_movement(
                session_id,
                "suprimento",
                amount_brl_cents,
                description,
                operator,
            )
            .map_err(|e| e.to_string())?;

        log::info!(
            "Suprimento: R$ {:.2} na sessão {session_id}",
            amount_brl_cents as f64 / 100.0
        );

        Ok(CashMovement {
            id,
            session_id,
            movement_type: "suprimento".into(),
            amount_brl_cents,
            description: description.map(|s| s.into()),
            operator: operator.map(|s| s.into()),
            created_at: chrono::Utc::now().timestamp(),
        })
    }

    pub fn fechamento_cego(
        db: &Db,
        session_id: i64,
        counted: CashCounted,
    ) -> Result<FechamentoReport, String> {
        let status = db.cash_status().map_err(|e| e.to_string())?;
        if !status.open || status.session_id != Some(session_id) {
            return Err("Caixa não está aberto nesta sessão".into());
        }

        let movements = db
            .cash_movements_for_session(session_id)
            .map_err(|e| e.to_string())?;

        let opened_at = status.opened_at.unwrap_or(0);
        let sales_by_method = db
            .sales_by_method_since(opened_at)
            .map_err(|e| e.to_string())?;

        let counted_total = counted.cash_brl_cents
            + counted.card_brl_cents
            + counted.pix_brl_cents
            + counted.cheque_brl_cents;

        let non_cash_sales: i64 = sales_by_method["methods"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| entry["method"].as_str() != Some("cash"))
            .filter_map(|entry| entry["total_brl_cents"].as_i64())
            .sum();
        let expected_total = status.expected_brl_cents + non_cash_sales;
        let difference = counted_total - expected_total;

        log::info!(
            "Fechamento cego sessão {session_id}: esperado R$ {:.2}, contado R$ {:.2}, diff R$ {:.2}",
            expected_total as f64 / 100.0,
            counted_total as f64 / 100.0,
            difference as f64 / 100.0
        );

        Ok(FechamentoReport {
            session: status,
            movements,
            sales_by_method,
            expected_total,
            counted_total,
            difference,
        })
    }
}
