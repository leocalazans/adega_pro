//! Durable intake only. No provider ACK or order-state transition happens here.
//! Callers must authenticate the transport and resolve merchant -> tenant/unit
//! from trusted configuration, never from an arbitrary request's tenant fields.
use super::Db;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

pub struct DeliveryScope<'a> {
    pub tenant_id: &'a str,
    pub unit_id: &'a str,
    pub provider: &'a str,
    pub merchant_id: &'a str,
}

pub struct IncomingDeliveryEvent {
    pub event_id: String,
    pub order_id: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct StoredDeliveryEvent {
    pub sequence: i64,
    pub event_id: String,
    pub order_id: String,
    pub payload: Value,
}

fn invalid(message: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(message.into())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

impl DeliveryScope<'_> {
    fn validate(&self) -> rusqlite::Result<()> {
        if [
            self.tenant_id,
            self.unit_id,
            self.provider,
            self.merchant_id,
        ]
        .iter()
        .all(|s| valid_id(s))
        {
            Ok(())
        } else {
            Err(invalid("Escopo de delivery inválido"))
        }
    }
}

impl Db {
    /// All-or-nothing intake; duplicates with different content are conflicts.
    /// The returned count includes only newly persisted events, not replays.
    pub fn store_delivery_events(
        &self,
        scope: &DeliveryScope<'_>,
        events: &[IncomingDeliveryEvent],
    ) -> rusqlite::Result<usize> {
        scope.validate()?;
        if events.len() > 500 {
            return Err(invalid("Lote de delivery excede 500 eventos"));
        }
        let mut encoded = Vec::with_capacity(events.len());
        let mut bytes = 0;
        for event in events {
            if !valid_id(&event.event_id)
                || !valid_id(&event.order_id)
                || !event.payload.is_object()
            {
                return Err(invalid("Evento de delivery inválido"));
            }
            let json = event.payload.to_string();
            bytes += json.len();
            if bytes > 4 * 1024 * 1024 {
                return Err(invalid("Lote de delivery excede 4 MiB"));
            }
            encoded.push(json);
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut inserted = 0;
        for (event, json) in events.iter().zip(encoded) {
            let existing: Option<(String, String)> = tx
                .query_row(
                    "SELECT order_id,payload_json FROM delivery_inbox WHERE
                 tenant_id=?1 AND unit_id=?2 AND provider=?3 AND merchant_id=?4 AND event_id=?5",
                    params![
                        scope.tenant_id,
                        scope.unit_id,
                        scope.provider,
                        scope.merchant_id,
                        event.event_id
                    ],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((order_id, previous)) = existing {
                let previous: Value = serde_json::from_str(&previous)
                    .map_err(|_| invalid("Evento de delivery armazenado está corrompido"))?;
                if order_id != event.order_id || previous != event.payload {
                    return Err(invalid(
                        "Evento de delivery repetido com conteúdo divergente",
                    ));
                }
                continue;
            }
            inserted += tx.execute(
                "INSERT INTO delivery_inbox
                 (tenant_id,unit_id,provider,merchant_id,event_id,order_id,payload_json,received_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    scope.tenant_id,
                    scope.unit_id,
                    scope.provider,
                    scope.merchant_id,
                    event.event_id,
                    event.order_id,
                    json,
                    chrono::Utc::now().timestamp()
                ],
            )?;
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Keyset pagination in arrival order. Arrival order is NOT lifecycle order.
    pub fn delivery_events_after(
        &self,
        scope: &DeliveryScope<'_>,
        after: i64,
        limit: u32,
    ) -> rusqlite::Result<Vec<StoredDeliveryEvent>> {
        scope.validate()?;
        if after < 0 || !(1..=500).contains(&limit) {
            return Err(invalid("Paginação de delivery inválida"));
        }
        let conn = self.conn.lock().unwrap();
        let mut statement = conn.prepare(
            "SELECT id,event_id,order_id,payload_json FROM delivery_inbox WHERE
             tenant_id=?1 AND unit_id=?2 AND provider=?3 AND merchant_id=?4 AND id>?5
             ORDER BY id LIMIT ?6",
        )?;
        let rows = statement.query_map(
            params![
                scope.tenant_id,
                scope.unit_id,
                scope.provider,
                scope.merchant_id,
                after,
                limit
            ],
            |row| {
                let json: String = row.get(3)?;
                let payload = serde_json::from_str(&json)
                    .map_err(|_| invalid("Evento de delivery armazenado está corrompido"))?;
                Ok(StoredDeliveryEvent {
                    sequence: row.get(0)?,
                    event_id: row.get(1)?,
                    order_id: row.get(2)?,
                    payload,
                })
            },
        )?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn scope() -> DeliveryScope<'static> {
        DeliveryScope {
            tenant_id: "tenant-a",
            unit_id: "unit-a",
            provider: "ifood",
            merchant_id: "merchant-a",
        }
    }
    fn event(id: &str) -> IncomingDeliveryEvent {
        IncomingDeliveryEvent {
            event_id: id.into(),
            order_id: "order-a".into(),
            payload: serde_json::json!({"code":"PLC","futureField":{"keep":true}}),
        }
    }

    #[test]
    fn replay_conflict_rolls_back_entire_batch() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        assert_eq!(
            db.store_delivery_events(&scope(), &[event("e1"), event("e1")])
                .unwrap(),
            1
        );
        assert_eq!(
            db.store_delivery_events(&scope(), &[event("e1")]).unwrap(),
            0
        );
        let mut conflict = event("e1");
        conflict.order_id = "other-order".into();
        assert!(db
            .store_delivery_events(&scope(), &[event("e2"), conflict])
            .is_err());
        let mut conflict = event("e1");
        conflict.payload = serde_json::json!({"code":"CFM"});
        assert!(db.store_delivery_events(&scope(), &[conflict]).is_err());
        assert_eq!(db.delivery_events_after(&scope(), 0, 500).unwrap().len(), 1);
    }

    #[test]
    fn scope_isolation_and_keyset_pagination() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        db.store_delivery_events(&scope(), &[event("e1"), event("e2")])
            .unwrap();
        for other in [
            DeliveryScope {
                tenant_id: "tenant-b",
                ..scope()
            },
            DeliveryScope {
                unit_id: "unit-b",
                ..scope()
            },
            DeliveryScope {
                provider: "99food",
                ..scope()
            },
            DeliveryScope {
                merchant_id: "merchant-b",
                ..scope()
            },
        ] {
            assert!(db.delivery_events_after(&other, 0, 500).unwrap().is_empty());
            assert_eq!(db.store_delivery_events(&other, &[event("e1")]).unwrap(), 1);
        }
        let first = db.delivery_events_after(&scope(), 0, 1).unwrap();
        let next = db
            .delivery_events_after(&scope(), first[0].sequence, 1)
            .unwrap();
        assert_eq!(first[0].event_id, "e1");
        assert_eq!(next[0].event_id, "e2");
        assert_eq!(first[0].payload, event("e1").payload);
        assert!(db
            .delivery_events_after(&scope(), next[0].sequence, 500)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn inbox_survives_database_reopen() {
        let path =
            std::env::temp_dir().join(format!("commercectrl-delivery-{}.db", uuid::Uuid::new_v4()));
        {
            let db = Db::open(&path).unwrap();
            db.store_delivery_events(&scope(), &[event("e1")]).unwrap();
        }
        {
            let db = Db::open(&path).unwrap();
            let events = db.delivery_events_after(&scope(), 0, 500).unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].payload, event("e1").payload);
            assert_eq!(
                db.store_delivery_events(&scope(), &[event("e1")]).unwrap(),
                0
            );
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn malformed_batches_do_not_persist() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        assert!(db
            .store_delivery_events(&scope(), &[event("e1"), event("")])
            .is_err());
        let mut oversized = event("e2");
        oversized.payload = serde_json::json!({"data":"a".repeat(4 * 1024 * 1024)});
        assert!(db.store_delivery_events(&scope(), &[oversized]).is_err());
        let batch: Vec<_> = (0..501).map(|n| event(&n.to_string())).collect();
        assert!(db.store_delivery_events(&scope(), &batch).is_err());
        assert!(db
            .delivery_events_after(&scope(), 0, 500)
            .unwrap()
            .is_empty());
        assert!(db.delivery_events_after(&scope(), -1, 1).is_err());
        assert!(db.delivery_events_after(&scope(), 0, 0).is_err());
        assert!(db.delivery_events_after(&scope(), 0, 501).is_err());
        assert!(db
            .store_delivery_events(
                &DeliveryScope {
                    unit_id: "",
                    ..scope()
                },
                &[]
            )
            .is_err());
    }
}
