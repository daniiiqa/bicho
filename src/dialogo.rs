//! El globo de diálogo de claudio: su capa, que sigue a quien lo dijo, y lo
//! que pasa al escoger "sí" o "no".

use super::*;

/// El globo de diálogo de claudio ("¿ocupas ayuda?"): su propia capa, que
/// sigue a quien lo dijo.
pub(crate) struct Dialogo {
    /// quién lo dice
    pub(crate) i: usize,
    pub(crate) layer: LayerSurface,
    pub(crate) pool: Option<SlotPool>,
    pub(crate) globo: globo::Globo,
    pub(crate) configurado: bool,
    pub(crate) margen: (i32, i32),
    /// se cierra solo en tantos segundos (ya contestaste)
    pub(crate) cierra_en: Option<f32>,
    /// por qué salió
    pub(crate) motivo: Motivo,
}

/// Por qué salió el globo.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Motivo {
    /// le diste clic a claudio: "¿ocupas ayuda?"
    Ayuda,
    /// Claude te está preguntando algo en otra ventana: "voy / luego"
    Pregunta {
        sesion: String,
        t: u64,
        ventana: Option<String>,
    },
}

impl App {
    pub(crate) fn abre_dialogo(&mut self, i: usize) {
        let globo = if menu::es() {
            globo::Globo::new("¿ocupas ayuda?", &["sí", "no"])
        } else {
            globo::Globo::new("need help?", &["yes", "no"])
        };
        self.dialogo_con(i, globo, None, Motivo::Ayuda);
    }

    pub(crate) fn dialogo_con(
        &mut self,
        i: usize,
        globo: globo::Globo,
        cierra_en: Option<f32>,
        motivo: Motivo,
    ) {
        self.cierra_dialogo();
        // en el monitor donde anda quien habla
        let monitor = self
            .mascotas
            .get(i)
            .map(|p| p.monitor.clone())
            .or(self.monitor.clone());
        let Some(output) = self
            .output
            .outputs()
            .find(|o| self.output.info(o).and_then(|i| i.name) == monitor)
        else {
            return;
        };
        let px = self.cfg.escala;
        let surface = self.compositor.create_surface(&self.qh);
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
        layer.set_size(globo.w as u32 * px, globo.h as u32 * px);
        layer.commit();
        if let Some(p) = self.mascotas.get_mut(i) {
            p.bicho.platica = true;
        }
        self.dialogo = Some(Dialogo {
            i,
            layer,
            pool: None,
            globo,
            configurado: false,
            margen: (i32::MIN, i32::MIN),
            cierra_en,
            motivo,
        });
    }

    pub(crate) fn cierra_dialogo(&mut self) {
        if let Some(d) = self.dialogo.take()
            && let Some(p) = self.mascotas.get_mut(d.i)
        {
            p.bicho.platica = false;
        }
    }

