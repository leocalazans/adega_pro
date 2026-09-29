use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::mpsc;

use crate::db::Db;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum PrintStatus {
    Ok,
    Busy,
    Error(String),
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub mode: String,
    pub windows_printer: Option<String>,
    pub escpos_host: Option<String>,
    pub escpos_port: u16,
    pub local_port: Option<String>,
    pub prefer_escpos: bool,
    pub print_logo: bool,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            mode: "auto".into(),
            windows_printer: None,
            escpos_host: None,
            escpos_port: 9100,
            local_port: None,
            prefer_escpos: true,
            print_logo: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
    pub is_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrintJob {
    Receipt {
        sale_uuid: String,
        items: Vec<(String, i64)>,
        total_brl_cents: i64,
        #[serde(default)]
        store_name: Option<String>,
        #[serde(default)]
        payment_method: Option<String>,
        #[serde(default)]
        consumer_document: Option<String>,
        #[serde(default)]
        logo_data_url: Option<String>,
    },
    Raw {
        bytes: Vec<u8>,
    },
    Text {
        text: String,
    },
}

pub struct PrintWorker {
    tx: mpsc::Sender<PrintJob>,
    status: Arc<Mutex<PrintStatus>>,
    config: Arc<Mutex<PrinterConfig>>,
    config_path: PathBuf,
}

impl PrintWorker {
    pub fn start(db: Arc<Db>, app: tauri::AppHandle, config_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<PrintJob>(32);
        let status = Arc::new(Mutex::new(PrintStatus::Offline));
        let status_clone = status.clone();
        let config = Arc::new(Mutex::new(load_config(&config_path)));
        let config_clone = config.clone();

        std::thread::Builder::new()
            .name("pdv-printer".into())
            .spawn(move || {
                worker_loop(rx, db, app, status_clone, config_clone);
            })
            .ok();

        Self {
            tx,
            status,
            config,
            config_path,
        }
    }

    pub async fn submit(&self, job: PrintJob) -> Result<(), String> {
        *self.status.lock().unwrap() = PrintStatus::Busy;
        self.tx
            .send(job)
            .await
            .map_err(|e| format!("Fila de impressão cheia: {e}"))
    }

    pub fn status(&self) -> PrintStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn config(&self) -> PrinterConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn save_config(&self, config: PrinterConfig) -> Result<(), String> {
        if config.escpos_port == 0 {
            return Err("Porta ESC/POS inválida".into());
        }
        let json = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        std::fs::write(&self.config_path, json)
            .map_err(|e| format!("Configuração da impressora: {e}"))?;
        *self.config.lock().unwrap() = config;
        Ok(())
    }
}

fn worker_loop(
    mut rx: mpsc::Receiver<PrintJob>,
    db: Arc<Db>,
    app: tauri::AppHandle,
    status: Arc<Mutex<PrintStatus>>,
    config: Arc<Mutex<PrinterConfig>>,
) {
    log::info!("PrintWorker: thread iniciada");

    while let Some(job) = rx.blocking_recv() {
        *status.lock().unwrap() = PrintStatus::Busy;

        match try_print(&job, &config.lock().unwrap().clone()) {
            Ok(()) => {
                *status.lock().unwrap() = PrintStatus::Ok;
                let _ = app.emit("print-success", ());
            }
            Err(e) => {
                log::warn!("PrintWorker: impressão falhou: {e}");
                *status.lock().unwrap() = PrintStatus::Error(e.clone());

                fallback_to_spool(&job, &db, &e);

                let _ = app.emit("print-error", e);
            }
        }
    }

    log::info!("PrintWorker: thread encerrada");
}

fn try_print(job: &PrintJob, config: &PrinterConfig) -> Result<(), String> {
    match job {
        PrintJob::Receipt {
            items,
            total_brl_cents,
            sale_uuid,
            store_name,
            payment_method,
            consumer_document,
            logo_data_url,
            ..
        } => {
            let mut esc = EscPos::new().init().align(Align::Center);
            if config.print_logo {
                if let Some(logo) = logo_data_url.as_deref() {
                    if let Ok(raster) = logo_to_escpos(logo, 320) {
                        esc = esc.raw(&raster).newline(1);
                    }
                }
            }
            esc = esc.double_height(true);
            esc = esc
                .text(store_name.as_deref().unwrap_or("ESTABELECIMENTO"))
                .newline(1);
            esc = esc.double_height(false).align(Align::Left);
            esc = esc
                .align(Align::Center)
                .text("COMPROVANTE DE VENDA")
                .newline(1)
                .text("NAO E DOCUMENTO FISCAL")
                .newline(1)
                .align(Align::Left);
            esc = esc.text(&format!(
                "{}",
                chrono::Local::now().format("%d/%m/%Y %H:%M")
            ));
            esc = esc.newline(1);

            for (desc, price) in items {
                let price_str = format!("R$ {:.2}", *price as f64 / 100.0);
                let line = format!("{:<30}{}", desc, price_str);
                esc = esc.text(&line).newline(1);
            }

            esc = esc.text(&"-".repeat(32)).newline(1);
            esc = esc.text(&format!("Venda: {}", sale_uuid)).newline(1);
            if let Some(method) = payment_method {
                esc = esc.text(&format!("Pagamento: {}", method)).newline(1);
            }
            if let Some(document) = consumer_document.as_deref().filter(|v| !v.is_empty()) {
                esc = esc.text(&format!("Consumidor: {}", document)).newline(1);
            }
            esc = esc
                .align(Align::Center)
                .double_height(true)
                .text(&format!("TOTAL R$ {:.2}", *total_brl_cents as f64 / 100.0))
                .newline(2)
                .text("Obrigado!")
                .newline(2)
                .cut(CutMode::Full);

            write_to_printer(esc.to_bytes(), config)
        }
        PrintJob::Raw { bytes } => write_to_printer(bytes, config),
        PrintJob::Text { text } => {
            let esc = EscPos::new()
                .init()
                .text(text)
                .newline(1)
                .cut(CutMode::Full);
            write_to_printer(esc.to_bytes(), config)
        }
    }
}

fn write_to_printer(bytes: &[u8], config: &PrinterConfig) -> Result<(), String> {
    use std::io::Write;

    let network = || -> Result<(), String> {
        let host = config
            .escpos_host
            .as_deref()
            .ok_or("IP da ESC/POS não configurado")?;
        let mut stream = std::net::TcpStream::connect_timeout(
            &format!("{host}:{}", config.escpos_port)
                .parse()
                .map_err(|_| "Endereço ESC/POS inválido")?,
            std::time::Duration::from_secs(2),
        )
        .map_err(|e| format!("ESC/POS {host}:{}: {e}", config.escpos_port))?;
        stream
            .write_all(bytes)
            .and_then(|_| stream.flush())
            .map_err(|e| format!("ESC/POS: {e}"))
    };
    if config.prefer_escpos && config.escpos_host.as_deref().is_some_and(|v| !v.is_empty()) {
        if network().is_ok() {
            return Ok(());
        }
    }
    if config.mode == "escpos_network" {
        return network();
    }

    if config.mode == "windows" || config.mode == "auto" {
        #[cfg(windows)]
        if let Ok(()) = windows_raw_print(config.windows_printer.as_deref(), bytes) {
            return Ok(());
        }
    }

    let port_name = config
        .local_port
        .clone()
        .or_else(|| std::env::var("PRINTER_PORT").ok())
        .ok_or_else(|| "Nenhuma impressora configurada ou disponível".to_string())?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&port_name)
        .map_err(|e| format!("Falha ao abrir porta {port_name}: {e}"))?;

    file.write_all(bytes)
        .map_err(|e| format!("Falha na escrita para {port_name}: {e}"))?;

    file.flush()
        .map_err(|e| format!("Falha no flush para {port_name}: {e}"))?;

    Ok(())
}

