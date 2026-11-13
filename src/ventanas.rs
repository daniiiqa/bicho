//! Seguir a las ventanas: el mundo de Hyprland, suavizado cuando lo anima,
//! a 60 por segundo cuando algo se mueve; y las letras de las terminales que
//! se le piden al hilo de `texto`.

use super::*;

/// un brinco de ventana más grande que esto (px) es una animación de
/// Hyprland (avisa a dónde va a quedar): se desliza; lo demás se sigue directo
const SALTO_ANIMADO: f32 = 120.0;
/// tras moverse una ventana, cuánto sigue a 60 por segundo
const SE_MOVIO: f32 = 0.6;
/// cuánto alrededor de un animalito se ven las letras (lo que le alcanza
/// un brinco, y tantito más)
const ZONA_X: f32 = 320.0;
const ZONA_Y: f32 = 240.0;
/// qué tan rápido alcanza una ventana su lugar nuevo (s; constante de tiempo)
const SUAVE: f32 = 0.045;

impl App {
    /// Las terminales con animalitos adentro: se le piden al hilo, y lo que
    /// ya vio se les pasa como repisas.
    pub(crate) fn letras(&mut self) {
        let m = &self.mundo;
        let (ox, oy) = m.origen;
        let mut tapar: Vec<texto::Rect> = self
            .mascotas
            .iter()
            .map(|p| {
                (
                    p.bicho.x + ox,
                    p.bicho.y - 4.0 + oy,
                    p.bicho.cw,
                    p.bicho.ch + 4.0,
                )
            })
            .collect();
        if let Some(d) = &self.dialogo {
            let s = self.cfg.escala as f32;
            tapar.push((
                d.margen.1 as f32 + ox,
                d.margen.0 as f32 + oy,
                d.globo.w as f32 * s,
                d.globo.h as f32 * s,
            ));
        }
        let mut pedidos: Vec<texto::Pedido> = Vec::new();
        // nomás los despiertos piden fotos (dormido o sentado no brinca)
        for p in self.mascotas.iter().filter(|p| p.bicho.despierto()) {
            let Some(id) = p.bicho.dentro.as_ref().filter(|id| m.clis.contains(id)) else {
                continue;
            };
            let Some(v) = m.ventanas.iter().find(|v| &v.id == id) else {
                continue;
            };
            // lo que le alcanza un brinco, sin salirse de la ventana
            let (b, (vx, vy)) = (&p.bicho, (v.x + ox, v.y + oy));
            let (zx0, zy0) = ((b.x + ox - ZONA_X).max(vx), (b.y + oy - ZONA_Y).max(vy));
            let (zx1, zy1) = (
                (b.x + ox + b.cw + ZONA_X).min(vx + v.w),
                (b.y + oy + b.ch + ZONA_Y).min(vy + v.h),
            );
            if zx1 <= zx0 || zy1 <= zy0 {
                continue;
            }
            match pedidos.iter_mut().find(|q| &q.id == id) {
                // dos en la misma terminal: que quepa el más grande, y la
                // zona de los dos
                Some(q) => {
                    q.cuerpo = (
                        q.cuerpo.0.min(p.cuerpo.0),
                        q.cuerpo.1.max(p.cuerpo.1),
                        q.cuerpo.2.max(p.cuerpo.2),
                    );
                    let (ax, ay, aw, ah) = q.zona;
                    let (x0, y0) = (ax.min(zx0), ay.min(zy0));
                    q.zona = (x0, y0, (ax + aw).max(zx1) - x0, (ay + ah).max(zy1) - y0);
                }
                None => pedidos.push(texto::Pedido {
                    id: id.clone(),
                    rect: (vx, vy, v.w, v.h),
                    zona: (zx0, zy0, zx1 - zx0, zy1 - zy0),
                    tapar: tapar.clone(),
                    lienzo: (p.bicho.cw, p.bicho.ch),
                    cuerpo: p.cuerpo,
                }),
            }
        }
        let Ok(mut e) = self.texto.lock() else { return };
        for p in &mut self.mascotas {
            match &p.bicho.dentro {
                Some(id) if m.clis.contains(id) => {
                    // cambió de terminal: las repisas de la otra no sirven
                    if p.repisas_de.as_ref() != Some(id) {
                        p.bicho.repisas.clear();
                        p.repisas_de = Some(id.clone());
                    }
                    // fotos nuevas, si ya llegaron; si no (o anda dormido),
                    // se queda con las que tenía
                    if pedidos.iter().any(|q| &q.id == id)
                        && let Some(r) = e.repisas.get(id)
                    {
                        p.bicho.repisas = r.clone();
                    }
                }
                _ => {
                    p.bicho.repisas.clear();
                    p.repisas_de = None;
                }
            }
        }
        e.pedidos = pedidos;
    }
}

