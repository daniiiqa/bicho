//! Qué hace el animalito: camina por el piso, trepa por los lados de las
//! ventanas, se pasea por arriba de ellas y se cae por la orilla. Se sienta,
//! se duerme (más de noche), baila si suena música, a veces persigue al
//! cursor, y si se lo acercas brinca con un corazón. La baba anda a brincos.
//!
//! Todo en pixeles lógicos del monitor; (x, y) es la esquina de arriba a la
//! izquierda de su lienzo (`cw × ch`), con las patas en la orilla de abajo.

use crate::arte::{Forma, Look, Pose};
use crate::hypr::{Mundo, Ventana};

/// Por dónde puede caminar: el piso (`ventana: None`) o el techo de una
/// ventana. `x0..x1` son las x válidas del lienzo; `y` donde queda parado.
#[derive(Debug)]
struct Plataforma {
    ventana: Option<String>,
    x0: f32,
    x1: f32,
    y: f32,
}

#[derive(Clone, PartialEq, Debug)]
enum Hace {
    Camina,
    Para,
    Sienta,
    Duerme,
    /// camina hasta el lado de la ventana para treparla
    VaATrepar { ventana: String, pared_der: bool },
    Trepa { ventana: String, pared_der: bool },
    /// en el aire (cayendo o brincando)
    Vuela,
    Baila,
    /// va hacia el cursor
    Persigue,
}

pub struct Bicho {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    der: bool,
    /// la ventana sobre la que anda (None: el piso)
    sobre: Option<String>,
    hace: Hace,
    falta: f32,
    reloj: f32,
    parpadeo: f32,
    prox_parpadeo: f32,
    corazon: f32,
    cariño_espera: f32,
    rng: u64,
    /// tamaño de una celda del dibujo
    s: f32,
    pub cw: f32,
    pub ch: f32,
    pub velocidad: f32,
    pub trepa: bool,
    /// anda a brincos (la baba)
    pub salta: bool,
    /// brincando para caminar: al caer sigue con esto
    vuelve: Option<Hace>,
    /// lo que falta para el siguiente brinco
    carga: f32,
    /// segundos desde que cayó (la baba tiembla al caer)
    aterrizo: f32,
    /// suena música (playerctl)
    pub musica: bool,
    /// de noche le da más sueño
    pub noche: bool,
    cursor: Option<(f32, f32)>,
    /// cuánto sube el dibujo sin moverse de lugar (bailando)
    pub dy: f32,
}

const CORAZON_DURA: f32 = 1.3;
const GRAVEDAD: f32 = 1400.0;

impl Bicho {
    pub fn new(s: f32, cw: f32, ch: f32, semilla: u64) -> Bicho {
        Bicho {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            der: true,
            sobre: None,
            hace: Hace::Vuela,
            falta: 0.0,
            reloj: 0.0,
            parpadeo: 0.0,
            prox_parpadeo: 3.0,
            corazon: 0.0,
            cariño_espera: 0.0,
            rng: semilla | 1,
            s,
            cw,
            ch,
            velocidad: 1.0,
            trepa: true,
            salta: false,
            vuelve: None,
            carga: 0.0,
            aterrizo: 9.0,
            musica: false,
            noche: false,
            cursor: None,
            dy: 0.0,
        }
    }

    /// Aparece en algún lugar del piso, cayendo desde arriba.
    pub fn aparece(&mut self, ancho: f32) {
        self.x = self.azar() * (ancho - self.cw).max(0.0);
        self.y = -self.ch;
        self.der = self.azar() < 0.5;
        self.hace = Hace::Vuela;
    }

    fn azar(&mut self) -> f32 {
        // xorshift: no hace falta más
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + self.azar() * (b - a)
    }

    fn paso(&self) -> f32 {
        14.0 * self.s * self.velocidad
    }

