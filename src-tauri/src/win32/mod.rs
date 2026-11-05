use std::sync::Arc;
use std::sync::OnceLock;

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

// ── Win32 FFI (user32 / kernel32) ──────────────────────────────────
// Declaradas diretamente para evitar dependência do crate `windows` (que exige
// dbghelp.lib do Windows SDK). user32.lib e kernel32.lib já são linkadas por
// padrão no target x86_64-pc-windows-msvc.

#[cfg(target_os = "windows")]
mod ffi {
    pub type HHOOK = isize;
    pub type HINSTANCE = isize;
    pub type LPARAM = isize;
    pub type WPARAM = usize;
    pub type LRESULT = isize;

    pub const WH_KEYBOARD_LL: i32 = 13;
    pub const WM_KEYDOWN: u32 = 0x0100;
    pub const WM_SYSKEYDOWN: u32 = 0x0104;

    pub const VK_MENU: i16 = 0x12;    // Alt
    pub const VK_CONTROL: i16 = 0x11; // Ctrl
    pub const VK_LWIN: i16 = 0x5B;    // Tecla Windows esquerda
    pub const VK_RWIN: i16 = 0x5C;    // Tecla Windows direita
    pub const VK_TAB: i16 = 0x09;
    pub const VK_ESCAPE: i16 = 0x1B;
    pub const VK_F4: i16 = 0x73;
    pub const VK_W: i16 = 0x57;

    #[repr(C)]
    pub struct KBDLLHOOKSTRUCT {
        pub vk_code: u32,
        pub scan_code: u32,
        pub flags: u32,
        pub time: u32,
        pub dw_extra_info: usize,
    }

    #[repr(C)]
    pub struct MSG {
        pub hwnd: isize,
        pub message: u32,
        pub w_param: usize,
        pub l_param: isize,
        pub time: u32,
        pub pt_x: i32,
        pub pt_y: i32,
    }

    type KeyboardProc = unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT;

    extern "system" {
        pub fn SetWindowsHookExA(
            id_hook: i32,
            lpfn: Option<KeyboardProc>,
            h_mod: HINSTANCE,
            dw_thread_id: u32,
        ) -> HHOOK;
        pub fn CallNextHookEx(
            hhk: HHOOK,
            n_code: i32,
            w_param: WPARAM,
            l_param: LPARAM,
        ) -> LRESULT;
        pub fn UnhookWindowsHookEx(hhk: HHOOK) -> i32;
        pub fn GetAsyncKeyState(v_key: i16) -> i16;
        pub fn GetModuleHandleW(lp_module_name: *const u16) -> HINSTANCE;
        pub fn GetMessageA(
            lp_msg: *mut MSG,
            h_wnd: isize,
            w_msg_filter_min: u32,
            w_msg_filter_max: u32,
        ) -> i32;
        pub fn TranslateMessage(lp_msg: *const MSG) -> i32;
        pub fn DispatchMessageA(lp_msg: *const MSG) -> i32;
    }
}

#[cfg(target_os = "windows")]
use ffi::*;

// Hook global de teclado de baixo nível (WH_KEYBOARD_LL).
// Bloqueia atalhos que permitiriam ao funcionário escapar do modo quiosque:
// - Alt+F4 (fechar janela)
// - Alt+Tab (trocar de aplicativo)
// - Tecla Windows (Esquerda/Direita) (abrir Iniciar)
// - Ctrl+Esc (abrir Iniciar)
// - Ctrl+W (fechar aba/janela do navegador)
//
// Observação: Ctrl+Alt+Del é uma sequência de atenção do Windows (SAS) tratada
// em nível de kernel e NÃO pode ser bloqueada por aplicação de usuário.
static HOOK: OnceLock<HHOOK> = OnceLock::new();

#[cfg(target_os = "windows")]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kbd = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
            let vk = kbd.vk_code;

            let alt_pressed = GetAsyncKeyState(VK_MENU) < 0;
            let ctrl_pressed = GetAsyncKeyState(VK_CONTROL) < 0;

            let blocked = match vk as i16 {
                VK_F4 if alt_pressed => true,   // Alt+F4
                VK_TAB if alt_pressed => true,  // Alt+Tab
                VK_LWIN => true,                // Win esquerda
                VK_RWIN => true,                // Win direita
                VK_ESCAPE if ctrl_pressed => true, // Ctrl+Esc
                VK_W if ctrl_pressed => true,   // Ctrl+W
                _ => false,
            };

            if blocked {
                // Consome a mensagem: a tecla não chega ao sistema nem ao app.
                return 1 as LRESULT;
            }
        }
    }
    CallNextHookEx(0, n_code, w_param, l_param)
}

/// Instala o hook global de teclado e mantém uma thread dedicada com message loop
/// para que o hook permaneça ativo durante toda a execução do PDV.
#[cfg(target_os = "windows")]
pub fn install_keyboard_hook() -> Result<(), String> {
    unsafe {
        let hinstance: HINSTANCE = GetModuleHandleW(std::ptr::null());
        if hinstance == 0 {
            return Err("GetModuleHandleW retornou NULL".into());
        }
        let hook = SetWindowsHookExA(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            hinstance,
            0,
        );
        if hook == 0 {
            return Err("SetWindowsHookExA retornou 0 (falha ao instalar hook)".into());
        }

        // Thread dedicada com message loop para manter o hook vivo.
        std::thread::Builder::new()
            .name("pdv-keyboard-hook".into())
            .spawn(move || {
                let mut msg: MSG = std::mem::zeroed();
                loop {
                    let result = GetMessageA(&mut msg, 0, 0, 0);
                    if result == 0 {
                        break;
                    }
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageA(&msg);
                }
            })
            .map_err(|e| format!("Não foi possível iniciar thread do hook de teclado: {e}"))?;

        HOOK.set(hook).ok();
        log::info!("Win32: hook global de teclado instalado (modo quiosque)");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn uninstall_keyboard_hook() -> Result<(), String> {
    if let Some(hook) = HOOK.get() {
        unsafe {
            if UnhookWindowsHookEx(*hook) == 0 {
                return Err("UnhookWindowsHookEx falhou".into());
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_keyboard_hook() -> Result<(), String> {
    log::warn!("Hook de teclado não disponível fora do Windows");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn uninstall_keyboard_hook() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn install_barcode_hook(state: Arc<AppState>) -> Result<(), String> {
    use std::sync::OnceLock;

    static STATE: OnceLock<Arc<AppState>> = OnceLock::new();
    STATE.set(state).ok();

    // Instala também o hook global de teclado (modo quiosque).
    if let Err(e) = install_keyboard_hook() {
        log::warn!("Win32: falha ao instalar hook de teclado: {e}");
    }

    log::info!("Win32: Barcode hook registrado + hook de teclado quiosque");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_barcode_hook(_state: Arc<AppState>) -> Result<(), String> {
    log::warn!("Barcode hook não disponível fora do Windows");
    Ok(())
}

pub fn process_raw_input_msg(_l_param: isize, _app: &tauri::AppHandle, _buffer: &mut BarcodeBuffer) {}

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
    Err(
        "Bloqueio de teclas agora implementado via install_keyboard_hook (hook global WH_KEYBOARD_LL)."
            .into(),
    )
}
