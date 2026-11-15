//! bicho en Windows. Cada animalito es una ventanita transparente, siempre
//! encima y sin botón en la barra de tareas (UpdateLayeredWindow): Windows
//! deja pasar el mouse por lo transparente, así que solo su dibujo agarra
//! clics. Los que viven dentro de una ventana se acomodan justo encima de
//! ella (no siempre encima de todo), para que otra ventana los pueda tapar.
//!
//! Lo que hay en el escritorio lo ve escritorio.rs; el ícono de la bandeja
//! con su menú, bandeja.rs. El cerebro (bicho.rs), el dibujo (arte.rs) y la
//! correteada (juego.rs) son los mismos de Linux.
//!
//! Lo que aquí no hay (todavía): las letras de las terminales, claudio con
//! Claude Code, las reacciones a discada y el baile con música.
//!
//!   bicho.exe             los suelta (doble clic; si ya andan, nada)
//!   bicho.exe michi baba  esos (si ya andan, los cambia)
//!   bicho.exe off         los guarda
//!   bicho.exe toggle      una cosa o la otra
//!   bicho.exe ver         cómo se ven, en la terminal

mod bandeja;
mod escritorio;

use super::*;
use escritorio::{Caja, Pantalla};
use std::cell::RefCell;
use windows_sys::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC,
    SelectObject,
};
use windows_sys::Win32::Media::timeBeginPeriod;
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode,
    GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleMode,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_LWIN, VK_RWIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(crate) const CLASE_BICHO: &str = "bicho";
pub(crate) const CLASE_CONTROL: &str = "bicho-control";
const TIMER: usize = 1;
/// con qué tanto de lo último que lo moviste se calcula el aventón
const VENTANA_AVIENTA: Duration = Duration::from_millis(90);
/// más lento que esto (px/s) al soltarlo es "dejarlo ahí", no aventarlo
const SOLTAR: f32 = 900.0;
/// tras moverse una ventana, cuánto sigue a 60 por segundo
const SE_MOVIO: f32 = 0.6;

/// Texto para Windows (UTF-16 con su cero al final).
pub(crate) fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Con la App, si no la está usando ya alguien más arriba (Windows a veces
/// llama a la ventana desde dentro de otra llamada: entonces se salta).
fn con_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut a| a.as_mut().map(f)))
}

fn instancia() -> windows_sys::Win32::Foundation::HINSTANCE {
    unsafe { GetModuleHandleW(std::ptr::null()) }
}

fn control() -> HWND {
    let c = w(CLASE_CONTROL);
    unsafe { FindWindowW(c.as_ptr(), std::ptr::null()) }
}

/// ¿Ya andan? (el que anda tiene su ventana de control)
fn anda() -> bool {
    !control().is_null()
}

