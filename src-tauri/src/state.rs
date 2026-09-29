use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use crate::db::Db;
use crate::printer::PrintWorker;
use crate::sync::Outbox;
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct SyncInfo {
    pub online: bool,
    pub last_sync_ms: i64,
    pub pending: usize,
    pub conflicts: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserSession {
    pub user_id: i64,
    pub employee_id: Option<i64>,
    pub display_name: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

impl Default for SyncInfo {
    fn default() -> Self {
        Self {
            online: false,
            last_sync_ms: 0,
            pending: 0,
            conflicts: 0,
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub outbox: Arc<Outbox>,
    pub printer: Arc<PrintWorker>,
    pub events: broadcast::Sender<String>,
    pub sync: Arc<Mutex<SyncInfo>>,
    /// Set only after a successful local administrator PIN verification.
    pub kiosk_exit_authorized: Arc<AtomicBool>,
    pub current_user: Arc<Mutex<Option<UserSession>>>,
}

impl AppState {
    pub fn new(db: Arc<Db>, outbox: Arc<Outbox>, printer: Arc<PrintWorker>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            db,
            outbox,
            printer,
            events,
            sync: Arc::new(Mutex::new(SyncInfo::default())),
            kiosk_exit_authorized: Arc::new(AtomicBool::new(false)),
            current_user: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_user(&self, session: UserSession) {
        *self.current_user.lock().unwrap() = Some(session);
    }
    pub fn clear_user(&self) {
        *self.current_user.lock().unwrap() = None;
    }
    pub fn user(&self) -> Option<UserSession> {
        self.current_user.lock().unwrap().clone()
    }
    pub fn require(&self, permission: &str) -> Result<UserSession, String> {
        let user = self
            .user()
            .ok_or_else(|| "Sessão expirada. Faça login novamente.".to_string())?;
        if user
            .permissions
            .iter()
            .any(|value| value == "*" || value == permission)
        {
            Ok(user)
        } else {
            Err(format!(
                "O cargo {} não possui a permissão {}",
                user.role, permission
            ))
        }
    }

    pub fn authorize_kiosk_exit(&self) {
        self.kiosk_exit_authorized.store(true, Ordering::SeqCst);
    }

    pub fn kiosk_exit_is_authorized(&self) -> bool {
        self.kiosk_exit_authorized.load(Ordering::SeqCst)
    }

    pub fn publish(&self, payload: impl Into<String>) {
        let _ = self.events.send(payload.into());
    }

    pub fn set_online(&self, online: bool) {
        let mut s = self.sync.lock().unwrap();
        s.online = online;
        if online {
            s.last_sync_ms = chrono::Utc::now().timestamp_millis();
        }
    }

    pub fn refresh_pending(&self) {
        let pending = self.db.count_pending_outbox().unwrap_or(0);
        let (conflicts, last_error) = self.db.sync_conflicts().unwrap_or((0, None));
        let mut sync = self.sync.lock().unwrap();
        sync.pending = pending;
        sync.conflicts = conflicts;
        sync.last_error = last_error;
    }

    pub fn status(&self) -> SyncInfo {
        self.sync.lock().unwrap().clone()
    }
}
