//! Caga palos: las travesuras (con "Caga palos" palomeado en el menú).
//!
//! - Se te escapan: le acercas rápido el cursor para agarrarlo y, a veces,
//!   brinca para el otro lado con un "!".
//! - Se roban el cursor: si uno que te anda persiguiendo te alcanza, lo
//!   agarra y se lo lleva corriendo un ratito. Si jalas fuerte, lo suelta
//!   (y se asusta).
//! - Pisotones: uno encima de una ventana brinca tres veces y con cada
//!   pisotón la ventana se sacude.
//!
//! Nunca mientras traes apretado un botón del mouse (no se meten con lo que
//! estés arrastrando), ni con ventanas maximizadas.

use super::{App, escritorio};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsZoomed, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetCursorPos, SetWindowPos,
};

/// Cada cuánto, más o menos, hacen una travesura grande (s).
const ENTRE: (f32, f32) = (70.0, 200.0);
/// Lo que dura el robo del cursor (s).
const ROBO: f32 = 1.6;
/// Si el cursor quedó más lejos que esto de donde lo dejó (px de verdad),
/// fue que jalaste.
const JALON: i32 = 40;
/// Qué tan cerca (lógicos) y qué tan rápido (lógicos/s) tiene que venir el
/// cursor para que se escape.
const ESCAPA_CERCA: f32 = 90.0;
const ESCAPA_RAPIDO: f32 = 700.0;
/// Después de escaparse, cuánto tarda en volver a hacerlo (s).
const ESCAPA_OTRA: f32 = 15.0;
/// Lo que se hunde la ventana con un pisotón (px de verdad) y lo que tarda
/// en regresar (s).
const PISOTON: i32 = 9;
const REGRESA: f32 = 0.08;

pub(crate) enum Travesura {
    /// se robó el cursor
    Roba {
        i: usize,
        t: f32,
        /// donde dejó el cursor el cuadro pasado
        puso: Option<(i32, i32)>,
    },
    /// brinca encima de la ventana
    Pisa {
        i: usize,
        ventana: HWND,
        t: f32,
        /// cuántas veces había caído (cada caída nueva es un pisotón)
        cayo: u32,
        /// la ventana va hundida: dónde estaba y lo que falta para regresarla
        hundida: Option<(RECT, f32)>,
    },
}

fn boton_apretado() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON as i32) < 0 || GetAsyncKeyState(VK_RBUTTON as i32) < 0 }
}

