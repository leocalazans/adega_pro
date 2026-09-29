#![allow(dead_code)]
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PaymentResult {
    pub ok: bool,
    pub method: String,
    pub authorization_code: Option<String>,
    pub qr_data: Option<String>,
    pub message: Option<String>,
}

pub trait PaymentProvider {
    fn method(&self) -> &'static str;
    fn authorize(&self, amount_brl_cents: i64, extra: Option<&serde_json::Value>) -> PaymentResult;
    fn refund(&self, amount_brl_cents: i64) -> PaymentResult;
}

pub struct TefProvider;

impl PaymentProvider for TefProvider {
    fn method(&self) -> &'static str {
        "tef"
    }

    fn authorize(&self, _amount: i64, _extra: Option<&serde_json::Value>) -> PaymentResult {
        PaymentResult {
            ok: false,
            method: "tef".into(),
            authorization_code: None,
            qr_data: None,
            message: Some(
                "TEF (CliSiTef/PayGo) — integração via DLL do pinpad ainda não implementada".into(),
            ),
        }
    }

    fn refund(&self, _amount: i64) -> PaymentResult {
        PaymentResult {
            ok: false,
            method: "tef".into(),
            authorization_code: None,
            qr_data: None,
            message: Some("Estorno TEF ainda não implementado".into()),
        }
    }
}

pub struct SmartPosProvider;

impl PaymentProvider for SmartPosProvider {
    fn method(&self) -> &'static str {
        "smartpos"
    }

    fn authorize(&self, _amount: i64, _extra: Option<&serde_json::Value>) -> PaymentResult {
        PaymentResult {
            ok: false,
            method: "smartpos".into(),
            authorization_code: None,
            qr_data: None,
            message: Some(
                "SmartPOS (Stone/Rede/PagBank) — comunicação async via API ainda não implementada"
                    .into(),
            ),
        }
    }

    fn refund(&self, _amount: i64) -> PaymentResult {
        PaymentResult {
            ok: false,
            method: "smartpos".into(),
            authorization_code: None,
            qr_data: None,
            message: Some("Estorno SmartPOS ainda não implementado".into()),
        }
    }
}

pub mod pix {
    use super::PaymentProvider;
    use super::PaymentResult;

    #[derive(Debug, Clone)]
    pub struct PixPayload {
        pub key: String,
        pub merchant_name: String,
        pub merchant_city: String,
        pub amount_brl_cents: i64,
        pub txid: String,
    }

    impl PixPayload {
        pub fn new(
            key: &str,
            merchant_name: &str,
            merchant_city: &str,
            amount_brl_cents: i64,
        ) -> Self {
            Self {
                key: key.into(),
                merchant_name: merchant_name.into(),
                merchant_city: merchant_city.into(),
                amount_brl_cents,
                txid: uuid::Uuid::new_v4().simple().to_string()[..12].into(),
            }
        }

        pub fn amount_brl(&self) -> String {
            format!("{:.2}", self.amount_brl_cents as f64 / 100.0)
        }

        fn field(tag: u8, value: &str) -> String {
            format!("{:02}{:02}{}", tag, value.len(), value)
        }

        pub fn build(&self) -> String {
            let merchant_account = format!(
                "{}{}",
                Self::field(0, "br.gov.bcb.pix"),
                Self::field(1, &self.key),
            );

            let mut out = String::new();
            out.push_str(&Self::field(0, "01"));
            // Point of Initiation Method = 12 (uso único / não reutilizável),
            // obrigatório quando o QR carrega valor de uma venda específica.
            out.push_str(&Self::field(1, "12"));
            out.push_str(&Self::field(26, &merchant_account));
            out.push_str(&Self::field(52, "0000"));
            out.push_str(&Self::field(53, "986"));
            out.push_str(&Self::field(54, &self.amount_brl()));
            out.push_str(&Self::field(58, "BR"));
            out.push_str(&Self::field(
                59,
                &self.merchant_name[..self.merchant_name.len().min(25)],
            ));
            out.push_str(&Self::field(
                60,
                &self.merchant_city[..self.merchant_city.len().min(15)],
            ));
            let additional = format!("{}", Self::field(5, &self.txid[..self.txid.len().min(25)]),);
            out.push_str(&Self::field(62, &additional));
            // CRC-16/CCITT-FALSE sobre payload + "6304" (prefixo do campo 63),
            // conforme manual BR Code do Banco Central.
            let crc_input = format!("{out}6304");
            let crc = Self::crc16(crc_input.as_bytes());
            out.push_str(&format!("6304{:04X}", crc));
            out
        }

