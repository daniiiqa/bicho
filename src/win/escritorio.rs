//! Lo que hay en el escritorio de Windows: las pantallas (dónde empieza
//! cada una, cuánto escala, cuánto ocupa la barra de tareas), las ventanas
//! (EnumWindows, de arriba para abajo), el cursor y lo que se ve detrás de
//! un animalito (para el contraste).
//!
//! Cada animalito vive en una pantalla y ve todo en coordenadas lógicas de
//! ella, como en Linux: el proceso es "per monitor v2" (ve pixeles de verdad)
//! y aquí se divide entre la escala de esa pantalla.
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
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetMonitorInfoW, HDC,
    HMONITOR, MONITORINFO, ReleaseDC, SRCCOPY, SelectObject,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetCursorPos,
    GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextW, IsIconic,
    IsWindowVisible, IsZoomed, MONITORINFOF_PRIMARY, WS_CAPTION, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

/// Una pantalla.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Pantalla {
    /// dónde está, en pixeles de verdad (izq, arriba, der, abajo)
    pub(crate) rect: (i32, i32, i32, i32),
    /// cuánto mide, en lógicos
    pub(crate) tam: (f32, f32),
    /// pixeles de verdad por pixel lógico (1.25 al 125 %)
    pub(crate) escala: f32,
    /// lo que ocupa la barra de tareas abajo y arriba (lógicos)
    pub(crate) abajo: f32,
    pub(crate) arriba: f32,
}

impl Pantalla {
    /// Un punto de la pantalla (pixeles de verdad) → lógicos de esta.
    pub(crate) fn a_logico(&self, (x, y): (i32, i32)) -> (f32, f32) {
        (
            (x - self.rect.0) as f32 / self.escala,
            (y - self.rect.1) as f32 / self.escala,
        )
    }

    fn tiene(&self, (x, y): (i32, i32)) -> bool {
        x >= self.rect.0 && x < self.rect.2 && y >= self.rect.1 && y < self.rect.3
    }
}

unsafe extern "system" fn junta_monitor(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> i32 {
    let v = unsafe { &mut *(lp as *mut Vec<HMONITOR>) };
    v.push(m);
    1
}

/// Todas las pantallas; la principal primero, luego de izquierda a derecha.
pub(crate) fn pantallas() -> Vec<Pantalla> {
    let mut mons: Vec<HMONITOR> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(junta_monitor),
            &mut mons as *mut Vec<HMONITOR> as LPARAM,
        );
    }
    let mut v: Vec<(bool, Pantalla)> = mons
        .into_iter()
        .filter_map(|mon| unsafe {
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(mon, &mut info) == 0 {
                return None;
            }
            let (mut dx, mut dy) = (96u32, 96u32);
            GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            let escala = (dx.max(96) as f32 / 96.0).max(1.0);
            let (m, t) = (info.rcMonitor, info.rcWork);
            // con la barra en auto-ocultar el área de trabajo es toda la
            // pantalla: lo que mide la barra se ve en su ventana
            let (barra_abajo, barra_arriba) = barras(&m);
            Some((
                info.dwFlags & MONITORINFOF_PRIMARY != 0,
                Pantalla {
                    rect: (m.left, m.top, m.right, m.bottom),
                    tam: (
                        (m.right - m.left) as f32 / escala,
                        (m.bottom - m.top) as f32 / escala,
                    ),
                    escala,
                    abajo: (m.bottom - t.bottom).max(barra_abajo).max(0) as f32 / escala,
                    arriba: (t.top - m.top).max(barra_arriba).max(0) as f32 / escala,
                },
            ))
        })
        .collect();
    v.sort_by_key(|(principal, p)| (!principal, p.rect.0, p.rect.1));
    v.into_iter().map(|(_, p)| p).collect()
}

