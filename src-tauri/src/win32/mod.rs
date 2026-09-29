use std::sync::Arc;

use crate::state::AppState;

struct BarcodeBuffer {
    chars: String,
    last_key_time: std::time::Instant,
}

impl BarcodeBuffer {
    fn new() -> Self {
        Self {
            chars: String::with_capacity(32),
            last_key_time: std::time::Instant::now(),
        }
    }

    fn push(&mut self, ch: char) -> Option<String> {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_key_time);
        self.last_key_time = now;

        if elapsed > std::time::Duration::from_millis(30) && !self.chars.is_empty() {
            self.chars.clear();
        }

        if ch == '\r' || ch == '\n' {
            if !self.chars.is_empty() {
                let code = self.chars.clone();
                self.chars.clear();
                return Some(code);
            }
            return None;
        }

        if ch == '\x08' {
            self.chars.pop();
            return None;
        }

        self.chars.push(ch);
        None
    }

    fn clear(&mut self) {
        self.chars.clear();
    }
}

#[cfg(target_os = "windows")]
pub fn install_barcode_hook(state: Arc<AppState>) -> Result<(), String> {
    use std::sync::OnceLock;

    static STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    STATE.set(state).ok();

    log::info!("Win32: Barcode hook registrado (via emit_barcode para injeção manual)");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_barcode_hook(_state: Arc<AppState>) -> Result<(), String> {
    log::warn!("Barcode hook não disponível fora do Windows");
    Ok(())
}

pub fn process_raw_input_msg(_l_param: isize, app: &tauri::AppHandle, buffer: &mut BarcodeBuffer) {
    let _ = (app, buffer);
}

pub fn open_cash_drawer() -> Result<(), String> {
    Err("Abertura de gaveta via Win32 Print Spooler ainda não implementada".into())
}

pub fn read_scale(_port: &str) -> Result<f64, String> {
    Err(
        "Leitura de balança comercial (Toledo/Filizola) via porta COM ainda não implementada"
            .into(),
    )
}

pub fn setup_customer_display() -> Result<(), String> {
    Err("Display secundário do cliente via Win32 Multi-Monitor ainda não implementado".into())
}

pub fn lock_system_keys() -> Result<(), String> {
    Err("Modo kiosk (trava de Alt+Tab/Alt+F4/WinKey) ainda não implementado".into())
}
