//! El ícono de la bandeja (junto al reloj) y su menú: qué animalitos andan,
//! de qué tamaño, si trepan, si arrancan con Windows, y guardarlos. Hace lo
//! que en Linux hace `bicho menu`; lo que cambies se guarda en
//! %APPDATA%\bicho\config.json.

use super::{App, w};
use crate::arte;
use serde_json::{Value, json};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject,
};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, HICON, HMENU, ICONINFO, MF_CHECKED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, SM_CXSMICON, WM_APP,
};

/// Lo que manda el ícono cuando le das clic.
pub(crate) const WM_BANDEJA: u32 = WM_APP + 1;

// los comandos del menú
const ESPECIE: usize = 100;
const OTRO: usize = 200;
const TAMANO: usize = 300;
pub(crate) const TREPA: usize = 400;
pub(crate) const INICIO: usize = 401;
pub(crate) const GUARDA: usize = 402;
const TAMANOS: [u32; 5] = [2, 3, 4, 5, 6];

/// ¿Windows está en español?
pub(crate) fn es() -> bool {
    unsafe { GetUserDefaultUILanguage() & 0x3ff == 0x0a }
}

fn t<'a>(es: &'a str, en: &'a str) -> &'a str {
    if self::es() { es } else { en }
}

/// El ícono: la cara del primer animalito, con sus colores.
pub(crate) fn icono(e: &arte::Especie) -> HICON {
    let look = arte::Look {
        pose: arte::Pose::Parado,
        derecha: true,
        parpadeo: false,
        corazon: None,
        zzz: None,
        notas: None,
        piensa: None,
        alerta: None,
        reloj: None,
        casa: false,
        forma: arte::Forma::Normal,
    };
    let g = arte::grid(e, &look);
    let colores = crate::colores(e);
    let lado = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics(SM_CXSMICON).max(16)
    } as usize;
    let (x0, y0, x1, y1) = arte::caja(&g).unwrap_or((0, 0, arte::ANCHO - 1, arte::ALTO - 1));
    let (bw, bh) = (x1 - x0 + 1, y1 - y0 + 1);
    let px = (lado / bw.max(bh)).max(1);
    let (dx, dy) = (
        (lado.saturating_sub(bw * px)) / 2,
        (lado.saturating_sub(bh * px)) / 2,
    );
    unsafe {
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: lado as i32,
            biHeight: -(lado as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits = std::ptr::null_mut();
        let color = CreateDIBSection(
            std::ptr::null_mut(),
            &bi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if color.is_null() || bits.is_null() {
            return std::ptr::null_mut();
        }
        let buf = std::slice::from_raw_parts_mut(bits as *mut u32, lado * lado);
        for y in 0..bh {
            for x in 0..bw {
                let c = g[y0 + y][x0 + x];
                if c == 0 {
                    continue;
                }
                for py in 0..px {
                    for pxx in 0..px {
                        let (cx, cy) = (dx + x * px + pxx, dy + y * px + py);
                        if cx < lado && cy < lado {
                            buf[cy * lado + cx] = colores[c as usize];
                        }
                    }
                }
            }
        }
        let mascara = CreateBitmap(lado as i32, lado as i32, 1, 1, std::ptr::null());
        let info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mascara,
            hbmColor: color,
        };
        let i = CreateIconIndirect(&info);
        DeleteObject(color);
        DeleteObject(mascara);
        i
    }
}

fn datos(control: HWND, icono: HICON) -> NOTIFYICONDATAW {
    let mut d: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    d.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = control;
    d.uID = 1;
    d.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    d.uCallbackMessage = WM_BANDEJA;
    d.hIcon = icono;
    for (i, c) in "bicho".encode_utf16().enumerate() {
        d.szTip[i] = c;
    }
    d
}

/// Pone el ícono (también cuando Explorer se reinicia y lo pierde).
pub(crate) fn pon(control: HWND, icono: HICON) {
    let d = datos(control, icono);
    unsafe { Shell_NotifyIconW(NIM_ADD, &d) };
}

pub(crate) fn quita(control: HWND) {
    let d = datos(control, std::ptr::null_mut());
    unsafe { Shell_NotifyIconW(NIM_DELETE, &d) };
}

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// ¿Arranca al iniciar sesión? (una entrada en Run, en el registro del
/// usuario)
pub(crate) fn al_iniciar() -> bool {
    let (k, v) = (w(RUN), w("bicho"));
    let mut tam = 0u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            k.as_ptr(),
            v.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut tam,
        ) == 0
    }
}