fn load_config(path: &PathBuf) -> PrinterConfig {
    std::fs::read(path)
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or_default()
}

fn logo_to_escpos(data_url: &str, max_width: u32) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let encoded = data_url.split_once(',').ok_or("Logo inválida")?.1;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "Logo inválida")?;
    let source = image::load_from_memory(&bytes)
        .map_err(|e| format!("Logo: {e}"))?
        .into_rgba8();
    let width = source.width().min(max_width).max(1);
    let height =
        ((source.height() as f64 * width as f64 / source.width() as f64).round() as u32).max(1);
    let image = image::imageops::resize(
        &source,
        width,
        height,
        image::imageops::FilterType::Triangle,
    );
    let row_bytes = width.div_ceil(8);
    let mut out = vec![
        0x1d,
        0x76,
        0x30,
        0x00,
        (row_bytes & 255) as u8,
        (row_bytes >> 8) as u8,
        (height & 255) as u8,
        (height >> 8) as u8,
    ];
    for y in 0..height {
        for byte_x in 0..row_bytes {
            let mut byte = 0u8;
            for bit in 0..8 {
                let x = byte_x * 8 + bit;
                if x < width {
                    let p = image.get_pixel(x, y);
                    let luminance =
                        (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
                    if p[3] > 80 && luminance < 170 {
                        byte |= 1 << (7 - bit);
                    }
                }
            }
            out.push(byte);
        }
    }
    Ok(out)
}

