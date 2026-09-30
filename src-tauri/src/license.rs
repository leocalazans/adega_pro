use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::db::Db;

const TOKEN_SETTING: &str = "license.token";
const INSTALLATION_SETTING: &str = "license.installation_id";
const INSTALLED_AT_SETTING: &str = "license.installed_at";
const LAST_SEEN_SETTING: &str = "license.last_seen_at";
const TRIAL_DAYS: i64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LicenseClaims {
    pub tenant_id: String,
    pub unit_id: String,
    pub installation_id: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub grace_until: i64,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedLicense {
    pub claims: LicenseClaims,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LicenseStatus {
    pub allowed_to_sell: bool,
    pub mode: String,
    pub installation_id: String,
    pub expires_at: Option<i64>,
    pub grace_until: Option<i64>,
    pub message: String,
}

fn installation(db: &Db) -> Result<(String, i64), String> {
    let now = Utc::now().timestamp();
    let id = match db
        .get_setting(INSTALLATION_SETTING)
        .map_err(|e| e.to_string())?
    {
        Some(value) => value,
        None => {
            let value = uuid::Uuid::new_v4().to_string();
            db.set_setting(INSTALLATION_SETTING, &value)
                .map_err(|e| e.to_string())?;
            value
        }
    };
    let installed_at = match db
        .get_setting(INSTALLED_AT_SETTING)
        .map_err(|e| e.to_string())?
    {
        Some(value) => value.parse().unwrap_or(now),
        None => {
            db.set_setting(INSTALLED_AT_SETTING, &now.to_string())
                .map_err(|e| e.to_string())?;
            now
        }
    };
    Ok((id, installed_at))
}

pub fn installation_id(db: &Db) -> Result<String, String> {
    installation(db).map(|(id, _)| id)
}

fn verifying_key() -> Result<VerifyingKey, String> {
    let configured = option_env!("COMMERCECTRL_LICENSE_PUBLIC_KEY_B64")
        .map(str::to_owned)
        .or_else(|| {
            if cfg!(debug_assertions) {
                std::env::var("COMMERCECTRL_LICENSE_PUBLIC_KEY_B64").ok()
            } else {
                None
            }
        });
    if let Some(value) = configured {
        let bytes = STANDARD
            .decode(value.trim())
            .map_err(|_| "chave pública de licença inválida".to_string())?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "chave pública deve ter 32 bytes".to_string())?;
        return VerifyingKey::from_bytes(&bytes).map_err(|e| e.to_string());
    }
    if cfg!(debug_assertions) {
        return Ok(SigningKey::from_bytes(&[7_u8; 32]).verifying_key());
    }
    Err("aplicativo de produção sem chave pública de licenciamento".into())
}

fn verify_token(db: &Db, token: &SignedLicense, installation_id: &str) -> Result<(), String> {
    if token.claims.installation_id != installation_id {
        return Err("licença pertence a outra instalação".into());
    }
    let expected_unit = crate::sync::credentials(db)
        .map(|value| value.unit_id)
        .unwrap_or_default();
    if !expected_unit.is_empty() && token.claims.unit_id != expected_unit {
        return Err("licença pertence a outra unidade".into());
    }
    let expected_tenant = crate::sync::credentials(db)
        .map(|value| value.tenant_id)
        .unwrap_or_default();
    if !expected_tenant.is_empty() && token.claims.tenant_id != expected_tenant {
        return Err("licença pertence a outra empresa".into());
    }
    if token.claims.issued_at > Utc::now().timestamp() + 300
        || token.claims.expires_at < token.claims.issued_at
        || token.claims.grace_until < token.claims.expires_at
        || token.claims.nonce.trim().is_empty()
    {
        return Err("período de carência inválido".into());
    }
    let payload = serde_json::to_vec(&token.claims).map_err(|e| e.to_string())?;
    let signature_bytes = STANDARD
        .decode(&token.signature)
        .map_err(|_| "assinatura de licença inválida".to_string())?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|e| e.to_string())?;
    verifying_key()?
        .verify(&payload, &signature)
        .map_err(|_| "assinatura da licença não confere".to_string())
}

pub fn status(db: &Db) -> LicenseStatus {
    let now = Utc::now().timestamp();
    let (installation_id, installed_at) = match installation(db) {
        Ok(value) => value,
        Err(error) => {
            return LicenseStatus {
                allowed_to_sell: false,
                mode: "restricted".into(),
                installation_id: String::new(),
                expires_at: None,
                grace_until: None,
                message: error,
            }
        }
    };

    let last_seen = db
        .get_setting(LAST_SEEN_SETTING)
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(installed_at);
    if now + 300 < last_seen {
        let _ = db.audit_license("clock_rollback", Some("relógio anterior ao último uso"));
        return LicenseStatus {
            allowed_to_sell: false,
            mode: "restricted".into(),
            installation_id,
            expires_at: None,
            grace_until: None,
            message: "Relógio do computador retrocedeu. Importe uma licença de recuperação.".into(),
        };
    }
    if now > last_seen {
        let _ = db.set_setting(LAST_SEEN_SETTING, &now.to_string());
    }

    let raw = db.get_setting(TOKEN_SETTING).ok().flatten();
    let Some(raw) = raw else {
        let trial_until = installed_at + TRIAL_DAYS * 86_400;
        return LicenseStatus {
            allowed_to_sell: now <= trial_until,
            mode: if now <= trial_until {
                "trial"
            } else {
                "restricted"
            }
            .into(),
            installation_id,
            expires_at: Some(trial_until),
            grace_until: Some(trial_until),
            message: if now <= trial_until {
                "Período de avaliação local ativo".into()
            } else {
                "Avaliação encerrada. Importe uma licença assinada para continuar vendendo.".into()
            },
        };
    };

    let token: SignedLicense = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => {
            return LicenseStatus {
                allowed_to_sell: false,
                mode: "restricted".into(),
                installation_id,
                expires_at: None,
                grace_until: None,
                message: "Arquivo de licença armazenado é inválido".into(),
            }
        }
    };
    if let Err(error) = verify_token(db, &token, &installation_id) {
        return LicenseStatus {
            allowed_to_sell: false,
            mode: "restricted".into(),
            installation_id,
            expires_at: Some(token.claims.expires_at),
            grace_until: Some(token.claims.grace_until),
            message: error,
        };
    }
    let (allowed, mode, message) = if now <= token.claims.expires_at {
        (true, "active", "Licença ativa")
    } else if now <= token.claims.grace_until {
        (true, "grace", "Licença vencida; período de carência ativo")
    } else {
        (false, "restricted", "Licença e carência expiradas")
    };
    LicenseStatus {
        allowed_to_sell: allowed,
        mode: mode.into(),
        installation_id,
        expires_at: Some(token.claims.expires_at),
        grace_until: Some(token.claims.grace_until),
        message: message.into(),
    }
}

