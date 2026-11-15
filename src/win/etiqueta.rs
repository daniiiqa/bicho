//! La etiqueta con el nombre de un animalito (si le pusiste nombre): sale
//! encima de él con el cursor encima, unos segundos al soltarlos y al darle
//! cariño. Una ventanita en capas que deja pasar el mouse; nomás una a la
//! vez. Y ponerles nombre (desde el menú de la bandeja).

use super::{App, CLASE_BICHO, Lienzo, instancia, w};
use crate::{arte, globo};
use windows_sys::Win32::Foundation::{HWND, POINT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(crate) struct Etiqueta {
    /// de quién es
    i: usize,
    nombre: String,
    hwnd: HWND,
    lienzo: Lienzo,
    /// los colores con que se pintó
    colores: [u32; arte::COLORES],
    pos: (i32, i32),
}

impl Drop for Etiqueta {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) };
    }
}

/// Los colores de la etiqueta, de los del animalito (como el globo).
fn colores_etiqueta(c: &[u32; arte::COLORES]) -> [u32; globo::COLORES] {
    let mut colores = [0u32; globo::COLORES];
    colores[globo::FONDO as usize] = c[arte::PUPILA as usize];
    colores[globo::BORDE as usize] = c[arte::CUERPO as usize];
    colores[globo::MARCO as usize] = c[arte::OJO as usize];
    colores[globo::LETRA as usize] = c[arte::OJO as usize];
    colores[globo::ACENTO as usize] = c[arte::CUERPO as usize];
    colores
}

impl App {
    /// Le pone (o le quita, con "") nombre al `i`, y lo guarda en la config.
    pub(crate) fn nombra(&mut self, i: usize, nombre: &str) {
        let Some(entrada) = self.cfg.pets.get(i) else {
            return;
        };
        let nueva = crate::con_nombre(entrada, nombre);
        self.cfg.pets[i] = nueva.clone();
        if let Some(p) = self.mascotas.get_mut(i) {
            p.nombre = crate::parte(&nueva).1.map(String::from);
            p.muestra_nombre = 3.0;
        }
        super::bandeja::guarda_config(&self.cfg);
    }

    pub(crate) fn tick_etiqueta(&mut self, dt: f32) {
        for p in &mut self.mascotas {
            p.muestra_nombre = (p.muestra_nombre - dt).max(0.0);
        }
        let quien = self.mascotas.iter().position(|p| {
            p.nombre.is_some()
                && p.visible
                && !p.bicho.oculto
                && (p.bicho.encima || p.muestra_nombre > 0.0)
        });
        let Some(i) = quien else {
            self.etiqueta = None;
            return;
        };
        let p = &self.mascotas[i];
        let Some(ps) = self.pantallas.get(p.mon) else {
            return;
        };
        let nombre = p.nombre.clone().unwrap_or_default();
        // la mitad de un cuadrito del dibujo, en pixeles de verdad
        let px = ((self.cfg.escala.div_ceil(2).max(1)) as f32 * ps.escala)
            .round()
            .max(1.0) as usize;
        let (celdas, gw, gh) = globo::etiqueta(&nombre);
        let otra = self
            .etiqueta
            .as_ref()
            .is_none_or(|e| e.i != i || e.nombre != nombre || e.lienzo.px != px);
        if otra {
            self.etiqueta = None;
            let Some(lienzo) = Lienzo::de(gw * px, gh * px, px) else {
                return;
            };
            let clase = w(CLASE_BICHO);
            let titulo = w("bicho");
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_LAYERED
                        | WS_EX_TRANSPARENT
                        | WS_EX_TOPMOST
                        | WS_EX_TOOLWINDOW
                        | WS_EX_NOACTIVATE,
                    clase.as_ptr(),
                    titulo.as_ptr(),
                    WS_POPUP,
                    0,
                    0,
                    lienzo.w as i32,
                    lienzo.h as i32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instancia(),
                    std::ptr::null(),
                )
            };
            if hwnd.is_null() {
                return;
            }
            self.etiqueta = Some(Etiqueta {
                i,
                nombre,
                hwnd,
                lienzo,
                colores: [0; arte::COLORES],
                pos: (i32::MIN, i32::MIN),
            });
        }
        // centrada encima de su cabeza; si no cabe arriba, abajo de él
        let b = &p.bicho;
        let (x0, y0, ancho, _) = p.cuerpo.unwrap_or((0.0, 0.0, b.cw, b.ch));
        let s = ps.escala;
        let cabeza = ps.rect.1 + ((b.y - b.dy + y0) * s).round() as i32;
        let centro = ps.rect.0 + ((b.x + x0 + ancho / 2.0) * s).round() as i32;
        let Some(e) = self.etiqueta.as_mut() else {
            return;
        };
        let (tw, th) = (e.lienzo.w as i32, e.lienzo.h as i32);
        let x = (centro - tw / 2).clamp(ps.rect.0, (ps.rect.2 - tw).max(ps.rect.0));
        let y = if cabeza - th - 4 >= ps.rect.1 {
            cabeza - th - 4
        } else {
            ps.rect.1 + ((b.y + b.ch) * s).round() as i32 + 4
        };
        let colores = p.colores;
        let destino = POINT { x, y };
        unsafe {
            if colores != e.colores {
                e.colores = colores;
                let canvas =
                    std::slice::from_raw_parts_mut(e.lienzo.bits, e.lienzo.w * e.lienzo.h * 4);
                globo::pinta(
                    &celdas,
                    gw,
                    &colores_etiqueta(&colores),
                    e.lienzo.px,
                    canvas,
                );
                let tam = SIZE { cx: tw, cy: th };
                let origen = POINT { x: 0, y: 0 };
                let mezcla = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                UpdateLayeredWindow(
                    e.hwnd,
                    std::ptr::null_mut(),
                    &destino,
                    &tam,
                    e.lienzo.dc,
                    &origen,
                    0,
                    &mezcla,
                    ULW_ALPHA,
                );
                ShowWindow(e.hwnd, SW_SHOWNOACTIVATE);
                e.pos = (x, y);
            } else if (x, y) != e.pos {
                e.pos = (x, y);
                SetWindowPos(
                    e.hwnd,
                    HWND_TOPMOST,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}