fn guarda() {
    let c = control();
    if !c.is_null() {
        unsafe { PostMessageW(c, WM_CLOSE, 0, 0) };
        for _ in 0..40 {
            if !anda() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

/// Desde una terminal: que lo que imprima se vea ahí (sin consola propia,
/// para que el doble clic no abra una ventana negra).
fn consola() {
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            let h = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut modo = 0;
            if GetConsoleMode(h, &mut modo) != 0 {
                SetConsoleMode(h, modo | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }
}

fn uso() {
    let nombres: Vec<_> = arte::todas().iter().map(|e| e.nombre).collect();
    eprintln!(
        "\nuso: bicho [especie…] | off | toggle | ver\nespecies: {}",
        nombres.join(", ")
    );
}

pub(crate) fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match args.get(1).map(String::as_str) {
        None | Some("on") => suelta(&[]),
        Some("off") => {
            guarda();
            0
        }
        Some("toggle") => {
            if anda() {
                guarda();
                0
            } else {
                suelta(&[])
            }
        }
        Some("ver") => {
            consola();
            ver();
            0
        }
        Some("-h" | "--help" | "ayuda") => {
            consola();
            uso();
            0
        }
        Some(_) => {
            let pets = &args[1..];
            if let Some(malo) = pets.iter().find(|p| arte::especie(p).is_none()) {
                consola();
                eprintln!("\nbicho: no conozco a '{malo}'");
                uso();
                2
            } else {
                guarda();
                suelta(pets)
            }
        }
    };
    std::process::exit(code);
}

/// Los suelta (en este mismo proceso: aquí no hay terminal que esperar).
fn suelta(pets: &[String]) -> i32 {
    if anda() && pets.is_empty() {
        return 0;
    }
    std::panic::set_hook(Box::new(|info| bitacora(&format!("tronó: {info}"))));
    match vive(pets) {
        Ok(()) => 0,
        Err(e) => {
            bitacora(&e);
            1
        }
    }
}

fn es_de_noche() -> bool {
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    !(6..23).contains(&t.wHour)
}

/// Uno que traes agarrado con el mouse.
struct Arrastre {
    i: usize,
    /// de dónde lo agarraste (dentro de su lienzo, lógicos)
    agarre: (f32, f32),
    /// dónde lo agarraste en la pantalla, y cuándo (para saber si fue clic)
    inicio: (f32, f32, Instant),
    /// lo último que lo moviste: (cuándo, x, y) en la pantalla
    muestras: VecDeque<(Instant, f32, f32)>,
}

/// Un animalito en pantalla: su ventanita, su lienzo y su cabeza.
pub(crate) struct Mascota {
    pub(crate) especie: &'static arte::Especie,
    pub(crate) bicho: bicho::Bicho,
    hwnd: HWND,
    dc: HDC,
    bmp: HBITMAP,
    bits: *mut u8,
    /// pixeles de verdad por cuadrito del dibujo
    px: usize,
    colores: [u32; arte::COLORES],
    /// lo último que se pintó: si no cambia, no se vuelve a pintar
    pintado: Option<(arte::Grid, [u32; arte::COLORES])>,
    /// dónde está la ventanita (pixeles de verdad)
    pos: (i32, i32),
    /// su cuerpo en el lienzo (x, y, ancho, alto; lógicos): ahí es "encima"
    cuerpo: Option<(f32, f32, f32, f32)>,
    /// encima de qué ventana anda acomodado (None: encima de todo)
    acomodo: Option<HWND>,
    visible: bool,
}

impl Drop for Mascota {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
            DeleteDC(self.dc);
            DeleteObject(self.bmp);
        }
    }
}

pub(crate) struct App {
    pub(crate) mascotas: Vec<Mascota>,
    pub(crate) semilla: u64,
    pub(crate) juego: Option<juego::Juego>,
    pub(crate) cada_juego: f32,
    pub(crate) cfg: Config,
    control: HWND,
    icono: HICON,
    pantalla: Pantalla,
    mundo: mundo::Mundo,
    /// las ventanas que se ven, enteras, de arriba para abajo
    cajas: Vec<Caja>,
    cursor: Option<(f32, f32)>,
    arrastre: Option<Arrastre>,
    /// hay algo en pantalla completa: escondidos
    tapados: bool,
    ultimo: Instant,
    cada_mundo: f32,
    cada_resto: f32,
    se_movio: f32,
    colores_mtime: Option<SystemTime>,
}

fn vive(pets: &[String]) -> Result<(), String> {
    unsafe {
        // nunca dos a la vez
        let nombre = w("Local\\bicho-uno-solo");
        let candado = CreateMutexW(std::ptr::null(), 0, nombre.as_ptr());
        if candado.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
        // pixeles de verdad (si no, Windows lo estira borroso al 125 %)
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // que el reloj dé cuadros parejos (si no, de 15.6 en 15.6 ms)
        timeBeginPeriod(1);
        let hinst = instancia();
        let cursor = LoadCursorW(std::ptr::null_mut(), IDC_HAND);
        for (clase, proc_) in [
            (CLASE_BICHO, ventana_bicho as WndProc),
            (CLASE_CONTROL, ventana_control as WndProc),
        ] {
            let c = w(clase);
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc_),
                hInstance: hinst,
                lpszClassName: c.as_ptr(),
                hCursor: cursor,
                ..std::mem::zeroed()
            };
            RegisterClassW(&wc);
        }
        let (c, t) = (w(CLASE_CONTROL), w("bicho"));
        let control = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            c.as_ptr(),
            t.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if control.is_null() {
            return Err("no se pudo crear la ventana de control".into());
        }
        let mut cfg = config();
        if !pets.is_empty() {
            cfg.pets = pets.to_vec();
        }
        let icono = bandeja::icono(arte::especie(&cfg.pets[0]).unwrap_or(arte::todas()[0]));
        let mut app = App {
            mascotas: Vec::new(),
            semilla: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(7, |d| d.as_nanos() as u64),
            juego: None,
            cada_juego: 20.0,
            cfg,
            control,
            icono,
            pantalla: escritorio::pantalla(),
            mundo: mundo::Mundo::default(),
            cajas: Vec::new(),
            cursor: None,
            arrastre: None,
            tapados: false,
            ultimo: Instant::now(),
            cada_mundo: 0.0,
            cada_resto: 0.0,
            se_movio: 0.0,
            colores_mtime: mtime(&colores_path()),
        };
        app.crea();
        bandeja::pon(control, icono);
        APP.with(|a| *a.borrow_mut() = Some(app));
        SetTimer(control, TIMER, CUADRO.as_millis() as u32, None);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        bandeja::quita(control);
        // al soltar la App se cierran sus ventanitas
        APP.with(|a| a.borrow_mut().take());
        DestroyIcon(icono);
    }
    Ok(())
}

type WndProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// La ventana escondida: el reloj, el ícono de la bandeja y el "ya
/// guárdense" de `bicho off`.
unsafe extern "system" fn ventana_control(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    static EXPLORER: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let reinicio = *EXPLORER.get_or_init(|| {
        let n = w("TaskbarCreated");
        unsafe { RegisterWindowMessageW(n.as_ptr()) }
    });
    match msg {
        WM_TIMER => {
            if let Some(cuadro) = con_app(|a| {
                a.tick();
                a.cuadro()
            }) {
                unsafe { SetTimer(h, TIMER, cuadro.as_millis() as u32, None) };
            }
            0
        }
        bandeja::WM_BANDEJA => {
            let ev = lp as u32;
            if ev == WM_RBUTTONUP || ev == WM_LBUTTONUP {
                menu(h);
            }
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(h) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        // Explorer se reinició: el ícono se perdió
        m if m == reinicio && m != 0 => {
            if let Some(i) = con_app(|a| a.icono) {
                bandeja::pon(h, i);
            }
            0
        }
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

/// El menú de la bandeja. Mientras está abierto los animalitos siguen
/// andando (el menú tiene su propio ciclo, y ahí llega el reloj): por eso
/// no se agarra la App mientras está abierto.
fn menu(control: HWND) {
    let Some(m) = con_app(|a| a.arma_menu()) else {
        return;
    };
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        // sin esto el menú no se cierra al dar clic afuera
        SetForegroundWindow(control);
        let cmd = TrackPopupMenu(
            m,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            control,
            std::ptr::null(),
        );
        DestroyMenu(m);
        if cmd > 0 {
            con_app(|a| a.orden(cmd as usize));
        }
    }
}

/// La ventanita de un animalito: el mouse.
unsafe extern "system" fn ventana_bicho(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        // que darle clic no le quite el foco a tu ventana
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONDOWN => {
            unsafe { SetCapture(h) };
            con_app(|a| a.agarra(h));
            0
        }
        WM_MOUSEMOVE => {
            con_app(|a| a.mueve(h));
            0
        }
        WM_LBUTTONUP => {
            // ReleaseCapture manda WM_CAPTURECHANGED, que lo suelta
            unsafe { ReleaseCapture() };
            con_app(|a| a.suelta());
            0
        }
        WM_CAPTURECHANGED => {
            con_app(|a| a.suelta());
            0
        }
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

impl App {
    fn crea(&mut self) {
        let celda = self.cfg.escala as f32;
        let px = (celda * self.pantalla.escala).round().max(1.0) as usize;
        let (cw, ch) = (arte::ANCHO as f32 * celda, arte::ALTO as f32 * celda);
        let (pw, ph) = ((arte::ANCHO * px) as i32, (arte::ALTO * px) as i32);
        let clase = w(CLASE_BICHO);
        for (i, nombre) in self.cfg.pets.clone().iter().enumerate() {
            let Some(especie) = arte::especie(nombre) else {
                continue;
            };
            let Some((hwnd, dc, bmp, bits)) = (unsafe { ventanita(&clase, pw, ph) }) else {
                continue;
            };
            let semilla = self
                .semilla
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .wrapping_add(i as u64 * 0x2545_f491);
            let mut b = bicho::Bicho::new(celda, cw, ch, semilla);
            b.velocidad = self.cfg.velocidad;
            b.trepa = self.cfg.trepa;
            b.salta = especie.brinca;
            b.aparece(self.pantalla.tam.0);
            self.mascotas.push(Mascota {
                especie,
                bicho: b,
                hwnd,
                dc,
                bmp,
                bits,
                px,
                colores: colores(especie),
                pintado: None,
                pos: (i32::MIN, i32::MIN),
                cuerpo: None,
                acomodo: None,
                visible: false,
            });
        }
    }

    /// Otra vez desde cero (cambió la config, el tamaño o la escala).
    fn recrea(&mut self) {
        if self.arrastre.take().is_some() {
            unsafe { ReleaseCapture() };
        }
        self.juego = None;
        self.mascotas.clear();
        self.crea();
    }

    fn sal(&mut self) {
        unsafe { PostMessageW(self.control, WM_CLOSE, 0, 0) };
    }

    /// El lienzo lógico de un animalito → su ventanita en pixeles de verdad:
    /// pegada por abajo y centrada (al 125 % los cuadritos no miden justo
    /// lo lógico, y así las patas quedan donde van).
    fn a_pantalla(&self, p: &Mascota) -> (i32, i32) {
        let (b, s) = (&p.bicho, self.pantalla.escala);
        let (pw, ph) = ((arte::ANCHO * p.px) as i32, (arte::ALTO * p.px) as i32);
        (
            self.pantalla.origen.0 + ((b.x + b.cw / 2.0) * s).round() as i32 - pw / 2,
            self.pantalla.origen.1 + ((b.y - b.dy + b.ch) * s).round() as i32 - ph,
        )
    }

    fn cual(&self, h: HWND) -> Option<usize> {
        self.mascotas.iter().position(|p| p.hwnd == h)
    }

    fn agarra(&mut self, h: HWND) {
        let Some(i) = self.cual(h) else { return };
        if self.arrastre.is_some() {
            return;
        }
        let Some((gx, gy)) = escritorio::cursor(&self.pantalla) else {
            return;
        };
        let ahora = Instant::now();
        let b = &mut self.mascotas[i].bicho;
        let agarre = (gx - b.x, gy - (b.y - b.dy));
        b.agarra();
        self.arrastre = Some(Arrastre {
            i,
            agarre,
            inicio: (gx, gy, ahora),
            muestras: VecDeque::from([(ahora, gx, gy)]),
        });
    }

    fn mueve(&mut self, h: HWND) {
        let Some(i) = self.cual(h) else { return };
        let Some((gx, gy)) = escritorio::cursor(&self.pantalla) else {
            return;
        };
        let Some(a) = self.arrastre.as_mut().filter(|a| a.i == i) else {
            // nomás pasó por encima (si de veras se movió el mouse: Windows
            // también avisa cuando es él el que se mueve debajo)
            if self.cursor != Some((gx, gy)) {
                self.mascotas[i].bicho.mouse_se_movio();
            }
            return;
        };
        let ahora = Instant::now();
        a.muestras.push_back((ahora, gx, gy));
        while a.muestras.len() > 2 && ahora - a.muestras[0].0 > VENTANA_AVIENTA {
            a.muestras.pop_front();
        }
        let (x, y) = (gx - a.agarre.0, gy - a.agarre.1);
        self.mascotas[i].bicho.arrastra(x, y);
        // que se mueva ya, sin esperar al siguiente cuadro
        self.mueve_ventanita(i);
    }

    fn suelta(&mut self) {
        let Some(a) = self.arrastre.take() else {
            return;
        };
        let (gx, gy) = escritorio::cursor(&self.pantalla).unwrap_or((a.inicio.0, a.inicio.1));
        // ¿traías Ctrl (o la tecla de Windows)? se mete a la ventana
        let tecla = |k: u16| unsafe { GetAsyncKeyState(k as i32) } < 0;
        let adentro = tecla(VK_CONTROL) || tecla(VK_LWIN) || tecla(VK_RWIN);
        let b = &mut self.mascotas[a.i].bicho;
        let (x0, y0, t0) = a.inicio;
        if (gx - x0).hypot(gy - y0) < 6.0 && t0.elapsed() < Duration::from_millis(350) {
            // fue un clic: cariño
            b.avienta(0.0, 0.0);
            let der = b.x + b.cw / 2.0 < gx;
            b.cariño(der);
            return;
        }
        let ahora = Instant::now();
        let (vx, vy) = match a
            .muestras
            .iter()
            .find(|m| ahora - m.0 <= VENTANA_AVIENTA * 2)
        {
            Some(&(t, x, y)) if (ahora - t).as_secs_f32() > 0.005 => {
                let dt = (ahora - t).as_secs_f32();
                ((gx - x) / dt, (gy - y) / dt)
            }
            _ => (0.0, 0.0),
        };
        b.avienta(vx, vy);
        if adentro
            && vx.hypot(vy) < SOLTAR
            && let Some(v) = escritorio::ventana_en(&self.cajas, gx, gy)
        {
            self.mascotas[a.i].bicho.entra(v);
        }
    }

    fn lee_mundo(&mut self) {
        let alto = arte::ALTO as f32 * self.cfg.escala as f32;
        let ancho = arte::ANCHO as f32 * self.cfg.escala as f32;
        let (nuevo, cajas) = escritorio::mundo(&self.pantalla, alto, ancho);
        let movida = nuevo.ventanas.iter().any(|v| {
            self.mundo
                .ventanas
                .iter()
                .any(|r| r.id == v.id && (r.x, r.y, r.w, r.h) != (v.x, v.y, v.w, v.h))
        });
        if movida {
            self.se_movio = SE_MOVIO;
        }
        self.mundo = nuevo;
        self.cajas = cajas;
    }

    fn tick(&mut self) {
        let dt = self.ultimo.elapsed().as_secs_f32().min(0.25);
        self.ultimo = Instant::now();
        self.cada_mundo -= dt;
        self.cada_resto -= dt;
        self.se_movio = (self.se_movio - dt).max(0.0);
        let mut noche = None;
        if self.cada_resto <= 0.0 {
            self.cada_resto = 0.4;
            noche = Some(es_de_noche());
            // cambiaste la escala, la resolución o la barra de tareas
            let p = escritorio::pantalla();
            if p != self.pantalla {
                let otra_escala = p.escala != self.pantalla.escala;
                self.pantalla = p;
                if otra_escala {
                    self.recrea();
                }
            }
            let m = mtime(&colores_path());
            if m != self.colores_mtime {
                self.colores_mtime = m;
                for p in &mut self.mascotas {
                    p.colores = colores(p.especie);
                }
            }
            let tapados = escritorio::pantalla_completa();
            if tapados != self.tapados {
                self.tapados = tapados;
                for p in &mut self.mascotas {
                    p.visible = false;
                    unsafe { ShowWindow(p.hwnd, SW_HIDE) };
                    p.pintado = None;
                }
            }
            // otras "siempre encima" pudieron quedar arriba
            for p in &mut self.mascotas {
                p.acomodo = Some(std::ptr::null_mut());
            }
        }
        if self.tapados {
            return;
        }
        let pegados = self.mascotas.iter().any(|p| p.bicho.pegado());
        if self.cada_mundo <= 0.0 || pegados {
            self.cada_mundo = 0.4;
            self.lee_mundo();
        }
        let cursor = escritorio::cursor(&self.pantalla);
        // el cursor encima de su cuerpo (para el cariño y su casita)
        for p in &mut self.mascotas {
            p.bicho.encima = match (cursor, p.cuerpo) {
                (Some((cx, cy)), Some((x, y, w, h))) => {
                    let (bx, by) = (p.bicho.x + x, p.bicho.y - p.bicho.dy + y);
                    cx >= bx && cx < bx + w && cy >= by && cy < by + h
                }
                _ => false,
            };
        }
        self.cursor = cursor;
        let (w, h) = self.pantalla.tam;
        self.tick_juego(dt);
        let mut looks = Vec::with_capacity(self.mascotas.len());
        for p in &mut self.mascotas {
            p.bicho.musica = false;
            if let Some(n) = noche {
                p.bicho.noche = n;
            }
            looks.push(p.bicho.tick(dt, &self.mundo, w, h, self.cursor));
        }
        // si dos se topan en el mismo lugar, se saludan
        for i in 0..self.mascotas.len() {
            for j in i + 1..self.mascotas.len() {
                let (a, b) = (&self.mascotas[i].bicho, &self.mascotas[j].bicho);
                let (Some((xa, sa)), Some((xb, sb))) = (a.saludable(), b.saludable()) else {
                    continue;
                };
                if sa == sb && (xa - xb).abs() < a.cw * 0.7 {
                    self.mascotas[i].bicho.cariño(xb > xa);
                    self.mascotas[j].bicho.cariño(xa > xb);
                    looks[i] = self.mascotas[i].bicho.tick(0.0, &self.mundo, w, h, None);
                    looks[j] = self.mascotas[j].bicho.tick(0.0, &self.mundo, w, h, None);
                }
            }
        }
        for (i, look) in looks.iter().enumerate() {
            let g = arte::grid(self.mascotas[i].especie, look);
            self.pinta(i, &g);
            self.acomoda(i);
        }
    }

    fn rapido(&self) -> bool {
        self.arrastre.is_some()
            || self.se_movio > 0.0
            || self.mascotas.iter().any(|p| p.bicho.en_el_aire())
    }

    /// Cada cuánto dibujar: rápido si algo se mueve, a 30 si alguien anda
    /// haciendo algo, y a 10 si todos están quietos.
    fn cuadro(&self) -> Duration {
        if self.tapados {
            return Duration::from_millis(400);
        }
        if self.rapido() {
            return CUADRO_RAPIDO;
        }
        match self.mascotas.iter().map(|p| p.bicho.ritmo()).max() {
            Some(1..) => CUADRO,
            _ => CUADRO_QUIETO,
        }
    }

    fn mueve_ventanita(&mut self, i: usize) {
        let pos = self.a_pantalla(&self.mascotas[i]);
        let p = &mut self.mascotas[i];
        if pos != p.pos {
            p.pos = pos;
            unsafe {
                SetWindowPos(
                    p.hwnd,
                    std::ptr::null_mut(),
                    pos.0,
                    pos.1,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
        }
    }

    fn pinta(&mut self, i: usize, g: &arte::Grid) {
        let celda = self.cfg.escala as f32;
        let pos = self.a_pantalla(&self.mascotas[i]);
        let p = &mut self.mascotas[i];
        // su ventana no se ve (minimizada, en otro escritorio): nada
        let vacio = [[0; arte::ANCHO]; arte::ALTO];
        let g = if p.bicho.oculto { &vacio } else { g };
        p.cuerpo = arte::caja(g).map(|(x0, y0, x1, y1)| {
            (
                x0 as f32 * celda,
                y0 as f32 * celda,
                (x1 - x0 + 1) as f32 * celda,
                (y1 - y0 + 1) as f32 * celda,
            )
        });
        let igual = p
            .pintado
            .as_ref()
            .is_some_and(|(gg, c)| gg == g && c == &p.colores);
        if igual {
            if pos != p.pos {
                p.pos = pos;
                unsafe {
                    SetWindowPos(
                        p.hwnd,
                        std::ptr::null_mut(),
                        pos.0,
                        pos.1,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
            }
            return;
        }
        let (pw, ph) = (arte::ANCHO * p.px, arte::ALTO * p.px);
        let canvas = unsafe { std::slice::from_raw_parts_mut(p.bits, pw * ph * 4) };
        arte::pinta(g, &p.colores, p.px, canvas);
        p.pintado = Some((*g, p.colores));
        p.pos = pos;
        let destino = POINT { x: pos.0, y: pos.1 };
        let tam = SIZE {
            cx: pw as i32,
            cy: ph as i32,
        };
        let origen = POINT { x: 0, y: 0 };
        let mezcla = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        unsafe {
            UpdateLayeredWindow(
                p.hwnd,
                std::ptr::null_mut(),
                &destino,
                &tam,
                p.dc,
                &origen,
                0,
                &mezcla,
                ULW_ALPHA,
            );
            if !p.visible {
                ShowWindow(p.hwnd, SW_SHOWNOACTIVATE);
                p.visible = true;
            }
        }
    }

    /// Dónde va en el orden de las ventanas: el que vive dentro de una,
    /// justo encima de ella (y la que la tape, lo tapa); los demás, encima de
    /// todo. `acomodo` = Some(null) es "hay que volver a ponerlo".
    fn acomoda(&mut self, i: usize) {
        let p = &self.mascotas[i];
        let host = p
            .bicho
            .dentro
            .as_deref()
            .filter(|_| self.arrastre.as_ref().is_none_or(|a| a.i != i))
            .and_then(escritorio::hwnd);
        unsafe {
            match host {
                Some(v) => {
                    // ya está justo encima: nada
                    if p.acomodo == Some(v) && GetWindow(p.hwnd, GW_HWNDNEXT) == v {
                        return;
                    }
                    if p.acomodo.is_none_or(|a| a.is_null()) {
                        SetWindowPos(
                            p.hwnd,
                            HWND_NOTOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                    // encima de v = debajo de la que está encima de v (si esa
                    // es de las "siempre encima", hasta arriba de las normales)
                    let antes = GetWindow(v, GW_HWNDPREV);
                    let tras = if antes.is_null()
                        || antes == p.hwnd
                        || GetWindowLongPtrW(antes, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0
                    {
                        HWND_TOP
                    } else {
                        antes
                    };
                    SetWindowPos(
                        p.hwnd,
                        tras,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                    self.mascotas[i].acomodo = Some(v);
                }
                None => {
                    if p.acomodo.is_none() {
                        return;
                    }
                    SetWindowPos(
                        p.hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                    self.mascotas[i].acomodo = None;
                }
            }
        }
    }
}

/// Una ventanita transparente con su lienzo (un DIB de 32 bits, de arriba
/// para abajo: los mismos bytes que el ARGB8888 de Wayland).
unsafe fn ventanita(clase: &[u16], ancho: i32, alto: i32) -> Option<(HWND, HDC, HBITMAP, *mut u8)> {
    unsafe {
        let titulo = w("bicho");
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            clase.as_ptr(),
            titulo.as_ptr(),
            WS_POPUP,
            0,
            0,
            ancho,
            alto,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instancia(),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return None;
        }
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ancho,
            biHeight: -alto,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let mut bits = std::ptr::null_mut();
        let bmp = CreateDIBSection(
            std::ptr::null_mut(),
            &bi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        if bmp.is_null() || bits.is_null() || dc.is_null() {
            DestroyWindow(hwnd);
            return None;
        }
        SelectObject(dc, bmp);
        Some((hwnd, dc, bmp, bits as *mut u8))
    }
}