/// Lo que miden las barras de tareas pegadas abajo y arriba de esa pantalla
/// (pixeles de verdad), aunque estén escondidas (auto-ocultar): escondida,
/// la barra se sale casi toda por la orilla, pero sigue midiendo lo mismo.
/// Así nunca caminan donde la barra va a salir (y los taparía).
fn barras(m: &RECT) -> (i32, i32) {
    let (mut abajo, mut arriba) = (0, 0);
    for clase in ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"] {
        let c = w(clase);
        let mut h: HWND = std::ptr::null_mut();
        loop {
            h = unsafe { FindWindowExW(std::ptr::null_mut(), h, c.as_ptr(), std::ptr::null()) };
            if h.is_null() {
                break;
            }
            let mut r: RECT = unsafe { std::mem::zeroed() };
            unsafe { GetWindowRect(h, &mut r) };
            let grueso = r.bottom - r.top;
            // de lado a lado de esta pantalla (las de los lados no cuentan)
            let ancha = (r.left - m.left).abs() <= 4 && (r.right - m.right).abs() <= 4;
            if !ancha || grueso <= 0 || grueso > (m.bottom - m.top) / 3 {
                continue;
            }
            let medio = (m.top + m.bottom) / 2;
            if r.top >= medio && r.top <= m.bottom {
                abajo = abajo.max(grueso);
            } else if r.bottom <= medio && r.bottom >= m.top {
                arriba = arriba.max(grueso);
            }
        }
    }
    (abajo, arriba)
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// En qué pantalla está ese punto (pixeles de verdad).
pub(crate) fn pantalla_de(ps: &[Pantalla], punto: (i32, i32)) -> Option<usize> {
    ps.iter().position(|p| p.tiene(punto))
}

/// La pantalla pegada a la orilla derecha (o izquierda) de `i`, a la altura
/// `y` (pixeles de verdad); si a esa altura no hay, cualquiera pegada.
pub(crate) fn vecina(ps: &[Pantalla], i: usize, der: bool, y: i32) -> Option<usize> {
    let p = ps.get(i)?;
    let pegadas: Vec<usize> = (0..ps.len())
        .filter(|&j| j != i)
        .filter(|&j| {
            let q = &ps[j];
            let junto = if der {
                (q.rect.0 - p.rect.2).abs() <= 2
            } else {
                (q.rect.2 - p.rect.0).abs() <= 2
            };
            // que se toquen de alto
            junto && q.rect.1 < p.rect.3 && q.rect.3 > p.rect.1
        })
        .collect();
    pegadas
        .iter()
        .copied()
        .find(|&j| y >= ps[j].rect.1 && y < ps[j].rect.3)
        .or(pegadas.first().copied())
}

/// Una ventana entera, como se ve (en lógicos de una pantalla), para saber
/// cuál está debajo de un punto.
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

/// El mundo de ahorita en cada pantalla. `alto` es lo que mide un animalito
/// parado (lógicos): la franja arriba de cada ventana que tiene que estar
/// libre para pararse. Con cada mundo van todas las ventanas que se ven en
/// esa pantalla, enteras y de arriba para abajo.
pub(crate) fn mundos(ps: &[Pantalla], alto: f32, ancho: f32) -> Vec<(Mundo, Vec<Caja>)> {
    // una sola pasada por las ventanas
    let mut todas_ids = Vec::new();
    let mut visibles: Vec<(String, RECT)> = Vec::new();
    for hwnd in todas() {
        if !es_de_alguien(hwnd) {
            continue;
        }
        let id = id(hwnd);
        // minimizada o en otro escritorio: existe, pero no se ve
        todas_ids.push(id.clone());
        if unsafe { IsIconic(hwnd) } != 0 || encubierta(hwnd) {
            continue;
        }
        visibles.push((id, rect(hwnd)));
    }
    ps.iter()
        .map(|p| {
            let mut m = Mundo {
                abajo: p.abajo,
                arriba: p.arriba,
                borde: 0.0,
                origen: (p.rect.0 as f32, p.rect.1 as f32),
                todas: todas_ids.clone(),
                ..Mundo::default()
            };
            let mut cajas: Vec<Caja> = Vec::new();
            for (id, r) in &visibles {
                let (x, y) = p.a_logico((r.left, r.top));
                let c = Caja {
                    id: id.clone(),
                    x,
                    y,
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
                // en pantalla completa no hay de dónde agarrarse
                let completa =
                    c.x <= 0.0 && c.y <= 0.0 && c.x + c.w >= p.tam.0 && c.y + c.h >= p.tam.1;
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
        })
        .collect()
}

/// La ventana que se ve en (x, y) (lógicos de la pantalla de esas cajas).
pub(crate) fn ventana_en(cajas: &[Caja], x: f32, y: f32) -> Option<String> {
    cajas.iter().find(|c| c.tiene(x, y)).map(|c| c.id.clone())
}

/// El cursor, en pixeles de verdad.
pub(crate) fn cursor() -> Option<(i32, i32)> {
    let mut pt = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut pt) } != 0).then_some((pt.x, pt.y))
}

