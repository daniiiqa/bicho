//! El mouse: agarrarlos, arrastrarlos y aventarlos (o meterlos a una
//! ventana con SUPER), y el clic de cariño. El teclado nomás sirve para saber
//! si traes SUPER apretado.

use super::*;

/// con qué tanto de lo último que lo moviste se calcula el aventón
const VENTANA_AVIENTA: Duration = Duration::from_millis(90);
/// más lento que esto (px/s) al soltarlo es "dejarlo ahí", no aventarlo
const SOLTAR: f32 = 900.0;
/// SUPER en los modificadores de Wayland (Mod4)
const MOD_SUPER: u32 = 0x40;
/// tras soltarlo, cuánto tarda en volver a poder tomar el teclado
const FOCO_VUELVE: f32 = 0.3;

/// Uno que traes agarrado con el mouse.
pub(crate) struct Arrastre {
    pub(crate) i: usize,
    /// de dónde lo agarraste (dentro de su capa)
    pub(crate) agarre: (f32, f32),
    /// dónde lo agarraste en la pantalla, y cuándo (para saber si fue clic)
    pub(crate) inicio: (f32, f32, Instant),
    /// lo último que lo moviste: (cuándo, x, y) en la pantalla
    pub(crate) muestras: VecDeque<(Instant, f32, f32)>,
}

impl App {
    pub(crate) fn cual(&self, surface: &wl_surface::WlSurface) -> Option<usize> {
        self.mascotas
            .iter()
            .position(|p| p.layer.wl_surface() == surface)
    }

    /// Dónde anda el mouse en la pantalla, a partir de lo que dice la capa.
    pub(crate) fn en_pantalla(&self, i: usize, (lx, ly): (f64, f64)) -> (f32, f32) {
        let m = self.mascotas[i].margen;
        (m.1 as f32 + lx as f32, m.0 as f32 + ly as f32)
    }
}

impl App {
    /// Le regresa el teclado a tu ventana: los animalitos dejan de pedirlo
    /// un ratito (y Hyprland enfoca lo de antes).
    pub(crate) fn suelta_teclado(&mut self) {
        self.super_apretado = false;
        for p in &self.mascotas {
            p.layer
                .set_keyboard_interactivity(KeyboardInteractivity::None);
            p.layer.commit();
        }
        self.foco_vuelve = Some(FOCO_VUELVE);
    }

    pub(crate) fn tick_teclado(&mut self, dt: f32) {
        if let Some(t) = self.foco_vuelve.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.foco_vuelve = None;
                for p in &self.mascotas {
                    p.layer
                        .set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
                }
            }
        }
    }
}

/// El teclado de los animalitos (nomás para saber si traes SUPER); su
/// propia marca para no cruzarse con lo de sctk.
pub(crate) struct Teclado;

impl Dispatch<wl_keyboard::WlKeyboard, Teclado> for App {
    fn event(
        app: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &Teclado,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Modifiers { mods_depressed, .. } => {
                app.super_apretado = mods_depressed & MOD_SUPER != 0
            }
            wl_keyboard::Event::Leave { .. } => app.super_apretado = false,
            _ => {}
        }
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for ev in events {
            if self
                .dialogo
                .as_ref()
                .is_some_and(|d| d.layer.wl_surface() == &ev.surface)
            {
                self.en_dialogo(ev);
                continue;
            }
            let Some(i) = self.cual(&ev.surface) else {
                continue;
            };
            // el cursor encima de él (para enseñar su casita, si es de casa)
            match ev.kind {
                PointerEventKind::Enter { .. } => self.mascotas[i].bicho.encima = true,
                PointerEventKind::Leave { .. } => self.mascotas[i].bicho.encima = false,
                _ => {}
            }
            match ev.kind {
                PointerEventKind::Press {
                    button: BOTON_IZQ, ..
                } if self.arrastre.is_none() => {
                    let (gx, gy) = self.en_pantalla(i, ev.position);
                    let ahora = Instant::now();
                    self.mascotas[i].bicho.agarra();
                    self.arrastre = Some(Arrastre {
                        i,
                        agarre: (ev.position.0 as f32, ev.position.1 as f32),
                        inicio: (gx, gy, ahora),
                        muestras: VecDeque::from([(ahora, gx, gy)]),
                    });
                }
                PointerEventKind::Motion { .. } => {
                    self.mascotas[i].bicho.mouse_se_movio();
                    let (gx, gy) = self.en_pantalla(i, ev.position);
                    let Some(a) = self.arrastre.as_mut().filter(|a| a.i == i) else {
                        continue;
                    };
                    let ahora = Instant::now();
                    a.muestras.push_back((ahora, gx, gy));
                    while a.muestras.len() > 2 && ahora - a.muestras[0].0 > VENTANA_AVIENTA {
                        a.muestras.pop_front();
                    }
                    let (x, y) = (gx - a.agarre.0, gy - a.agarre.1);
                    let p = &mut self.mascotas[i];
                    p.bicho.arrastra(x, y);
                    // que se mueva ya, sin esperar al siguiente cuadro
                    let margen = (y.round() as i32, x.round() as i32);
                    if margen != p.margen {
                        p.layer.set_margin(margen.0, 0, 0, margen.1);
                        p.margen = margen;
                        p.layer.commit();
                    }
                }
                PointerEventKind::Release {
                    button: BOTON_IZQ, ..
                } => {
                    let Some(a) = self.arrastre.take() else {
                        continue;
                    };
                    // ¿traías SUPER? (y el teclado de vuelta a tu ventana)
                    let con_super = self.super_apretado;
                    self.suelta_teclado();
                    let (gx, gy) = self.en_pantalla(a.i, ev.position);
                    let invoca = self.mascotas[a.i].especie.invoca;
                    let b = &mut self.mascotas[a.i].bicho;
                    let (x0, y0, t0) = a.inicio;
                    if (gx - x0).hypot(gy - y0) < 6.0 && t0.elapsed() < Duration::from_millis(350) {
                        // fue un clic: cariño; claudio además te pregunta si
                        // ocupas ayuda (otro clic: se lo guarda)
                        b.avienta(0.0, 0.0);
                        let der = b.x + b.cw / 2.0 < gx;
                        // y que se vea cómo se llama
                        self.mascotas[a.i].muestra_nombre = 2.5;
                        let b = &mut self.mascotas[a.i].bicho;
                        if !invoca {
                            b.cariño(der);
                        } else if self.dialogo.as_ref().is_some_and(|d| d.i == a.i) {
                            self.cierra_dialogo();
                        } else {
                            b.platica(der);
                            self.abre_dialogo(a.i);
                        }
                        continue;
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
                    // soltado (no aventado) con SUPER encima de una ventana: se mete
                    if con_super
                        && vx.hypot(vy) < SOLTAR
                        && let Some(m) = Some(self.mascotas[a.i].monitor.clone())
                        && let Some(v) = hypr::ventana_en(&m, gx, gy)
                    {
                        // en una terminal (y con letras: en KDE no hay), que
                        // espere a verlas para acomodarse en una línea
                        if self.mundo.clis.contains(&v) && !kde::activo() {
                            self.mascotas[a.i].bicho.espera_letras();
                        }
                        self.mascotas[a.i].bicho.entra(v);
                    }
                }
                _ => {}
            }
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seat.get_pointer(qh, &seat).ok();
        }
        if capability == Capability::Keyboard && self.teclado.is_none() {
            self.teclado = Some(seat.get_keyboard(qh, Teclado));
        }
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer
            && let Some(p) = self.pointer.take()
        {
            p.release();
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}
