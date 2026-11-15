//! Lo que hay en el escritorio de Windows: la pantalla principal (dónde
//! empieza, cuánto escala, cuánto ocupa la barra de tareas), las ventanas
//! (EnumWindows, de arriba para abajo) y el cursor.
//!
//! Todo se pasa a coordenadas lógicas de la pantalla, como en Linux: el
//! proceso es "per monitor v2" (ve pixeles de verdad) y aquí se divide entre
//! la escala.
//!
//! En Hyprland las ventanas no se enciman; aquí sí. Si otra tapa la orilla de
//! arriba de una ventana, a esa nomás le queda el tramo más largo que sigue
//! libre (y si no le queda donde quepa un animalito, no cuenta: los que
//! viven adentro se esconden hasta que se vea).

use crate::mundo::{Mundo, Ventana};
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetClassNameW, GetCursorPos, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, IsIconic, IsWindowVisible, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
};

/// La pantalla donde viven (la principal).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Pantalla {
    /// dónde empieza, en pixeles de verdad
    pub(crate) origen: (i32, i32),
    /// cuánto mide, en lógicos
    pub(crate) tam: (f32, f32),
    /// pixeles de verdad por pixel lógico (1.25 al 125 %)
    pub(crate) escala: f32,
    /// lo que ocupa la barra de tareas abajo y arriba (lógicos)
    pub(crate) abajo: f32,
    pub(crate) arriba: f32,
}

pub(crate) fn pantalla() -> Pantalla {
    unsafe {
        let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(mon, &mut info);
        let (mut dx, mut dy) = (96u32, 96u32);
        GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let escala = (dx.max(96) as f32 / 96.0).max(1.0);
        let (m, t) = (info.rcMonitor, info.rcWork);
        Pantalla {
            origen: (m.left, m.top),
            tam: (
                (m.right - m.left) as f32 / escala,
                (m.bottom - m.top) as f32 / escala,
            ),
            escala,
            abajo: (m.bottom - t.bottom).max(0) as f32 / escala,
            arriba: (t.top - m.top).max(0) as f32 / escala,
        }
    }
}

/// Una ventana entera, como se ve (en lógicos), para saber cuál está debajo
/// de un punto.
#[derive(Clone, Debug)]
pub(crate) struct Caja {
    pub(crate) id: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl Caja {
    fn tiene(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

pub(crate) fn id(hwnd: HWND) -> String {
    format!("{:#x}", hwnd as usize)
}

pub(crate) fn hwnd(id: &str) -> Option<HWND> {
    usize::from_str_radix(id.strip_prefix("0x")?, 16)
        .ok()
        .map(|n| n as HWND)
}

fn clase(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Lo que no es una ventana de alguien: el fondo, la barra, los animalitos.
const NO_CUENTAN: &[&str] = &[
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "NotifyIconOverflowWindow",
    "Windows.UI.Core.CoreWindow",
    "XamlExplorerHostIslandWindow",
    super::CLASE_BICHO,
    super::CLASE_CONTROL,
];

/// El rectángulo que se ve (sin la sombra invisible de Windows 10/11).
fn rect(hwnd: HWND) -> RECT {
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        let ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut r as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        );
        if ok != 0 {
            GetWindowRect(hwnd, &mut r);
        }
        r
    }
}

/// Escondida por el sistema: en otro escritorio virtual, o una app de la
/// tienda suspendida.
fn encubierta(hwnd: HWND) -> bool {
    let mut c = 0u32;
    unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, (&mut c as *mut u32).cast(), 4);
    }
    c != 0
}

unsafe extern "system" fn junta(hwnd: HWND, lp: LPARAM) -> i32 {
    let v = unsafe { &mut *(lp as *mut Vec<HWND>) };
    v.push(hwnd);
    1
}

/// Todas las ventanas de arriba, de la de hasta arriba a la de hasta abajo.
fn todas() -> Vec<HWND> {
    let mut v: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(Some(junta), &mut v as *mut Vec<HWND> as LPARAM);
    }
    v
}