pub fn list_windows_printers() -> Result<Vec<PrinterInfo>, String> {
    #[cfg(windows)]
    {
        let script = "Get-CimInstance Win32_Printer | Select-Object Name,Default,Network | ConvertTo-Json -Compress";
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| format!("Não foi possível consultar impressoras: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
        let rows = value.as_array().cloned().unwrap_or_else(|| vec![value]);
        return Ok(rows
            .into_iter()
            .filter_map(|v| {
                Some(PrinterInfo {
                    name: v.get("Name")?.as_str()?.to_string(),
                    is_default: v.get("Default").and_then(|x| x.as_bool()).unwrap_or(false),
                    is_network: v.get("Network").and_then(|x| x.as_bool()).unwrap_or(false),
                })
            })
            .collect());
    }
    #[cfg(not(windows))]
    {
        Ok(vec![])
    }
}

#[cfg(windows)]
fn windows_raw_print(name: Option<&str>, bytes: &[u8]) -> Result<(), String> {
    use std::{
        ffi::c_void,
        ptr::{null, null_mut},
    };
    #[repr(C)]
    struct DocInfo {
        doc_name: *const u16,
        output: *const u16,
        data_type: *const u16,
    }
    #[link(name = "Winspool")]
    extern "system" {
        fn GetDefaultPrinterW(name: *mut u16, size: *mut u32) -> i32;
        fn OpenPrinterW(name: *const u16, handle: *mut *mut c_void, defaults: *const c_void)
            -> i32;
        fn ClosePrinter(handle: *mut c_void) -> i32;
        fn StartDocPrinterW(handle: *mut c_void, level: u32, info: *const DocInfo) -> u32;
        fn EndDocPrinter(handle: *mut c_void) -> i32;
        fn StartPagePrinter(handle: *mut c_void) -> i32;
        fn EndPagePrinter(handle: *mut c_void) -> i32;
        fn WritePrinter(
            handle: *mut c_void,
            buffer: *const c_void,
            count: u32,
            written: *mut u32,
        ) -> i32;
    }
    fn wide(v: &str) -> Vec<u16> {
        v.encode_utf16().chain(Some(0)).collect()
    }
    let printer = if let Some(v) = name.filter(|v| !v.is_empty()) {
        wide(v)
    } else {
        unsafe {
            let mut size = 0;
            GetDefaultPrinterW(null_mut(), &mut size);
            let mut v = vec![0u16; size as usize];
            if GetDefaultPrinterW(v.as_mut_ptr(), &mut size) == 0 {
                return Err("Windows não possui impressora padrão".into());
            }
            v
        }
    };
    let mut handle = null_mut();
    unsafe {
        if OpenPrinterW(printer.as_ptr(), &mut handle, null()) == 0 {
            return Err("Não foi possível abrir a impressora do Windows".into());
        }
        let doc = wide("CommerceCTRL - comprovante");
        let raw = wide("RAW");
        let info = DocInfo {
            doc_name: doc.as_ptr(),
            output: null(),
            data_type: raw.as_ptr(),
        };
        if StartDocPrinterW(handle, 1, &info) == 0 {
            ClosePrinter(handle);
            return Err("Falha ao iniciar documento de impressão".into());
        }
        StartPagePrinter(handle);
        let mut written = 0;
        let ok = WritePrinter(
            handle,
            bytes.as_ptr() as *const c_void,
            bytes.len() as u32,
            &mut written,
        );
        EndPagePrinter(handle);
        EndDocPrinter(handle);
        ClosePrinter(handle);
        if ok == 0 || written != bytes.len() as u32 {
            Err("A fila do Windows não aceitou todo o documento".into())
        } else {
            Ok(())
        }
    }
}

fn fallback_to_spool(job: &PrintJob, db: &Db, error: &str) {
    if let PrintJob::Receipt { sale_uuid, .. } = job {
        let receipt_json = serde_json::to_string(job).unwrap_or_default();
        match db.enqueue_print(sale_uuid, &receipt_json) {
            Ok(id) => {
                log::info!("PrintWorker: job {id} salvo na fila de fallback para {sale_uuid}");
                let _ = db.mark_print_failed(id, error);
            }
            Err(e) => {
                log::error!("PrintWorker: falha ao salvar fallback: {e}");
            }
        }
    }
}

// ── ESC/POS Builder ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutMode {
    Full,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct EscPos {
    buf: Vec<u8>,
}

impl EscPos {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn init(mut self) -> Self {
        self.buf.extend_from_slice(&[0x1B, 0x40]);
        self
    }

    pub fn align(mut self, a: Align) -> Self {
        let code = match a {
            Align::Left => 0x00,
            Align::Center => 0x01,
            Align::Right => 0x02,
        };
        self.buf.extend_from_slice(&[0x1B, 0x61, code]);
        self
    }

    pub fn text(mut self, s: &str) -> Self {
        self.buf.extend_from_slice(s.as_bytes());
        self
    }

    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.buf.extend_from_slice(bytes);
        self
    }

    pub fn newline(mut self, n: u8) -> Self {
        for _ in 0..n {
            self.buf.push(b'\n');
        }
        self
    }

    pub fn double_height(mut self, on: bool) -> Self {
        self.buf
            .extend_from_slice(&[0x1D, 0x21, if on { 0x11 } else { 0x00 }]);
        self
    }

    pub fn beep(mut self) -> Self {
        self.buf.extend_from_slice(&[0x07]);
        self
    }

    pub fn cut(mut self, mode: CutMode) -> Self {
        let m = match mode {
            CutMode::Full => 0x00,
            CutMode::Partial => 0x01,
        };
        self.buf.extend_from_slice(&[0x1D, 0x56, m]);
        self
    }

    pub fn open_drawer(mut self) -> Self {
        self.buf.extend_from_slice(&[0x1B, 0x70, 0x00, 0x19, 0xFA]);
        self
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.buf
    }
}