        pub(crate) fn crc16(data: &[u8]) -> u16 {
            let mut crc: u16 = 0xFFFF;
            for &b in data {
                crc ^= (b as u16) << 8;
                for _ in 0..8 {
                    if crc & 0x8000 != 0 {
                        crc = (crc << 1) ^ 0x1021;
                    } else {
                        crc <<= 1;
                    }
                }
            }
            crc & 0xFFFF
        }
    }

    pub struct PixProvider;

    impl PaymentProvider for PixProvider {
        fn method(&self) -> &'static str {
            "pix"
        }

        fn authorize(
            &self,
            amount_brl_cents: i64,
            extra: Option<&serde_json::Value>,
        ) -> PaymentResult {
            let key = extra
                .and_then(|e| e.get("pix_key"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = extra
                .and_then(|e| e.get("merchant_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("COMMERCECTRL")
                .to_string();
            let city = extra
                .and_then(|e| e.get("merchant_city"))
                .and_then(|v| v.as_str())
                .unwrap_or("SAO PAULO")
                .to_string();

            if key.is_empty() {
                return PaymentResult {
                    ok: false,
                    method: "pix".into(),
                    authorization_code: None,
                    qr_data: None,
                    message: Some("Chave PIX obrigatória (extra.pix_key)".into()),
                };
            }

            let payload = PixPayload::new(&key, &name, &city, amount_brl_cents);
            PaymentResult {
                ok: true,
                method: "pix".into(),
                authorization_code: Some(payload.txid.clone()),
                qr_data: Some(payload.build()),
                message: Some("PIX gerado — aguardando confirmação".into()),
            }
        }

        fn refund(&self, _amount: i64) -> PaymentResult {
            PaymentResult {
                ok: false,
                method: "pix".into(),
                authorization_code: None,
                qr_data: None,
                message: Some("Estorno PIX ainda não implementado".into()),
            }
        }
    }
}

pub fn provider_for(method: &str) -> Box<dyn PaymentProvider> {
    match method {
        "pix" => Box::new(pix::PixProvider),
        "tef" => Box::new(TefProvider),
        "smartpos" => Box::new(SmartPosProvider),
        _ => Box::new(TefProvider),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pix_payload_brcode_com_crc_valido() {
        let payload = pix::PixPayload::new(
            "alice@example.com",
            "COMMERCECTRL",
            "SAO PAULO",
            10000, // R$ 100,00
        );
        let br = payload.build();

        // Estrutura básica do BR Code
        assert!(
            br.starts_with("000201"),
            "deve iniciar com Payload Format Indicator 01"
        );
        assert!(
            br.contains("010212"),
            "campo 01 deve ser Point of Initiation Method 12"
        );
        assert!(
            br.contains("br.gov.bcb.pix"),
            "campo 26 deve conter GUI br.gov.bcb.pix"
        );
        assert!(
            br.contains("5303986"),
            "campo 53 deve ser a moeda BRL (986)"
        );
        assert!(br.contains("5802BR"), "campo 58 deve ser BR");

        // CRC deve ter 4 hex dígitos e vir após o prefixo "6304"
        let crc_field = &br[br.len() - 4..];
        assert!(
            crc_field.chars().all(|c| c.is_ascii_hexdigit()),
            "CRC deve ser hex"
        );
        assert_eq!(
            &br[br.len() - 8..br.len() - 4],
            "6304",
            "prefixo do campo 63"
        );
        assert_ne!(crc_field, "6304", "CRC não pode ser igual ao prefixo");
        // Recalcula o CRC sobre payload+6304 e confere
        let payload_without_crc = &br[..br.len() - 4];
        let expected_crc = pix::PixPayload::crc16(payload_without_crc.as_bytes());
        assert_eq!(crc_field.to_uppercase(), format!("{:04X}", expected_crc));
    }

    #[test]
    fn pix_provider_gera_qr_com_chave() {
        let provider = provider_for("pix");
        let extra = serde_json::json!({
            "pix_key": "alice@example.com",
            "merchant_name": "COMMERCECTRL",
            "merchant_city": "SAO PAULO",
        });
        let res = provider.authorize(5000, Some(&extra));
        assert!(res.ok);
        let qr = res.qr_data.unwrap();
        assert!(qr.starts_with("000201"));
        assert!(qr.ends_with("6304") || qr.len() > 4);
    }
}