    fn plataformas(&self, m: &Mundo, ancho: f32, alto: f32) -> Vec<Plataforma> {
        let mut p = vec![Plataforma { ventana: None, x0: 0.0, x1: (ancho - self.cw).max(0.0), y: alto - m.abajo - self.ch }];
        for v in &m.ventanas {
            let y = v.y - m.borde - self.ch;
            // con la cabeza fuera de la pantalla no; las antenas sí pueden
            if y + 4.0 * self.s < 0.0 {
                continue;
            }
            let x0 = (v.x - m.borde - self.cw / 2.0).max(0.0);
            let x1 = (v.x + v.w + m.borde - self.cw / 2.0).min(ancho - self.cw);
            // casi fuera de la pantalla no cabe (y clamp(x0, x1) truena si x0 > x1)
            if x1 < x0 {
                continue;
            }
            p.push(Plataforma { ventana: Some(v.id.clone()), x0, x1, y });
        }
        p
    }

    /// La x del lienzo pegado al lado de `v` (`pared_der`: la ventana le queda
    /// a la derecha).
    fn junto_a(&self, v: &Ventana, m: &Mundo, pared_der: bool) -> f32 {
        if pared_der { v.x - m.borde - self.cw } else { v.x + v.w + m.borde }
    }

    /// Escoge qué hacer ahora que terminó lo anterior (estando parado en
    /// algo).
    fn decide(&mut self, m: &Mundo, ancho: f32, alto: f32) {
        let r = self.azar();
        let piso = alto - m.abajo - self.ch;
        if self.trepa && self.sobre.is_none() && r < 0.22 {
            // una ventana que llegue al piso y con lado libre en la pantalla
            let mut opciones = Vec::new();
            for v in &m.ventanas {
                if (v.y + v.h + m.borde - (piso + self.ch)).abs() > 40.0 || v.y - m.borde - self.ch + 4.0 * self.s < 0.0 {
                    continue;
                }
                for pared_der in [true, false] {
                    let x = self.junto_a(v, m, pared_der);
                    if x >= 0.0 && x <= ancho - self.cw {
                        opciones.push((v.id.clone(), pared_der));
                    }
                }
            }
            if !opciones.is_empty() {
                let (ventana, pared_der) = opciones[(self.azar() * opciones.len() as f32) as usize % opciones.len()].clone();
                self.hace = Hace::VaATrepar { ventana, pared_der };
                self.falta = 30.0;
                return;
            }
        }
        if self.musica && self.azar() < 0.6 {
            (self.hace, self.falta) = (Hace::Baila, self.entre(4.0, 10.0));
            return;
        }
        if let Some((cx, _)) = self.cursor
            && cx >= 0.0
            && cx <= ancho
            && self.azar() < 0.1
        {
            self.der = cx > self.x + self.cw / 2.0;
            (self.hace, self.falta) = (Hace::Persigue, self.entre(3.0, 6.0));
            return;
        }
        // de noche: más sentado y dormido
        let r = if self.noche { self.azar() * 0.7 + 0.3 } else { self.azar() };
        (self.hace, self.falta) = if r < 0.5 {
            if self.azar() < 0.35 {
                self.der = !self.der;
            }
            (Hace::Camina, self.entre(2.0, 7.0))
        } else if r < 0.72 {
            (Hace::Para, self.entre(1.0, 3.5))
        } else if r < 0.86 {
            (Hace::Sienta, self.entre(3.0, 8.0))
        } else if r < 0.96 {
            (Hace::Duerme, self.entre(12.0, 30.0))
        } else {
            self.vy = -380.0;
            self.vx = 0.0;
            (Hace::Vuela, 0.0)
        };
    }