fn mueve_ventana(h: HWND, x: i32, y: i32) {
    unsafe {
        SetWindowPos(
            h,
            std::ptr::null_mut(),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

impl App {
    fn azar_t(&mut self) -> f32 {
        let mut x = self.semilla | 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.semilla = x;
        (x >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Un cuadro de travesuras (antes de que se muevan).
    pub(crate) fn tick_travesuras(&mut self, dt: f32) {
        if !self.cfg.travieso || self.arrastre.is_some() {
            if let Some(t) = self.travesura.take() {
                self.acaba(t);
            }
            return;
        }
        self.escapan(dt);
        match self.travesura.take() {
            Some(t) => {
                if let Some(t) = self.sigue(t, dt) {
                    self.travesura = Some(t);
                }
            }
            None => {
                self.cada_travesura -= dt;
                self.roba_si_alcanza();
                if self.travesura.is_none() && self.cada_travesura <= 0.0 {
                    self.cada_travesura = ENTRE.0 + self.azar_t() * (ENTRE.1 - ENTRE.0);
                    self.empieza_pisotones();
                }
            }
        }
    }

    /// Se te escapan si les llegas muy rápido con el cursor.
    fn escapan(&mut self, dt: f32) {
        let Some(c) = self.cursor_px else { return };
        let antes = self.cursor_px_antes.replace(c);
        for p in &mut self.mascotas {
            p.escapa_otra = (p.escapa_otra - dt).max(0.0);
        }
        let Some(antes) = antes else { return };
        if dt <= 0.0 || boton_apretado() {
            return;
        }
        for i in 0..self.mascotas.len() {
            let p = &self.mascotas[i];
            if p.escapa_otra > 0.0 || !p.bicho.puede_jugar() {
                continue;
            }
            let Some(ps) = self.pantallas.get(p.mon) else {
                continue;
            };
            let (cx, cy) = ps.a_logico(c);
            let (ax, ay) = ps.a_logico(antes);
            let b = &p.bicho;
            let (bx, by) = (b.x + b.cw / 2.0, b.y + b.ch / 2.0);
            let (d, d_antes) = ((cx - bx).hypot(cy - by), (ax - bx).hypot(ay - by));
            let rapido = (cx - ax).hypot(cy - ay) / dt > ESCAPA_RAPIDO;
            if d < ESCAPA_CERCA && d < d_antes && rapido {
                self.mascotas[i].escapa_otra = ESCAPA_OTRA;
                if self.azar_t() < 0.4 {
                    self.mascotas[i].bicho.esquiva(cx < bx);
                }
            }
        }
    }

    /// Uno que te persigue y te alcanza: se roba el cursor.
    fn roba_si_alcanza(&mut self) {
        let Some(c) = self.cursor_px else { return };
        if boton_apretado() {
            return;
        }
        for i in 0..self.mascotas.len() {
            let p = &self.mascotas[i];
            if !p.bicho.persigue() {
                continue;
            }
            let Some(ps) = self.pantallas.get(p.mon) else {
                continue;
            };
            let (cx, cy) = ps.a_logico(c);
            let b = &p.bicho;
            // el cursor a la altura de su cabeza, justo encima
            let cerca = (cx - (b.x + b.cw / 2.0)).abs() < b.cw * 0.6
                && cy > b.y - b.ch * 0.6
                && cy < b.y + b.ch;
            if cerca && self.azar_t() < 0.5 {
                let der = self.azar_t() < 0.5;
                self.mascotas[i].bicho.corre(der, ROBO + 1.0);
                self.travesura = Some(Travesura::Roba {
                    i,
                    t: 0.0,
                    puso: None,
                });
                return;
            }
        }
    }

    /// Uno encima de una ventana (no maximizada) le da de pisotones.
    fn empieza_pisotones(&mut self) {
        let candidatos: Vec<(usize, HWND)> = self
            .mascotas
            .iter()
            .enumerate()
            .filter(|(_, p)| p.bicho.puede_jugar())
            .filter_map(|(i, p)| {
                let (_, dentro, sobre) = p.bicho.suelo();
                let h = escritorio::hwnd(sobre?)?;
                (dentro.is_none() && unsafe { IsZoomed(h) } == 0).then_some((i, h))
            })
            .collect();
        if candidatos.is_empty() {
            // nadie arriba de una ventana: que lo intenten pronto
            self.cada_travesura = 15.0;
            return;
        }
        let (i, ventana) =
            candidatos[(self.azar_t() * candidatos.len() as f32) as usize % candidatos.len()];
        let medio = self
            .pantallas
            .get(self.mascotas[i].mon)
            .map_or(0.0, |p| p.tam.0 / 2.0);
        let b = &mut self.mascotas[i].bicho;
        let der = b.x < medio;
        b.llama(der);
        let cayo = b.aterrizajes;
        self.travesura = Some(Travesura::Pisa {
            i,
            ventana,
            t: 0.0,
            cayo,
            hundida: None,
        });
    }

    /// Un cuadro de la travesura en curso; None si ya se acabó.
    fn sigue(&mut self, t: Travesura, dt: f32) -> Option<Travesura> {
        match t {
            Travesura::Roba { i, t, puso } => {
                let t = t + dt;
                let p = self.mascotas.get(i)?;
                let ps = self.pantallas.get(p.mon)?;
                // ¿lo jalaste?
                if let (Some(c), Some(puso)) = (self.cursor_px, puso)
                    && ((c.0 - puso.0).abs() > JALON || (c.1 - puso.1).abs() > JALON)
                {
                    let der = self.mascotas[i].bicho.x < ps.a_logico(c).0;
                    self.mascotas[i].bicho.esquiva(!der);
                    return None;
                }
                if t > ROBO || boton_apretado() {
                    // ya, que se lo quede tantito y se ríe
                    let der = self.azar_t() < 0.5;
                    self.mascotas[i].bicho.cariño(der);
                    return None;
                }
                // en su cabeza
                let b = &p.bicho;
                let x = ps.rect.0 + ((b.x + b.cw / 2.0) * ps.escala).round() as i32;
                let y = ps.rect.1 + ((b.y - b.dy + b.ch * 0.15) * ps.escala).round() as i32;
                unsafe { SetCursorPos(x, y) };
                self.cursor_px = Some((x, y));
                self.cursor_px_antes = Some((x, y));
                Some(Travesura::Roba {
                    i,
                    t,
                    puso: Some((x, y)),
                })
            }
            Travesura::Pisa {
                i,
                ventana,
                t,
                cayo,
                mut hundida,
            } => {
                let t = t + dt;
                // la ventana hundida regresa a su lugar
                if let Some((r, falta)) = hundida.as_mut() {
                    *falta -= dt;
                    if *falta <= 0.0 {
                        mueve_ventana(ventana, r.left, r.top);
                        hundida = None;
                    }
                }
                let b = &self.mascotas.get(i)?.bicho;
                let aire = b.en_el_aire();
                let (_, _, sobre) = b.suelo();
                let encima = sobre.and_then(escritorio::hwnd) == Some(ventana);
                // cayó: ¡pisotón!
                let ahora_cayo = b.aterrizajes;
                if ahora_cayo != cayo && encima && hundida.is_none() && !boton_apretado() {
                    let mut r: RECT = unsafe { std::mem::zeroed() };
                    unsafe { GetWindowRect(ventana, &mut r) };
                    mueve_ventana(ventana, r.left, r.top + PISOTON);
                    hundida = Some((r, REGRESA));
                }
                // ya cayó del último brinco (o tardó demasiado): ya
                let acabo = t > 5.0 || (!aire && t > 0.3);
                if acabo && hundida.is_none() {
                    return None;
                }
                Some(Travesura::Pisa {
                    i,
                    ventana,
                    t,
                    cayo: ahora_cayo,
                    hundida,
                })
            }
        }
    }

    /// Se acabó a la fuerza (lo agarraste, apagaste las travesuras): que la
    /// ventana no se quede hundida.
    pub(crate) fn acaba(&mut self, t: Travesura) {
        if let Travesura::Pisa {
            ventana,
            hundida: Some((r, _)),
            ..
        } = t
        {
            mueve_ventana(ventana, r.left, r.top);
        }
    }
}
