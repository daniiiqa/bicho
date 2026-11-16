//! Varios monitores en Linux: cada animalito vive en uno (su capa está en
//! esa salida) y ve su mundo en coordenadas de ese monitor. Por las orillas
//! que tocan a otro monitor se cruzan (caminando, persiguiendo al cursor, o
//! aventados): se le cierra la capa y le sale otra en el de al lado, a la
//! misma altura. Con Hyprland; en KDE se quedan en el suyo.

use super::*;

/// Un monitor: su salida, dónde empieza y cuánto mide (lógicos, globales).
pub(crate) struct Pantalla {
    pub(crate) nombre: String,
    /// (None nomás en las pruebas)
    pub(crate) output: Option<wl_output::WlOutput>,
    pub(crate) origen: (f32, f32),
    pub(crate) tam: (f32, f32),
    pub(crate) escala: i32,
}

/// La pantalla pegada a la orilla derecha (o izquierda) de `p`, a la altura
/// `y` (global); si a esa altura no hay, cualquiera pegada.
pub(crate) fn vecina<'a>(
    ps: &'a [Pantalla],
    p: &Pantalla,
    der: bool,
    y: f32,
) -> Option<&'a Pantalla> {
    let toca = |q: &&Pantalla| {
        let junto = if der {
            (q.origen.0 - (p.origen.0 + p.tam.0)).abs() <= 2.0
        } else {
            (q.origen.0 + q.tam.0 - p.origen.0).abs() <= 2.0
        };
        q.nombre != p.nombre
            && junto
            && q.origen.1 < p.origen.1 + p.tam.1
            && q.origen.1 + q.tam.1 > p.origen.1
    };
    let pegadas: Vec<&Pantalla> = ps.iter().filter(toca).collect();
    pegadas
        .iter()
        .copied()
        .find(|q| y >= q.origen.1 && y < q.origen.1 + q.tam.1)
        .or(pegadas.first().copied())
}

impl App {
    /// Los monitores de ahorita (de lo que dice Wayland de cada salida).
    pub(crate) fn lee_pantallas(&mut self) {
        self.pantallas = self
            .output
            .outputs()
            .filter_map(|o| {
                let info = self.output.info(&o)?;
                let (w, h) = info.logical_size?;
                let (x, y) = info.logical_position.unwrap_or((0, 0));
                Some(Pantalla {
                    nombre: info.name.clone()?,
                    output: Some(o),
                    origen: (x as f32, y as f32),
                    tam: (w as f32, h as f32),
                    escala: info.scale_factor.max(1),
                })
            })
            .collect();
    }

    pub(crate) fn pantalla(&self, nombre: &str) -> Option<&Pantalla> {
        self.pantallas.iter().find(|p| p.nombre == nombre)
    }

    /// El mundo del monitor de `i` (el de casa si no se sabe).
    pub(crate) fn mundo_de(&self, i: usize) -> &hypr::Mundo {
        self.mascotas
            .get(i)
            .and_then(|p| self.mundos.get(&p.monitor))
            .unwrap_or(&self.mundo)
    }

    /// Lo que mide el monitor de `i`.
    pub(crate) fn tam_de(&self, i: usize) -> (f32, f32) {
        self.mascotas
            .get(i)
            .and_then(|p| self.pantalla(&p.monitor))
            .map_or(self.pantalla, |p| p.tam)
    }

    /// Dónde empieza el monitor de `i` (global).
    pub(crate) fn origen_de(&self, i: usize) -> (f32, f32) {
        self.mascotas
            .get(i)
            .and_then(|p| self.pantalla(&p.monitor))
            .map_or(self.mundo.origen, |p| p.origen)
    }

    /// El cursor en coordenadas del monitor de `i`.
    pub(crate) fn cursor_de(&self, i: usize) -> Option<(f32, f32)> {
        let (ox, oy) = self.origen_de(i);
        self.cursor.map(|(x, y)| (x - ox, y - oy))
    }

    /// El número del monitor (para que dos en monitores distintos no se
    /// crean en el mismo suelo).
    pub(crate) fn numero_de(&self, monitor: &str) -> u8 {
        self.pantallas
            .iter()
            .position(|p| p.nombre == monitor)
            .unwrap_or(0) as u8
    }