    pub fn tick(&mut self, dt: f32, m: &Mundo, ancho: f32, alto: f32, cursor: Option<(f32, f32)>) -> Look {
        let dt = dt.min(0.1);
        self.reloj += dt;
        self.aterrizo += dt;
        self.falta -= dt;
        self.corazon = (self.corazon - dt).max(0.0);
        self.cariño_espera = (self.cariño_espera - dt).max(0.0);
        self.parpadeo = (self.parpadeo - dt).max(0.0);
        self.prox_parpadeo -= dt;
        if self.prox_parpadeo <= 0.0 {
            self.parpadeo = 0.14;
            self.prox_parpadeo = self.entre(2.0, 6.0);
        }
        self.cursor = cursor;
        self.dy = 0.0;
        let plats = self.plataformas(m, ancho, alto);
        let paso = self.paso();

        // ¿lo que pisa sigue ahí? (la ventana se cerró, se movió o cambiaste
        // de escritorio)
        if !matches!(self.hace, Hace::Vuela | Hace::Trepa { .. }) {
            match plats.iter().find(|p| p.ventana == self.sobre) {
                Some(p) if self.x >= p.x0 - 2.0 && self.x <= p.x1 + 2.0 => self.y = p.y,
                _ => self.cae(),
            }
        }

        // cariño: el cursor cerca lo despierta y brinca
        if let Some((cx, cy)) = cursor {
            let (bx, by) = (self.x + self.cw / 2.0, self.y + self.ch * 0.65);
            let cerca = (cx - bx).hypot(cy - by) < self.cw * 1.4;
            if cerca && self.cariño_espera <= 0.0 && !matches!(self.hace, Hace::Vuela | Hace::Trepa { .. }) {
                self.cariño(cx > bx);
                if self.hace == Hace::Persigue {
                    self.falta = 0.0;
                }
            }
        }

        match self.hace.clone() {
            Hace::Camina => {
                self.anda(dt, &plats);
                if self.falta <= 0.0 && self.hace == Hace::Camina {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::Persigue => {
                let meta = cursor.map(|(cx, _)| cx - self.cw / 2.0);
                match meta {
                    Some(meta) if (meta - self.x).abs() > self.cw / 3.0 && self.falta > 0.0 => {
                        self.der = meta > self.x;
                        self.anda(dt, &plats);
                    }
                    _ => {
                        self.hace = Hace::Para;
                        self.falta = self.entre(0.8, 2.0);
                    }
                }
            }
            Hace::Baila => {
                // de un lado al otro, al brinquito
                let t = self.reloj * 2.2;
                self.der = (t as u32).is_multiple_of(2);
                self.dy = (t.fract() * std::f32::consts::PI).sin() * 3.0 * self.s;
                if self.falta <= 0.0 || !self.musica {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::Para | Hace::Sienta => {
                if self.falta <= 0.0 || (self.musica && self.azar() < dt * 0.3) {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::Duerme => {
                if self.falta <= 0.0 {
                    self.decide(m, ancho, alto);
                }
            }
            Hace::VaATrepar { ventana, pared_der } => {
                let Some(v) = m.ventanas.iter().find(|v| v.id == ventana) else {
                    self.hace = Hace::Camina;
                    self.falta = 1.0;
                    return self.look();
                };
                let meta = self.junto_a(v, m, pared_der);
                if (meta - self.x).abs() <= paso * dt || self.falta <= 0.0 {
                    self.x = meta;
                    self.hace = Hace::Trepa { ventana, pared_der };
                } else {
                    self.der = meta > self.x;
                    self.x += if self.der { paso } else { -paso } * dt;
                }
            }
            Hace::Trepa { ventana, pared_der } => {
                let Some(v) = m.ventanas.iter().find(|v| v.id == ventana) else {
                    self.cae();
                    return self.look();
                };
                if (self.junto_a(v, m, pared_der) - self.x).abs() > 4.0 {
                    self.cae();
                    return self.look();
                }
                self.y -= paso * 0.7 * dt;
                // la cabeza ya llegó a la orilla de arriba: se sube
                let techo = v.y - m.borde;
                if self.y + 4.0 * self.s <= techo {
                    if let Some(p) = plats.iter().find(|p| p.ventana.as_deref() == Some(ventana.as_str())) {
                        self.x = if pared_der { p.x0 } else { p.x1 };
                        self.y = p.y;
                        self.sobre = p.ventana.clone();
                        self.der = pared_der;
                        self.hace = Hace::Camina;
                        self.falta = self.entre(2.0, 6.0);
                    } else {
                        self.cae();
                    }
                }
            }
            Hace::Vuela => {
                let antes = self.y;
                self.vy += GRAVEDAD * dt;
                self.y += self.vy * dt;
                self.x = (self.x + self.vx * dt).clamp(0.0, (ancho - self.cw).max(0.0));
                if self.vy > 0.0 {
                    // cae en lo más alto que cruzó
                    let aterriza = plats
                        .iter()
                        .filter(|p| antes <= p.y + 0.5 && self.y >= p.y && self.x >= p.x0 && self.x <= p.x1)
                        .min_by(|a, b| a.y.total_cmp(&b.y));
                    let piso = &plats[0];
                    let aterriza = aterriza.or((self.y >= piso.y).then_some(piso));
                    if let Some(p) = aterriza {
                        self.y = p.y;
                        self.sobre = p.ventana.clone();
                        // qué tan fuerte cayó: más alto, más se embarra
                        self.aterrizo = if self.vy > 500.0 { 0.0 } else { 0.07 };
                        self.vx = 0.0;
                        self.vy = 0.0;
                        match self.vuelve.take() {
                            Some(h) if self.falta > 0.0 => {
                                self.hace = h;
                                // que alcance a temblar antes del siguiente
                                self.carga = 0.55;
                            }
                            _ => {
                                self.hace = Hace::Para;
                                self.falta = self.entre(0.4, 1.2);
                            }
                        }
                    }
                }
            }
        }
        self.look()
    }

    /// Un paso hacia donde mira; en la orilla del techo de una ventana se
    /// avienta (o se da la vuelta), en la del piso se da la vuelta. La baba
    /// en vez de caminar se aplasta y brinca.
    fn anda(&mut self, dt: f32, plats: &[Plataforma]) {
        let paso = self.paso();
        let Some(p) = plats.iter().find(|p| p.ventana == self.sobre) else { return };
        if self.salta {
            self.carga -= dt;
            if self.carga > 0.0 {
                return;
            }
            let lejos = if self.der { self.x + paso * 0.5 > p.x1 } else { self.x - paso * 0.5 < p.x0 };
            if lejos && (self.sobre.is_none() || self.azar() < 0.4) {
                self.der = !self.der;
            }
            self.carga = 0.3;
            self.vx = if self.der { paso * 1.6 } else { -paso * 1.6 };
            self.vy = -300.0;
            self.vuelve = Some(self.hace.clone());
            self.hace = Hace::Vuela;
            return;
        }
        self.x += if self.der { paso } else { -paso } * dt;
        if self.x < p.x0 || self.x > p.x1 {
            self.x = self.x.clamp(p.x0, p.x1);
            if self.sobre.is_some() && self.azar() < 0.6 {
                // se avienta
                self.vx = if self.der { paso } else { -paso };
                self.vy = -120.0;
                self.hace = Hace::Vuela;
                self.sobre = None;
            } else {
                self.der = !self.der;
            }
        }
    }

    /// Brinca contento con un corazón, mirando hacia `der`.
    pub fn cariño(&mut self, der: bool) {
        self.der = der;
        self.corazon = CORAZON_DURA;
        self.cariño_espera = 4.0;
        self.vy = -320.0;
        self.vx = 0.0;
        self.vuelve = None;
        self.hace = Hace::Vuela;
    }

    /// Para saludarse con otro: anda suelto y no acaba de saludar.
    pub fn saludable(&self) -> Option<(f32, Option<&str>)> {
        let libre = matches!(self.hace, Hace::Camina | Hace::Para | Hace::Persigue | Hace::Sienta | Hace::Baila);
        (libre && self.cariño_espera <= 0.0).then_some((self.x, self.sobre.as_deref()))
    }

    fn cae(&mut self) {
        self.sobre = None;
        self.vuelve = None;
        self.vx = 0.0;
        self.vy = 0.0;
        self.hace = Hace::Vuela;
    }

    /// Lo blandito de la baba (squash & stretch): se aplasta para agarrar
    /// vuelo, se estira en el aire, se embarra al caer y tiembla como
    /// gelatina; quieta tiembla poquito, dormida respira.
    fn forma(&self) -> Forma {
        if !self.salta {
            return Forma::Normal;
        }
        match &self.hace {
            Hace::Vuela => match self.vy.abs() {
                v if v > 160.0 => Forma::Estirada,
                v if v > 70.0 => Forma::Alta,
                _ => Forma::Normal,
            },
            // como gusanito: se estira y se encoge
            Hace::Trepa { .. } => {
                if (self.reloj / 0.22) as u32 % 2 == 1 { Forma::Alta } else { Forma::Ancha }
            }
            Hace::Baila => {
                if (self.reloj * 2.2).fract() < 0.5 { Forma::Ancha } else { Forma::Alta }
            }
            h => {
                let carga = matches!(h, Hace::Camina | Hace::Persigue);
                if carga && self.carga < 0.08 {
                    return Forma::Plasta;
                }
                if carga && self.carga < 0.18 {
                    return Forma::Aplastada;
                }
                // al caer: plasta → aplastada → rebota alta → normal → ancha
                match self.aterrizo {
                    t if t < 0.07 => Forma::Plasta,
                    t if t < 0.14 => Forma::Aplastada,
                    t if t < 0.23 => Forma::Alta,
                    t if t < 0.31 => Forma::Normal,
                    t if t < 0.40 => Forma::Ancha,
                    _ => {
                        let (vel, umbral) = if matches!(h, Hace::Duerme | Hace::Sienta) { (2.2, 0.1) } else { (5.0, 0.75) };
                        if (self.reloj * vel).sin() > umbral { Forma::Ancha } else { Forma::Normal }
                    }
                }
            }
        }
    }

    fn look(&self) -> Look {
        let paso = (self.reloj / 0.14) as u32 % 2 == 1;
        let pose = match &self.hace {
            Hace::Camina | Hace::VaATrepar { .. } | Hace::Persigue => Pose::Camina(paso),
            Hace::Baila => Pose::Camina((self.reloj * 4.4) as u32 % 2 == 1),
            Hace::Para => Pose::Parado,
            Hace::Sienta => Pose::Sentado,
            Hace::Duerme => Pose::Duerme,
            Hace::Trepa { pared_der, .. } => Pose::Trepa { paso: (self.reloj / 0.2) as u32 % 2 == 1, pared_der: *pared_der },
            Hace::Vuela => Pose::Brinca,
        };
        Look {
            pose,
            forma: self.forma(),
            derecha: self.der,
            parpadeo: self.parpadeo > 0.0,
            corazon: (self.corazon > 0.0).then(|| 1.0 - self.corazon / CORAZON_DURA),
            zzz: (self.hace == Hace::Duerme).then_some(self.reloj),
            notas: (self.hace == Hace::Baila).then_some(self.reloj),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mundo(ventanas: Vec<Ventana>) -> Mundo {
        Mundo { ventanas, abajo: 0.0, borde: 1.0, origen: (0.0, 0.0) }
    }

    #[test]
    fn cae_al_piso() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 42);
        b.aparece(1920.0);
        let m = mundo(vec![]);
        for _ in 0..200 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
        }
        assert_eq!(b.y, 1200.0 - 60.0);
        assert!(b.x >= 0.0 && b.x <= 1920.0 - 48.0);
    }

    #[test]
    fn se_sube_a_las_ventanas_y_no_se_sale() {
        let v = Ventana { id: "a".into(), x: 600.0, y: 300.0, w: 600.0, h: 899.0 };
        let m = mundo(vec![v]);
        let mut b = Bicho::new(3.0, 48.0, 60.0, 7);
        b.aparece(1920.0);
        let mut arriba = false;
        for _ in 0..60_000 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            assert!(b.x >= 0.0 && b.x <= 1920.0 - 48.0, "x = {}", b.x);
            assert!(b.y <= 1200.0 - 60.0 + 0.01, "y = {}", b.y);
            arriba |= b.sobre.is_some();
        }
        assert!(arriba, "nunca trepó");
    }

    #[test]
    fn si_la_ventana_se_va_se_cae() {
        let v = Ventana { id: "a".into(), x: 0.0, y: 300.0, w: 1920.0, h: 899.0 };
        let mut b = Bicho::new(3.0, 48.0, 60.0, 9);
        b.x = 500.0;
        b.y = -60.0;
        for _ in 0..100 {
            b.tick(0.033, &mundo(vec![v.clone()]), 1920.0, 1200.0, None);
        }
        assert_eq!(b.sobre.as_deref(), Some("a"));
        for _ in 0..200 {
            b.tick(0.033, &mundo(vec![]), 1920.0, 1200.0, None);
        }
        assert_eq!(b.sobre, None);
        assert_eq!(b.y, 1200.0 - 60.0);
    }

    #[test]
    fn la_baba_anda_a_brincos() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 5);
        b.salta = true;
        b.x = 900.0;
        b.y = 1140.0;
        b.hace = Hace::Camina;
        b.falta = 5.0;
        let m = mundo(vec![]);
        let mut aire = 0;
        for _ in 0..60 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            aire += usize::from(b.hace == Hace::Vuela);
        }
        assert!((b.x - 900.0).abs() > 20.0, "no se movió");
        assert!(aire > 10, "no brincó");
    }

    #[test]
    fn con_musica_baila() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 11);
        b.musica = true;
        b.aparece(1920.0);
        let m = mundo(vec![]);
        let mut bailo = false;
        for _ in 0..3000 {
            b.tick(0.033, &m, 1920.0, 1200.0, None);
            bailo |= b.hace == Hace::Baila;
        }
        assert!(bailo);
    }

    #[test]
    fn ventanas_en_la_orilla_no_lo_tumban() {
        // una casi fuera de la pantalla, una angostita, una con x negativa
        let vs = vec![
            Ventana { id: "a".into(), x: 1900.0, y: 300.0, w: 300.0, h: 899.0 },
            Ventana { id: "b".into(), x: 500.0, y: 400.0, w: 4.0, h: 799.0 },
            Ventana { id: "c".into(), x: -400.0, y: 200.0, w: 380.0, h: 999.0 },
        ];
        let m = mundo(vs);
        for semilla in 1..40 {
            let mut b = Bicho::new(3.0, 48.0, 60.0, semilla);
            b.aparece(1920.0);
            for _ in 0..3000 {
                b.tick(0.033, &m, 1920.0, 1200.0, Some((1919.0, 1199.0)));
            }
        }
    }

    #[test]
    fn la_baba_se_aplasta_se_estira_y_tiembla() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 3);
        b.salta = true;
        b.x = 900.0;
        b.y = 1140.0;
        b.hace = Hace::Camina;
        b.falta = 20.0;
        let m = mundo(vec![]);
        let mut vistas = std::collections::HashSet::new();
        for _ in 0..400 {
            vistas.insert(format!("{:?}", b.tick(0.016, &m, 1920.0, 1200.0, None).forma));
        }
        for f in ["Plasta", "Aplastada", "Estirada", "Alta", "Normal", "Ancha"] {
            assert!(vistas.contains(f), "nunca hizo {f}: {vistas:?}");
        }
    }

    #[test]
    fn los_duros_no_se_deforman() {
        let mut b = Bicho::new(3.0, 48.0, 60.0, 3);
        b.aparece(1920.0);
        let m = mundo(vec![]);
        for _ in 0..2000 {
            assert_eq!(b.tick(0.033, &m, 1920.0, 1200.0, None).forma, Forma::Normal);
        }
    }
}