pub fn import_file(db: &Db, path: &Path) -> Result<LicenseStatus, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("não foi possível ler {}: {e}", path.display()))?;
    import_token(db, &raw, Some(&path.display().to_string()))
}

pub fn import_token(db: &Db, raw: &str, source: Option<&str>) -> Result<LicenseStatus, String> {
    let token: SignedLicense = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("arquivo de licença inválido: {e}"))?;
    let (installation_id, _) = installation(db)?;
    verify_token(db, &token, &installation_id)?;
    let canonical = serde_json::to_string(&token).map_err(|e| e.to_string())?;
    db.set_setting(TOKEN_SETTING, &canonical)
        .map_err(|e| e.to_string())?;
    db.audit_license("license_imported", source)
        .map_err(|e| e.to_string())?;
    Ok(status(db))
}

pub fn scan_removable(db: &Db) -> Result<LicenseStatus, String> {
    let candidates: Vec<PathBuf> = if cfg!(windows) {
        (b'D'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\commercectrl-license.json", letter as char)))
            .collect()
    } else {
        vec![
            PathBuf::from("/media/commercectrl-license.json"),
            PathBuf::from("/mnt/commercectrl-license.json"),
        ]
    };
    for candidate in candidates {
        if candidate.is_file() {
            return import_file(db, &candidate);
        }
    }
    Err("commercectrl-license.json não encontrado em mídia removível".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    #[test]
    fn assinatura_valida_e_detecta_adulteracao() {
        let key = SigningKey::from_bytes(&[7_u8; 32]);
        let claims = LicenseClaims {
            tenant_id: "tenant".into(),
            unit_id: "*".into(),
            installation_id: "install".into(),
            issued_at: 1,
            expires_at: 2,
            grace_until: 3,
            nonce: "n".into(),
        };
        let payload = serde_json::to_vec(&claims).unwrap();
        let token = SignedLicense {
            claims: claims.clone(),
            signature: STANDARD.encode(key.sign(&payload).to_bytes()),
        };
        let db = Db::open(Path::new(":memory:")).unwrap();
        assert!(verify_token(&db, &token, "install").is_ok());
        let mut changed = token;
        changed.claims.grace_until = 30;
        assert!(verify_token(&db, &changed, "install").is_err());
    }

    fn signed(claims: LicenseClaims) -> String {
        let signature =
            SigningKey::from_bytes(&[7_u8; 32]).sign(&serde_json::to_vec(&claims).unwrap());
        serde_json::to_string(&SignedLicense {
            claims,
            signature: STANDARD.encode(signature.to_bytes()),
        })
        .unwrap()
    }

    #[test]
    fn licenca_expirada_bloqueia_e_importacao_bom_renova() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        let initial = status(&db);
        assert!(initial.allowed_to_sell);
        let now = Utc::now().timestamp();
        let mut claims = LicenseClaims {
            tenant_id: "tenant".into(),
            unit_id: "unit".into(),
            installation_id: initial.installation_id,
            issued_at: now - 3600,
            expires_at: now - 120,
            grace_until: now - 60,
            nonce: "expiry-test".into(),
        };
        let expired = import_token(&db, &signed(claims.clone()), Some("test")).unwrap();
        assert!(!expired.allowed_to_sell);
        claims.expires_at = now + 3600;
        claims.grace_until = now + 7200;
        let renewed =
            import_token(&db, &format!("\u{feff}{}", signed(claims)), Some("test")).unwrap();
        assert!(renewed.allowed_to_sell);
        assert_eq!(renewed.mode, "active");
        assert!(status(&db).allowed_to_sell);
    }

    #[test]
    fn pendrive_de_outra_instalacao_e_relogio_retrocedido_nao_liberam() {
        let db = Db::open(Path::new(":memory:")).unwrap();
        status(&db);
        let now = Utc::now().timestamp();
        let claims = LicenseClaims {
            tenant_id: "tenant".into(),
            unit_id: "unit".into(),
            installation_id: "*".into(),
            issued_at: now,
            expires_at: now + 3600,
            grace_until: now + 7200,
            nonce: "other".into(),
        };
        assert!(import_token(&db, &signed(claims), Some("test")).is_err());
        db.set_setting(LAST_SEEN_SETTING, &(now + 3600).to_string())
            .unwrap();
        assert!(!status(&db).allowed_to_sell);
    }
}