/// ¿Es una ventana de alguien (con título, de las que van en la barra)?
fn es_de_alguien(hwnd: HWND) -> bool {
    unsafe {
        if IsWindowVisible(hwnd) == 0 {
            return false;
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if ex & (WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE) != 0 {
            return false;
        }
        // GetWindowText no le manda mensaje a las de otros procesos (no se
        // atora si una app está trabada)
        let mut t = [0u16; 2];
        if GetWindowTextW(hwnd, t.as_mut_ptr(), 2) == 0 {
            return false;
        }
    }
    !NO_CUENTAN.contains(&clase(hwnd).as_str())
}

/// Los tramos de [x0, x1] que no tapa ninguno de `encima`, en la franja
/// [y0, y1].
fn libres(x0: f32, x1: f32, y0: f32, y1: f32, encima: &[Caja]) -> Vec<(f32, f32)> {
    let mut tramos = vec![(x0, x1)];
    for c in encima {
        if c.y >= y1 || c.y + c.h <= y0 {
            continue;
        }
        let (cx0, cx1) = (c.x, c.x + c.w);
        tramos = tramos
            .into_iter()
            .flat_map(|(a, b)| {
                if cx1 <= a || cx0 >= b {
                    return vec![(a, b)];
                }
                let mut r = Vec::new();
                if cx0 > a {
                    r.push((a, cx0));
                }
                if cx1 < b {
                    r.push((cx1, b));
                }
                r
            })
            .collect();
    }
    tramos
}

/// El mundo de ahorita. `alto` es lo que mide un animalito parado (lógicos):
/// la franja arriba de cada ventana que tiene que estar libre para pararse.
/// También regresa todas las ventanas que se ven, enteras y de arriba para
/// abajo.
pub(crate) fn mundo(p: &Pantalla, alto: f32, ancho: f32) -> (Mundo, Vec<Caja>) {
    let (ox, oy) = p.origen;
    let mut m = Mundo {
        abajo: p.abajo,
        arriba: p.arriba,
        borde: 0.0,
        ..Mundo::default()
    };
    let mut cajas: Vec<Caja> = Vec::new();
    for hwnd in todas() {
        if !es_de_alguien(hwnd) {
            continue;
        }
        let id = id(hwnd);
        // minimizada o en otro escritorio: existe, pero no se ve
        m.todas.push(id.clone());
        if unsafe { IsIconic(hwnd) } != 0 || encubierta(hwnd) {
            continue;
        }
        let r = rect(hwnd);
        let c = Caja {
            id,
            x: (r.left - ox) as f32 / p.escala,
            y: (r.top - oy) as f32 / p.escala,
            w: (r.right - r.left) as f32 / p.escala,
            h: (r.bottom - r.top) as f32 / p.escala,
        };
        if c.w < 1.0 || c.h < 1.0 {
            continue;
        }
        // en otra pantalla
        if c.x >= p.tam.0 || c.y >= p.tam.1 || c.x + c.w <= 0.0 || c.y + c.h <= 0.0 {
            continue;
        }
        // en pantalla completa no hay de dónde agarrarse (y tapa todo lo de
        // abajo)
        let completa = c.x <= 0.0 && c.y <= 0.0 && c.x + c.w >= p.tam.0 && c.y + c.h >= p.tam.1;
        if !completa {
            // lo más largo que queda libre de su orilla de arriba
            let mejor = libres(c.x, c.x + c.w, c.y - alto, c.y, &cajas)
                .into_iter()
                .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)));
            if let Some((x0, x1)) = mejor.filter(|(a, b)| b - a >= ancho) {
                m.ventanas.push(Ventana {
                    id: c.id.clone(),
                    x: x0,
                    y: c.y,
                    w: x1 - x0,
                    h: c.h,
                });
            }
        }
        cajas.push(c);
    }
    (m, cajas)
}

/// La ventana que se ve en (x, y) (lógicos).
pub(crate) fn ventana_en(cajas: &[Caja], x: f32, y: f32) -> Option<String> {
    cajas.iter().find(|c| c.tiene(x, y)).map(|c| c.id.clone())
}

pub(crate) fn cursor(p: &Pantalla) -> Option<(f32, f32)> {
    let mut pt = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut pt) } != 0).then(|| {
        (
            (pt.x - p.origen.0) as f32 / p.escala,
            (pt.y - p.origen.1) as f32 / p.escala,
        )
    })
}

/// ¿Hay algo en pantalla completa (un juego, un video, una presentación)?
/// Entonces mejor no estorbar.
pub(crate) fn pantalla_completa() -> bool {
    let mut q = 0;
    unsafe { SHQueryUserNotificationState(&mut q) };
    matches!(
        q,
        QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caja(x: f32, y: f32, w: f32, h: f32) -> Caja {
        Caja {
            id: String::new(),
            x,
            y,
            w,
            h,
        }
    }

    #[test]
    fn lo_que_tapa_otra_ventana_no_cuenta() {
        let encima = [caja(100.0, 0.0, 50.0, 500.0)];
        assert_eq!(
            libres(0.0, 300.0, 140.0, 200.0, &encima),
            vec![(0.0, 100.0), (150.0, 300.0)]
        );
        // fuera de la franja no tapa
        assert_eq!(
            libres(0.0, 300.0, 600.0, 650.0, &encima),
            vec![(0.0, 300.0)]
        );
    }
}