impl Default for EscPos {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_is_safe_and_operational() {
        let config = PrinterConfig::default();
        assert_eq!(config.mode, "auto");
        assert_eq!(config.escpos_port, 9100);
        assert!(config.prefer_escpos);
        assert!(config.print_logo);
    }

    #[test]
    fn missing_or_invalid_configuration_falls_back_to_default() {
        let missing =
            std::env::temp_dir().join(format!("missing-printer-{}.json", uuid::Uuid::new_v4()));
        assert_eq!(load_config(&missing).escpos_port, 9100);
        let invalid =
            std::env::temp_dir().join(format!("invalid-printer-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&invalid, b"not-json").unwrap();
        assert_eq!(load_config(&invalid).mode, "auto");
        let _ = std::fs::remove_file(invalid);
    }

    #[test]
    fn escpos_builder_emits_all_control_sequences() {
        let bytes = EscPos::new()
            .init()
            .align(Align::Right)
            .double_height(true)
            .text("TESTE")
            .newline(1)
            .beep()
            .open_drawer()
            .cut(CutMode::Partial)
            .to_bytes()
            .to_vec();
        assert!(bytes.starts_with(&[0x1b, 0x40, 0x1b, 0x61, 0x02]));
        assert!(bytes.windows(5).any(|part| part == b"TESTE"));
        assert!(bytes.ends_with(&[0x1d, 0x56, 0x01]));
    }

    #[test]
    fn logo_conversion_rejects_malformed_content() {
        assert!(logo_to_escpos("invalid", 320).is_err());
        assert!(logo_to_escpos("data:image/webp;base64,not-base64", 320).is_err());
    }
}
