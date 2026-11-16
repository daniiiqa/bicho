//! La etiqueta con el nombre de un animalito (si le pusiste nombre): sale
//! encima de él con el cursor encima, unos segundos al soltarlos y al darle
//! cariño. Su propia capa, que no agarra clics; nomás una a la vez.

use super::*;

pub(crate) struct Etiqueta {
    /// de quién es
    pub(crate) i: usize,
    pub(crate) nombre: String,
    pub(crate) layer: LayerSurface,
    pub(crate) pool: Option<SlotPool>,
    pub(crate) configurado: bool,
    pub(crate) margen: (i32, i32),
    /// cómo se pintó la última vez (colores, escala del buffer)
    pub(crate) pintado: Option<([u32; arte::COLORES], i32)>,
}

/// Lo que mide un pixel de la etiqueta: la mitad de un cuadrito del dibujo
/// (para que no quede más grande que ellos).
pub(crate) fn px_etiqueta(escala: u32) -> u32 {
    escala.div_ceil(2).max(1)
}

/// Los colores de la etiqueta, de los del animalito (como el globo).
pub(crate) fn colores_etiqueta(c: &[u32; arte::COLORES]) -> [u32; globo::COLORES] {
    let mut colores = [0u32; globo::COLORES];
    colores[globo::FONDO as usize] = c[arte::PUPILA as usize];
    colores[globo::BORDE as usize] = c[arte::CUERPO as usize];
    colores[globo::MARCO as usize] = c[arte::OJO as usize];
    colores[globo::LETRA as usize] = c[arte::OJO as usize];
    colores[globo::ACENTO as usize] = c[arte::CUERPO as usize];
    colores
}

impl App {
    /// Quién enseña su nombre ahorita (el de más arriba en la lista que
    /// tenga el cursor encima o le quede tiempo).
    fn quien_se_nombra(&self) -> Option<usize> {
        self.mascotas.iter().position(|p| {
            p.nombre.is_some() && !p.bicho.oculto && (p.bicho.encima || p.muestra_nombre > 0.0)
        })
    }

    pub(crate) fn tick_etiqueta(&mut self, dt: f32) {
        for p in &mut self.mascotas {
            p.muestra_nombre = (p.muestra_nombre - dt).max(0.0);
        }
        let quien = self.quien_se_nombra();
        let nombre = quien.and_then(|i| self.mascotas[i].nombre.clone());
        let igual = self
            .etiqueta
            .as_ref()
            .is_some_and(|e| Some(e.i) == quien && Some(&e.nombre) == nombre.as_ref());
        if !igual {
            // otra (u otro nombre): capa nueva, de su tamaño
            self.etiqueta = None;
            if let (Some(i), Some(nombre)) = (quien, nombre) {
                self.abre_etiqueta(i, nombre);
            }
        }
        self.pinta_etiqueta();
    }

    fn abre_etiqueta(&mut self, i: usize, nombre: String) {
        let Some(output) = self.output.outputs().find(|o| {
            self.output.info(o).and_then(|i| i.name)
                == self.mascotas.get(i).map(|p| p.monitor.clone())
        }) else {
            return;
        };
        let (_, w, h) = globo::etiqueta(&nombre);
        let px = px_etiqueta(self.cfg.escala);
        let surface = self.compositor.create_surface(&self.qh);
        // no agarra clics: el mouse pasa
        if let Ok(region) = Region::new(&self.compositor) {
            surface.set_input_region(Some(region.wl_region()));
        }
        let layer = self.layer_shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Top,
            Some("bicho"),
            Some(&output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::LEFT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(w as u32 * px, h as u32 * px);
        layer.commit();
        self.etiqueta = Some(Etiqueta {
            i,
            nombre,
            layer,
            pool: None,
            configurado: false,
            margen: (i32::MIN, i32::MIN),
            pintado: None,
        });
    }

    fn pinta_etiqueta(&mut self) {
        // lo del monitor de su animalito
        let Some(quien) = self.etiqueta.as_ref().map(|e| e.i) else {
            return;
        };
        let pantalla = self.tam_de(quien);
        let escala_buffer = self.mascotas.get(quien).map_or(1, |p| p.escala_buffer);
        let Some(e) = self.etiqueta.as_mut() else {
            return;
        };
        if !e.configurado {
            return;
        }
        let Some(p) = self.mascotas.get(e.i) else {
            return;
        };
        let (celdas, gw, gh) = globo::etiqueta(&e.nombre);
        let pl = px_etiqueta(self.cfg.escala) as f32;
        let (tw, th) = (gw as f32 * pl, gh as f32 * pl);
        let b = &p.bicho;
        // centrada encima de su cabeza; si no cabe arriba, abajo de él
        let cabeza = b.y - b.dy + b.ch - p.cuerpo.2;
        let centro = b.x + (p.cuerpo.0 + p.cuerpo.1) / 2.0;
        let x = (centro - tw / 2.0).clamp(0.0, (pantalla.0 - tw).max(0.0));
        let y = if cabeza - th - 3.0 >= 0.0 {
            cabeza - th - 3.0
        } else {
            b.y + b.ch + 3.0
        };
        let margen = (y.round() as i32, x.round() as i32);
        let movida = margen != e.margen;
        if movida {
            e.layer.set_margin(margen.0, 0, 0, margen.1);
            e.margen = margen;
        }
        let ahora = (p.colores, escala_buffer);
        if e.pintado == Some(ahora) && e.pool.is_some() {
            if movida {
                e.layer.commit();
            }
            return;
        }
        let px = (px_etiqueta(self.cfg.escala) as i32 * escala_buffer) as usize;
        let (w, h) = ((gw * px) as i32, (gh * px) as i32);
        let mut pool = match e.pool.take() {
            Some(pool) => pool,
            None => match SlotPool::new((w * h * 4) as usize, &self.shm) {
                Ok(pool) => pool,
                Err(_) => return,
            },
        };
        if let Ok((buffer, canvas)) = pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) {
            globo::pinta(&celdas, gw, &colores_etiqueta(&p.colores), px, canvas);
            let surface = e.layer.wl_surface();
            surface.set_buffer_scale(escala_buffer);
            surface.damage_buffer(0, 0, w, h);
            if buffer.attach_to(surface).is_ok() {
                e.layer.commit();
                e.pintado = Some(ahora);
            }
        }
        e.pool = Some(pool);
    }
}