    pub(crate) fn en_dialogo(&mut self, ev: &PointerEvent) {
        let s = self.cfg.escala as f32;
        let Some(d) = self.dialogo.as_mut() else {
            return;
        };
        if d.cierra_en.is_some() {
            return;
        }
        let (x, y) = (ev.position.0 as f32 / s, ev.position.1 as f32 / s);
        match ev.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                if let Some(o) = d.globo.opcion_en(x, y) {
                    d.globo.hover = o;
                }
            }
            PointerEventKind::Press {
                button: BOTON_IZQ, ..
            } => {
                if !d.globo.completo() {
                    d.globo.termina();
                    return;
                }
                let opcion = d.globo.opcion_en(x, y);
                // te preguntaba Claude: "voy" te lleva a esa ventana; "luego"
                // ya no te insiste con esa misma pregunta
                if let Motivo::Pregunta { sesion, t, ventana } = d.motivo.clone() {
                    if opcion.is_some() {
                        // contestada (vas o no): ya no insiste con esa
                        self.claude_ignorada = Some((sesion, t));
                    }
                    match opcion {
                        Some(0) => {
                            if let Some(v) = ventana {
                                std::thread::spawn(move || hypr::enfoca(&v));
                            }
                            d.globo.cambia(if menu::es() { "¡va!" } else { "on it!" });
                            d.cierra_en = Some(0.7);
                        }
                        Some(_) => {
                            d.globo.cambia(":)");
                            d.cierra_en = Some(0.8);
                        }
                        None => {}
                    }
                    return;
                }
                match opcion {
                    Some(0) => {
                        // si anda metido en una ventana, en la carpeta de esa ventana
                        let dentro = self.mascotas.get(d.i).and_then(|p| p.bicho.dentro.clone());
                        std::thread::spawn(move || {
                            menu::abre_claude(
                                dentro
                                    .and_then(|v| hypr::pid_de(&v))
                                    .and_then(hypr::carpeta),
                            )
                        });
                        d.globo.cambia(if menu::es() { "¡va!" } else { "on it!" });
                        d.cierra_en = Some(1.2);
                    }
                    Some(_) => {
                        d.globo.cambia(":)");
                        d.cierra_en = Some(0.8);
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }

    /// Cada cuadro: el globo avanza, sigue a claudio y se pinta.
    pub(crate) fn tick_dialogo(&mut self, dt: f32) {
        // lo del monitor de quien habla
        let Some(quien) = self.dialogo.as_ref().map(|d| d.i) else {
            return;
        };
        let pantalla = self.tam_de(quien);
        let barra = self.mundo_de(quien).arriba;
        let escala_buffer = self.mascotas.get(quien).map_or(1, |p| p.escala_buffer);
        let Some(d) = self.dialogo.as_mut() else {
            return;
        };
        d.globo.avanza(dt);
        if let Some(t) = d.cierra_en.as_mut() {
            *t -= dt;
        }
        // se cansa de esperar
        let vence = d.cierra_en.is_some_and(|t| t <= 0.0) || d.globo.edad > 25.0;
        if vence || self.mascotas.get(d.i).is_none_or(|p| p.bicho.oculto) {
            self.cierra_dialogo();
            return;
        }
        if !d.configurado {
            return;
        }
        let s = self.cfg.escala as f32;
        let p = &self.mascotas[d.i];
        let b = &p.bicho;
        let (gw, gh) = (d.globo.w as f32 * s, d.globo.h as f32 * s);
        // encima de su cabeza; si ahí no cabe (anda muy arriba), abajo de
        // sus patas con la colita para arriba. Sin salirse de la pantalla
        let tope = arte::caja(&arte::grid(
            p.especie,
            &arte::Look {
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
            },
        ))
        .map_or(0.0, |c| c.1 as f32 * s);
        let centro = b.x + b.cw / 2.0;
        let x = (centro - gw / 2.0).clamp(4.0, (pantalla.0 - gw - 4.0).max(4.0));
        // que no quede debajo de la barra de arriba; ya volteado, regresa
        // arriba solo si sobra espacio (si no, al brincar parpadearía)
        let arriba = b.y + tope - gh - s;
        let limite = barra + 4.0;
        let cabe = if d.globo.cola_arriba {
            arriba >= limite + 24.0
        } else {
            arriba >= limite
        };
        d.globo.cola_arriba = !cabe;
        let y = if d.globo.cola_arriba {
            (b.y + b.ch + s).min(pantalla.1 - gh - 4.0).max(4.0)
        } else {
            arriba
        };
        let cola = ((centro - x) / s).max(0.0) as usize;
        let celdas = d.globo.grid(cola);

        let mut colores = [0u32; globo::COLORES];
        let c = &p.colores;
        colores[globo::FONDO as usize] = c[arte::PUPILA as usize];
        colores[globo::BORDE as usize] = c[arte::CONTORNO as usize];
        colores[globo::MARCO as usize] = c[arte::OJO as usize];
        colores[globo::LETRA as usize] = c[arte::OJO as usize];
        colores[globo::ACENTO as usize] = c[arte::CUERPO as usize];

        let px = (self.cfg.escala as i32 * escala_buffer) as usize;
        let (w, h) = ((d.globo.w * px) as i32, (d.globo.h * px) as i32);
        let mut pool = match d.pool.take() {
            Some(pool) => pool,
            None => match SlotPool::new((w * h * 4) as usize, &self.shm) {
                Ok(pool) => pool,
                Err(_) => return,
            },
        };
        if let Ok((buffer, canvas)) = pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) {
            globo::pinta(&celdas, d.globo.w, &colores, px, canvas);
            let margen = (y.round() as i32, x.round() as i32);
            if margen != d.margen {
                d.layer.set_margin(margen.0, 0, 0, margen.1);
                d.margen = margen;
            }
            let surface = d.layer.wl_surface();
            surface.set_buffer_scale(escala_buffer);
            surface.damage_buffer(0, 0, w, h);
            if buffer.attach_to(surface).is_ok() {
                d.layer.commit();
            }
        }
        d.pool = Some(pool);
    }
}
