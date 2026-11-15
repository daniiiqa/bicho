//! El ícono de la bandeja (junto al reloj) y su menú: qué animalitos andan,
//! de qué tamaño, si trepan, si bailan, el contraste, si son caga palos, si
//! arrancan con Windows, actualizarse y guardarlos. Hace lo que en Linux hace
//! `bicho menu`; lo que cambies se guarda en %APPDATA%\bicho\config.json.
//! También los avisos (la notificación del ícono).

use super::{App, actualiza, w};
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
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, HICON, HMENU, ICONINFO, MF_CHECKED,
    MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, SM_CXSMICON, WM_APP,
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
const BAILA: usize = 403;
const CONTRASTE: usize = 404;
const TRAVIESO: usize = 405;
const ACTUALIZA: usize = 406;
const NOMBRE: usize = 500;

/// ¿Es "ponerle nombre" a alguno? Cuál.
pub(crate) fn es_nombre(cmd: usize) -> Option<usize> {
    (NOMBRE..NOMBRE + 100).contains(&cmd).then(|| cmd - NOMBRE)
}
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

/// Un aviso del ícono (en Windows 11 sale como notificación).
pub(crate) fn avisa(control: HWND, titulo: &str, texto: &str) {
    let mut d = datos(control, std::ptr::null_mut());
    d.uFlags = NIF_INFO;
    d.dwInfoFlags = NIIF_INFO;
    for (i, c) in titulo
        .encode_utf16()
        .take(d.szInfoTitle.len() - 1)
        .enumerate()
    {
        d.szInfoTitle[i] = c;
    }
    for (i, c) in texto.encode_utf16().take(d.szInfo.len() - 1).enumerate() {
        d.szInfo[i] = c;
    }
    unsafe { Shell_NotifyIconW(NIM_MODIFY, &d) };
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
                let n = self
                    .cfg
                    .pets
                    .iter()
                    .filter(|p| crate::parte(p).0 == e.nombre)
                    .count();
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
            // ponerles nombre: cada uno de los que andan
            let nombres = CreatePopupMenu();
            for (i, p) in self.mascotas.iter().enumerate() {
                let texto = match &p.nombre {
                    Some(n) => format!("{n} ({})", p.especie.nombre),
                    None => format!("{} — {}", p.especie.nombre, t("sin nombre", "no name")),
                };
                agrega(nombres, MF_STRING, NOMBRE + i, &texto);
            }
            agrega(m, MF_POPUP, nombres as usize, t("Nombres", "Names"));
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
                MF_STRING | si(self.cfg.baila),
                BAILA,
                t("Bailan con música", "Dance to music"),
            );
            agrega(
                m,
                MF_STRING | si(self.cfg.contraste),
                CONTRASTE,
                t("Contraste (que no se pierdan)", "Contrast (don't get lost)"),
            );
            agrega(
                m,
                MF_STRING | si(self.cfg.travieso),
                TRAVIESO,
                t("Caga palos", "Troublemakers"),
            );
            agrega(
                m,
                MF_STRING | si(al_iniciar()),
                INICIO,
                t("Arrancar con Windows", "Start with Windows"),
            );
            agrega(m, MF_SEPARATOR, 0, "");
            let actualizar = match &self.nueva {
                Some(c) => format!(
                    "{} ({})",
                    t("Actualizar a la nueva", "Update to the new one"),
                    actualiza::corto(c)
                ),
                None => t("Buscar actualización", "Check for updates").to_string(),
            };
            let ocupado = if self.actualizando { MF_GRAYED } else { 0 };
            agrega(m, MF_STRING | ocupado, ACTUALIZA, &actualizar);
            let version = format!(
                "bicho {}",
                actualiza::COMMIT
                    .map_or(t("(compilado a mano)", "(local build)"), actualiza::corto)
            );
            agrega(m, MF_STRING | MF_GRAYED, 0, &version);
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
        // lo que no necesita volver a crearlos
        let solo_guarda = |a: &mut App| {
            guarda_config(&a.cfg);
            for p in &mut a.mascotas {
                p.bicho.travieso = a.cfg.travieso;
            }
        };
        match cmd {
            BAILA => {
                self.cfg.baila = !self.cfg.baila;
                return solo_guarda(self);
            }
            CONTRASTE => {
                self.cfg.contraste = !self.cfg.contraste;
                // que lo vea ya
                self.cada_contraste = 0.0;
                return solo_guarda(self);
            }
            TRAVIESO => {
                self.cfg.travieso = !self.cfg.travieso;
                return solo_guarda(self);
            }
            ACTUALIZA => {
                avisa(
                    self.control,
                    "bicho",
                    t("Buscando versión nueva…", "Looking for a new version…"),
                );
                self.revisa(true);
                return;
            }
            c if (ESPECIE..ESPECIE + todas.len()).contains(&c) => {
                let nombre = todas[c - ESPECIE].nombre.to_string();
                let es_de = |p: &String| crate::parte(p).0 == nombre;
                if pets.iter().any(es_de) {
                    // que quede al menos uno
                    if pets.iter().any(|p| !es_de(p)) {
                        pets.retain(|p| !es_de(p));
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
        (self.cfg.pets, self.cfg.escala, self.cfg.trepa) = (pets, escala, trepa);
        guarda_config(&self.cfg);
        self.recrea();
    }
}

/// Escribe la config (lo que no se toca aquí, se queda como estaba).
pub(crate) fn guarda_config(cfg: &crate::Config) {
    let archivo = crate::config_dir().join("config.json");
    let mut v: Value = std::fs::read(&archivo)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    v["pets"] = json!(cfg.pets);
    v["scale"] = json!(cfg.escala);
    v["climb"] = json!(cfg.trepa);
    v["dance"] = json!(cfg.baila);
    v["contrast"] = json!(cfg.contraste);
    v["mischief"] = json!(cfg.travieso);
    let _ = std::fs::create_dir_all(crate::config_dir());
    if let Ok(s) = serde_json::to_string_pretty(&v) {
        let _ = std::fs::write(&archivo, s + "\n");
    }
}