impl App {
    /// El mundo nuevo de Hyprland: las ventanas que ya conocía se quedan
    /// donde las veía (`suaviza` las acerca); las nuevas, donde estén.
    pub(crate) fn pon_mundo(&mut self, nuevo: hypr::Mundo) {
        // ¿se movió alguna? entonces un ratito a 60 por segundo
        let movida = nuevo.ventanas.iter().any(|v| {
            self.reales
                .iter()
                .any(|r| r.id == v.id && (r.x, r.y, r.w, r.h) != (v.x, v.y, v.w, v.h))
        });
        if movida {
            self.se_movio = SE_MOVIO;
        }
        self.reales = nuevo.ventanas.clone();
        let viejas = std::mem::take(&mut self.mundo.ventanas);
        let ventanas = nuevo
            .ventanas
            .into_iter()
            .map(|v| viejas.iter().find(|o| o.id == v.id).cloned().unwrap_or(v))
            .collect();
        self.mundo = hypr::Mundo { ventanas, ..nuevo };
        // sin Hyprland, lo de los paneles lo midió la capa de prueba
        if self.medidor.is_some() {
            self.mundo.abajo = self.libre_abajo;
        }
    }

    /// Las ventanas que brincaron lejos se deslizan a su lugar (Hyprland
    /// dice de una dónde van a quedar, aunque las anime); las que mueves con
    /// el mouse se siguen tal cual, sin retraso.
    pub(crate) fn suaviza(&mut self, dt: f32) {
        let k = 1.0 - (-dt / SUAVE).exp();
        for v in &mut self.mundo.ventanas {
            let Some(r) = self.reales.iter().find(|r| r.id == v.id) else {
                continue;
            };
            // ya que empezó a deslizarse, sigue hasta llegar (si no, el
            // último pedazo sería otro brinco)
            let dist = (r.x - v.x)
                .abs()
                .max((r.y - v.y).abs())
                .max((r.w - v.w).abs())
                .max((r.h - v.h).abs());
            if dist > SALTO_ANIMADO {
                self.deslizando.insert(v.id.clone());
            }
            if dist < 0.5 || !self.deslizando.contains(&v.id) {
                self.deslizando.remove(&v.id);
                (v.x, v.y, v.w, v.h) = (r.x, r.y, r.w, r.h);
                continue;
            }
            for (a, b) in [
                (&mut v.x, r.x),
                (&mut v.y, r.y),
                (&mut v.w, r.w),
                (&mut v.h, r.h),
            ] {
                *a += (b - *a) * k;
            }
        }
    }

    /// ¿Hay algo que se tenga que ver fluido? (algo moviéndose: uno en el
    /// aire o agarrado, una ventana que se mueve, el globo)
    pub(crate) fn rapido(&self) -> bool {
        self.arrastre.is_some()
            || self.dialogo.is_some()
            || self.se_movio > 0.0
            || !self.deslizando.is_empty()
            || self.mascotas.iter().any(|p| p.bicho.en_el_aire())
    }

    /// Cada cuánto dibujar: al ritmo de la pantalla si algo se mueve, a 30
    /// si alguien anda haciendo algo, y a 10 si todos están quietos.
    pub(crate) fn cuadro(&self) -> Duration {
        if self.rapido() {
            return CUADRO_RAPIDO;
        }
        match self.mascotas.iter().map(|p| p.bicho.ritmo()).max() {
            Some(1..) => CUADRO,
            _ => CUADRO_QUIETO,
        }
    }
}