/// ¿En qué pantalla hay algo en pantalla completa (un juego, un video, una
/// presentación)? Ahí mejor no estorbar.
pub(crate) fn completa_en(ps: &[Pantalla]) -> Option<usize> {
    let fg = unsafe { GetForegroundWindow() };
    if fg.is_null() || NO_CUENTAN.contains(&clase(fg).as_str()) {
        return None;
    }
    // una maximizada (con su barra de título) no es pantalla completa,
    // aunque con la barra de tareas escondida cubra toda la pantalla
    let estilo = unsafe { GetWindowLongPtrW(fg, GWL_STYLE) } as u32;
    if unsafe { IsZoomed(fg) } != 0 && estilo & WS_CAPTION == WS_CAPTION {
        return None;
    }
    let mut r: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetWindowRect(fg, &mut r) };
    ps.iter().position(|p| {
        r.left <= p.rect.0 && r.top <= p.rect.1 && r.right >= p.rect.2 && r.bottom >= p.rect.3
    })
}

/// Qué tan clarito es lo que se ve en ese rectángulo de la pantalla (pixeles
/// de verdad), sin contar a los animalitos (sus ventanas en capas no salen
/// en la foto si no se pide CAPTUREBLT). 0 negro, 1 blanco.
pub(crate) fn luz_detras(x: i32, y: i32, w: i32, h: i32) -> Option<f32> {
    if w <= 0 || h <= 0 {
        return None;
    }
    unsafe {
        let pantalla = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(pantalla);
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits = std::ptr::null_mut();
        let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        let mut r = None;
        if !bmp.is_null() && !bits.is_null() {
            let viejo = SelectObject(dc, bmp);
            if BitBlt(dc, 0, 0, w, h, pantalla, x, y, SRCCOPY) != 0 {
                let px = std::slice::from_raw_parts(bits as *const u32, (w * h) as usize);
                // de cada cuarto pixel basta
                let (suma, n) = px
                    .iter()
                    .step_by(4)
                    .fold((0.0, 0u32), |(s, n), &c| (s + crate::arte::luz(c), n + 1));
                r = (n > 0).then(|| suma / n as f32);
            }
            SelectObject(dc, viejo);
            DeleteObject(bmp);
        }
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), pantalla);
        r
    }
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

    fn pantalla(rect: (i32, i32, i32, i32)) -> Pantalla {
        Pantalla {
            rect,
            tam: ((rect.2 - rect.0) as f32, (rect.3 - rect.1) as f32),
            escala: 1.0,
            abajo: 0.0,
            arriba: 0.0,
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

    #[test]
    fn las_pantallas_de_al_lado() {
        // laptop en medio, una a la derecha más alta, nada a la izquierda
        let ps = [
            pantalla((0, 0, 1920, 1080)),
            pantalla((1920, -200, 4480, 1240)),
        ];
        assert_eq!(vecina(&ps, 0, true, 500), Some(1));
        assert_eq!(vecina(&ps, 0, false, 500), None);
        assert_eq!(vecina(&ps, 1, false, 1200), Some(0));
        assert_eq!(pantalla_de(&ps, (2000, -100)), Some(1));
    }
}
