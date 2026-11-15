//! El contraste en Linux: si un animalito se confunde con lo que tiene
//! atrás (uno oscuro en una ventana oscura), le sale un halo claro u oscuro,
//! igual que en Windows. Lo de atrás se ve con grim (Hyprland; en KDE no
//! hay cómo), en su hilo y cada ratito: de la foto de su lienzo nomás
//! cuentan los pixeles donde no está él (ni su halo).

use crate::arte;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Cada cuánto vuelve a ver.
const CADA: Duration = Duration::from_millis(900);
/// qué tan parecidos (de luz, 0 a 1) a lo de atrás para ponerles halo, y qué
/// tan distintos para quitárselo
pub(crate) const HALO_SI: f32 = 0.2;
pub(crate) const HALO_NO: f32 = 0.32;

/// Lo que el hilo tiene que ver: el lienzo de uno (global, lógico) y su
/// dibujo (para no contarse a sí mismo).
#[derive(Clone)]
pub(crate) struct Pedido {
    pub(crate) i: usize,
    pub(crate) rect: crate::texto::Rect,
    pub(crate) dibujo: arte::Grid,
}

#[derive(Default)]
pub(crate) struct Estado {
    pub(crate) pedidos: Vec<Pedido>,
    /// qué tan clarito es lo de atrás de cada uno
    pub(crate) luz: HashMap<usize, f32>,
}

pub(crate) type Compartido = Arc<Mutex<Estado>>;

/// La luz de lo de atrás en una foto de su lienzo: los cuadritos donde está
/// él (y uno alrededor, su halo) no cuentan.
pub(crate) fn luz_sin_el(img: &crate::texto::Imagen, dibujo: &arte::Grid) -> Option<f32> {
    if img.w == 0 || img.h == 0 {
        return None;
    }
    let ocupado = |cx: usize, cy: usize| {
        (cy.saturating_sub(1)..=(cy + 1).min(arte::ALTO - 1)).any(|y| {
            (cx.saturating_sub(1)..=(cx + 1).min(arte::ANCHO - 1)).any(|x| dibujo[y][x] != 0)
        })
    };
    let (mut suma, mut n) = (0.0, 0u32);
    for y in (0..img.h).step_by(2) {
        for x in (0..img.w).step_by(2) {
            let (cx, cy) = (x * arte::ANCHO / img.w, y * arte::ALTO / img.h);
            if ocupado(cx, cy) {
                continue;
            }
            let i = (y * img.w + x) * 3;
            let rgb = ((img.rgb[i] as u32) << 16)
                | ((img.rgb[i + 1] as u32) << 8)
                | img.rgb[i + 2] as u32;
            suma += arte::luz(rgb);
            n += 1;
        }
    }
    (n > 0).then(|| suma / n as f32)
}

pub(crate) fn arranca() -> Compartido {
    let c: Compartido = Arc::default();
    let hilo = c.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(CADA);
            let pedidos = hilo.lock().map(|e| e.pedidos.clone()).unwrap_or_default();
            let mut luz = HashMap::new();
            for p in &pedidos {
                if let Some(l) =
                    crate::texto::foto(p.rect).and_then(|img| luz_sin_el(&img, &p.dibujo))
                {
                    luz.insert(p.i, l);
                }
            }
            if let Ok(mut e) = hilo.lock() {
                e.luz = luz;
            }
        }
    });
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_se_cuenta_a_si_mismo() {
        // fondo blanco, y él (negro) en medio: lo de atrás es blanco
        let (w, h) = (arte::ANCHO * 2, arte::ALTO * 2);
        let mut dibujo = [[0u8; arte::ANCHO]; arte::ALTO];
        let mut rgb = vec![255u8; w * h * 3];
        for y in 8..12 {
            for x in 6..10 {
                dibujo[y][x] = arte::CUERPO;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = ((y * 2 + dy) * w + x * 2 + dx) * 3;
                    rgb[i..i + 3].copy_from_slice(&[0, 0, 0]);
                }
            }
        }
        let img = crate::texto::Imagen { w, h, rgb };
        let l = luz_sin_el(&img, &dibujo).unwrap();
        assert!(l > 0.99, "{l}");
    }
}