pub(crate) fn pon_al_iniciar(si: bool) {
    let (k, v) = (w(RUN), w("bicho"));
    unsafe {
        if si {
            let Ok(exe) = std::env::current_exe() else {
                return;
            };
            let dato = w(&format!("\"{}\"", exe.display()));
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                k.as_ptr(),
                v.as_ptr(),
                REG_SZ,
                dato.as_ptr().cast(),
                (dato.len() * 2) as u32,
            );
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, k.as_ptr(), v.as_ptr());
        }
    }
}

fn agrega(m: HMENU, flags: u32, id: usize, texto: &str) {
    let t = w(texto);
    unsafe { AppendMenuW(m, flags, id, t.as_ptr()) };
}

impl App {
    /// El menú, con lo de ahorita palomeado.
    pub(crate) fn arma_menu(&self) -> HMENU {
        unsafe {
            let m = CreatePopupMenu();
            for (i, e) in arte::todas().iter().enumerate() {
                let n = self.cfg.pets.iter().filter(|p| *p == e.nombre).count();
                let flags = MF_STRING | if n > 0 { MF_CHECKED } else { 0 };
                let nombre = if n > 1 {
                    format!("{} ×{n}", e.nombre)
                } else {
                    e.nombre.to_string()
                };
                agrega(m, flags, ESPECIE + i, &nombre);
            }
            let otro = CreatePopupMenu();
            for (i, e) in arte::todas().iter().enumerate() {
                agrega(otro, MF_STRING, OTRO + i, e.nombre);
            }
            agrega(m, MF_POPUP, otro as usize, t("Uno más", "One more"));
            agrega(m, MF_SEPARATOR, 0, "");
            let tam = CreatePopupMenu();
            for (i, n) in TAMANOS.iter().enumerate() {
                let flags = MF_STRING | if *n == self.cfg.escala { MF_CHECKED } else { 0 };
                agrega(tam, flags, TAMANO + i, &format!("×{n}"));
            }
            agrega(m, MF_POPUP, tam as usize, t("Tamaño", "Size"));
            let si = |b: bool| if b { MF_CHECKED } else { 0 };
            agrega(
                m,
                MF_STRING | si(self.cfg.trepa),
                TREPA,
                t("Trepan ventanas", "Climb windows"),
            );
            agrega(
                m,
                MF_STRING | si(al_iniciar()),
                INICIO,
                t("Arrancar con Windows", "Start with Windows"),
            );
            agrega(m, MF_SEPARATOR, 0, "");
            agrega(m, MF_STRING, GUARDA, t("Guardarlos", "Put them away"));
            m
        }
    }

    /// Lo que escogiste en el menú.
    pub(crate) fn orden(&mut self, cmd: usize) {
        let todas = arte::todas();
        let mut pets = self.cfg.pets.clone();
        let (mut escala, mut trepa) = (self.cfg.escala, self.cfg.trepa);
        match cmd {
            c if (ESPECIE..ESPECIE + todas.len()).contains(&c) => {
                let nombre = todas[c - ESPECIE].nombre.to_string();
                if pets.contains(&nombre) {
                    // que quede al menos uno
                    if pets.iter().any(|p| *p != nombre) {
                        pets.retain(|p| *p != nombre);
                    }
                } else {
                    pets.push(nombre);
                }
            }
            c if (OTRO..OTRO + todas.len()).contains(&c) => {
                pets.push(todas[c - OTRO].nombre.to_string())
            }
            c if (TAMANO..TAMANO + TAMANOS.len()).contains(&c) => escala = TAMANOS[c - TAMANO],
            TREPA => trepa = !trepa,
            INICIO => {
                pon_al_iniciar(!al_iniciar());
                return;
            }
            GUARDA => {
                self.sal();
                return;
            }
            _ => return,
        }
        guarda_config(&pets, escala, trepa);
        self.cfg = crate::config();
        self.recrea();
    }
}

/// Escribe la config (lo que no se toca aquí, se queda como estaba).
fn guarda_config(pets: &[String], escala: u32, trepa: bool) {
    let archivo = crate::config_dir().join("config.json");
    let mut v: Value = std::fs::read(&archivo)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    v["pets"] = json!(pets);
    v["scale"] = json!(escala);
    v["climb"] = json!(trepa);
    let _ = std::fs::create_dir_all(crate::config_dir());
    if let Ok(s) = serde_json::to_string_pretty(&v) {
        let _ = std::fs::write(&archivo, s + "\n");
    }
}
