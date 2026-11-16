//! claudio y Claude Code: con lo que dejan los hooks (`claude.rs`), mientras
//! Claude trabaja a claudio le salen puntitos, cuando acaba brinca con un
//! corazón, y si Claude te pregunta algo y andas en otra ventana, le sale un
//! globo: "¡claude te pregunta algo!" ▶ voy / luego.

use super::*;
use claude::Estado;
use dialogo::Motivo;

/// Cada cuánto se ven las sesiones (s).
const CADA_CLAUDE: f32 = 0.3;

impl App {
    pub(crate) fn tick_claude(&mut self, dt: f32) {
        self.cada_claude -= dt;
        if self.cada_claude > 0.0 {
            return;
        }
        self.cada_claude = CADA_CLAUDE;
        if !self.mascotas.iter().any(|p| p.especie.invoca) {
            return;
        }
        let sesiones = claude::sesiones();
        // las que acaban de terminar (estaban trabajando y ya no)
        let terminaron: Vec<&str> = sesiones
            .iter()
            .filter(|s| {
                s.estado == Estado::Listo
                    && self.claude_antes.get(&s.id) == Some(&Estado::Trabajando)
            })
            .map(|s| s.id.as_str())
            .collect();
        let ventanas: Vec<Option<String>> = sesiones
            .iter()
            .map(|s| hypr::ventana_de(&self.mundo, &s.pids))
            .collect();

        for p in self.mascotas.iter_mut().filter(|p| p.especie.invoca) {
            // las suyas: las de su ventana, si anda metido en una con Claude;
            // si no, todas
            let dentro = p.bicho.dentro.clone();
            let en_su_ventana: Vec<usize> = (0..sesiones.len())
                .filter(|&k| dentro.is_some() && ventanas[k] == dentro)
                .collect();
            let suyas = if en_su_ventana.is_empty() {
                (0..sesiones.len()).collect()
            } else {
                en_su_ventana
            };
            p.bicho.piensa = suyas
                .iter()
                .any(|&k| sesiones[k].estado == Estado::Trabajando);
            if !p.bicho.platica
                && suyas
                    .iter()
                    .any(|&k| terminaron.contains(&sesiones[k].id.as_str()))
            {
                p.bicho.celebra();
            }
        }
        self.claude_antes = sesiones.iter().map(|s| (s.id.clone(), s.estado)).collect();

        // la pregunta más reciente que no dejaste para luego
        let pregunta = sesiones.iter().enumerate().rev().find(|(_, s)| {
            s.estado == Estado::Pregunta
                && self.claude_ignorada.as_ref() != Some(&(s.id.clone(), s.t))
        });
        // ¿el globo de una pregunta que ya contestaste? fuera
        if let Some(d) = &self.dialogo
            && let Motivo::Pregunta { sesion, t, .. } = &d.motivo
            && d.cierra_en.is_none()
            && !sesiones
                .iter()
                .any(|s| &s.id == sesion && s.t == *t && s.estado == Estado::Pregunta)
        {
            self.cierra_dialogo();
        }
        let Some((k, s)) = pregunta else { return };
        if self.dialogo.is_some() {
            return;
        }
        // si ya estás en esa ventana, no hace falta avisar
        let ventana = ventanas[k].clone();
        if ventana.is_some() && ventana == hypr::activa() {
            return;
        }
        // el claudio que avise: el que ande en esa ventana, o cualquiera que se
        // vea; si todos andan escondidos (metidos en una ventana de otro
        // escritorio, como la de Claude), uno se sale y cae donde estás tú
        let visible = self
            .mascotas
            .iter()
            .position(|p| {
                p.especie.invoca
                    && !p.bicho.oculto
                    && ventana.is_some()
                    && p.bicho.dentro == ventana
            })
            .or_else(|| {
                self.mascotas
                    .iter()
                    .position(|p| p.especie.invoca && !p.bicho.oculto)
            });
        let i = match visible {
            Some(i) => i,
            None => {
                let Some(i) = self.mascotas.iter().position(|p| p.especie.invoca) else {
                    return;
                };
                let ancho = self.tam_de(i).0;
                self.mascotas[i].bicho.sal_a_avisar(ancho);
                i
            }
        };
        let globo = if menu::es() {
            globo::Globo::new("¡claude te pregunta algo!", &["voy", "luego"])
        } else {
            globo::Globo::new("claude has a question!", &["go", "later"])
        };
        let motivo = Motivo::Pregunta {
            sesion: s.id.clone(),
            t: s.t,
            ventana,
        };
        self.dialogo_con(i, globo, None, motivo);
    }
}