    /// ¿Hay monitor pasando la orilla izquierda / derecha de `i`, a la
    /// altura de sus patas? (nomás con Hyprland)
    pub(crate) fn puertas_de(&self, i: usize) -> (bool, bool) {
        if kde::activo() || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
            return (false, false);
        }
        let Some(p) = self.mascotas.get(i).and_then(|m| self.pantalla(&m.monitor)) else {
            return (false, false);
        };
        let b = &self.mascotas[i].bicho;
        let pies = p.origen.1 + b.y + b.ch;
        (
            vecina(&self.pantallas, p, false, pies).is_some(),
            vecina(&self.pantallas, p, true, pies).is_some(),
        )
    }

    /// Una capa para un animalito en esa salida (vacía: hasta que se pinte
    /// no agarra clics).
    pub(crate) fn capa_en(&self, output: &wl_output::WlOutput) -> LayerSurface {
        let px = self.cfg.escala;
        let surface = self.compositor.create_surface(&self.qh);
        if let Ok(region) = Region::new(&self.compositor) {
            surface.set_input_region(Some(region.wl_region()));
        }
        let layer = self.layer_shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Top,
            Some("bicho"),
            Some(output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::LEFT);
        layer.set_exclusive_zone(-1);
        // al darle clic toma el teclado: así se sabe si traes SUPER apretado
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_size(arte::ANCHO as u32 * px, arte::ALTO as u32 * px);
        layer.commit();
        layer
    }

    /// Lo pasa al monitor `nombre`, en (x, y) de allá: capa nueva en esa
    /// salida (la vieja se cierra al soltarla).
    pub(crate) fn muda(&mut self, i: usize, nombre: &str) {
        let Some(q) = self.pantalla(nombre) else {
            return;
        };
        let (Some(output), escala) = (q.output.clone(), q.escala) else {
            return;
        };
        let layer = self.capa_en(&output);
        let numero = self.numero_de(nombre);
        if self.etiqueta.as_ref().is_some_and(|e| e.i == i) {
            self.etiqueta = None;
        }
        if self.dialogo.as_ref().is_some_and(|d| d.i == i) {
            self.cierra_dialogo();
        }
        let p = &mut self.mascotas[i];
        p.layer = layer;
        p.pool = None;
        p.pintado = None;
        p.region = None;
        p.margen = (i32::MIN, i32::MIN);
        p.configurado = false;
        p.escala_buffer = escala;
        p.monitor = nombre.to_string();
        p.repisas_de = None;
        p.bicho.pantalla = numero;
    }

    /// Se salió por la orilla de su monitor: al de al lado, a la misma
    /// altura (si no hay, se regresa).
    pub(crate) fn cruza(&mut self, i: usize, der: bool) {
        let Some(p) = self.pantalla(&self.mascotas[i].monitor) else {
            self.mascotas[i].bicho.da_vuelta();
            return;
        };
        let b = &self.mascotas[i].bicho;
        let pies = p.origen.1 + b.y + b.ch;
        let Some(q) = vecina(&self.pantallas, p, der, pies) else {
            self.mascotas[i].bicho.da_vuelta();
            return;
        };
        let (cw, ch) = (b.cw, b.ch);
        let nombre = q.nombre.clone();
        let abajo = self.mundos.get(&nombre).map_or(0.0, |m| m.abajo);
        let y = (pies - q.origen.1 - ch).min(q.tam.1 - abajo - ch);
        let x = if der { 0.0 } else { (q.tam.0 - cw).max(0.0) };
        self.muda(i, &nombre);
        self.mascotas[i].bicho.cruza(x, y, der);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pantalla(nombre: &str, origen: (f32, f32), tam: (f32, f32)) -> Pantalla {
        Pantalla {
            nombre: nombre.into(),
            output: None,
            origen,
            tam,
            escala: 1,
        }
    }

    #[test]
    fn el_de_al_lado() {
        // la laptop, y a la derecha uno más grande y más arriba
        let ps = vec![
            pantalla("eDP-1", (0.0, 0.0), (1920.0, 1200.0)),
            pantalla("HDMI-A-1", (1920.0, -300.0), (2560.0, 1440.0)),
        ];
        let nombre = |p: Option<&Pantalla>| p.map(|p| p.nombre.clone());
        assert_eq!(
            nombre(vecina(&ps, &ps[0], true, 600.0)),
            Some("HDMI-A-1".into())
        );
        assert_eq!(nombre(vecina(&ps, &ps[0], false, 600.0)), None);
        assert_eq!(
            nombre(vecina(&ps, &ps[1], false, 100.0)),
            Some("eDP-1".into())
        );
        // a una altura donde no hay nada: cualquiera pegada
        assert_eq!(
            nombre(vecina(&ps, &ps[1], false, -200.0)),
            Some("eDP-1".into())
        );
    }
}
