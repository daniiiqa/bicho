//! Que jueguen entre ellos: la correteada. De vez en cuando, dos libres en
//! el mismo suelo (el piso, encima de la misma ventana o dentro de la misma)
//! se ponen a jugar: uno la trae y corretea al otro; cuando lo alcanza,
//! cambian (al agarrado le sale un "!", el que lo agarró brinca contento, y
//! el nuevo cuenta un ratito antes de salir). Unas rondas y a descansar.
//! Si andan michi y la baba, michi empieza correteando a la baba.

use super::*;

/// Cada cuánto, en promedio, intentan jugar (s).
const ENTRE_JUEGOS: (f32, f32) = (45.0, 120.0);
/// Si no hubo con quién, se vuelve a ver en tanto (s).
const REINTENTA: f32 = 6.0;
/// Más lejos que esto (px) no se ven para jugar.
const LEJOS: f32 = 700.0;
/// Lo más que dura una correteada (s).
const DURA: f32 = 30.0;
/// Lo que cuenta el nuevo que la trae antes de salir (s).
const CUENTA: f32 = 0.9;

/// Una correteada en curso.
pub(crate) struct Juego {
    /// el que la trae
    pub(crate) la_trae: usize,
    /// el que huye
    pub(crate) huye: usize,
    pub(crate) rondas: u8,
    /// a cuántas rondas juegan
    pub(crate) hasta: u8,
    pub(crate) tiempo: f32,
}

impl App {
    fn azar(&mut self) -> f32 {
        // xorshift con la semilla de la app
        let mut x = self.semilla | 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.semilla = x;
        (x >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Antes de que se muevan: empezar, seguir o acabar la correteada.
    pub(crate) fn tick_juego(&mut self, dt: f32) {
        match self.juego.take() {
            None => {
                self.cada_juego -= dt;
                if self.cada_juego <= 0.0 {
                    self.empieza_juego();
                }
            }
            Some(mut j) => {
                j.tiempo += dt;
                if self.sigue_juego(&mut j) {
                    self.juego = Some(j);
                }
            }
        }
    }

    fn empieza_juego(&mut self) {
        self.cada_juego = REINTENTA;
        let n = self.mascotas.len();
        let mut parejas = Vec::new();
        for i in 0..n {
            for k in i + 1..n {
                let (a, b) = (&self.mascotas[i].bicho, &self.mascotas[k].bicho);
                if a.puede_jugar()
                    && b.puede_jugar()
                    && a.suelo() == b.suelo()
                    && (a.x - b.x).abs() < LEJOS
                {
                    parejas.push((i, k));
                }
            }
        }
        if parejas.is_empty() {
            return;
        }
        let nombre = |i: usize| self.mascotas[i].especie.nombre;
        // michi y la baba: michi la trae
        let michi_baba = parejas
            .iter()
            .find_map(|&(i, k)| match (nombre(i), nombre(k)) {
                ("michi", "baba") => Some((i, k)),
                ("baba", "michi") => Some((k, i)),
                _ => None,
            });
        let (la_trae, huye) = match michi_baba {
            Some(p) => p,
            None => {
                let (i, k) = parejas[(self.azar() * parejas.len() as f32) as usize % parejas.len()];
                if self.azar() < 0.5 { (i, k) } else { (k, i) }
            }
        };
        let hasta = 3 + (self.azar() * 4.0) as u8;
        self.juego = Some(Juego {
            la_trae,
            huye,
            rondas: 0,
            hasta,
            tiempo: 0.0,
        });
        // arrancan: el que la trae cuenta, para darle ventaja al otro
        let (xa, xb) = (self.mascotas[la_trae].bicho.x, self.mascotas[huye].bicho.x);
        self.mascotas[la_trae].bicho.juega(xb, true, CUENTA);
        self.mascotas[huye].bicho.juega(xa, false, 0.0);
    }

    /// Un cuadro de correteada; false si ya se acabó.
    fn sigue_juego(&mut self, j: &mut Juego) -> bool {
        let (ia, ib) = (j.la_trae, j.huye);
        if ia >= self.mascotas.len() || ib >= self.mascotas.len() {
            return false;
        }
        let (a, b) = (&self.mascotas[ia].bicho, &self.mascotas[ib].bicho);
        // ¿se puede seguir? los dos jugando (o a medio brinco) y en el mismo
        // suelo (si uno se aventó por la orilla, se acabó)
        let en_juego = a.jugando() && b.jugando();
        let mismo_suelo = a.suelo() == b.suelo() || a.en_el_aire() || b.en_el_aire();
        let completo = j.rondas >= j.hasta;
        if !en_juego || !mismo_suelo || completo || j.tiempo > DURA {
            let contento = j.rondas >= 2;
            self.mascotas[ia].bicho.deja_de_jugar(contento);
            self.mascotas[ib].bicho.deja_de_jugar(contento);
            self.cada_juego = ENTRE_JUEGOS.0 + self.azar() * (ENTRE_JUEGOS.1 - ENTRE_JUEGOS.0);
            return false;
        }
        let (xa, xb) = (a.x, b.x);
        // ¡lo alcanzó! (los dos en el piso, juntitos)
        let alcanzo = !a.en_el_aire()
            && !b.en_el_aire()
            && (xa - xb).abs() < a.cw * 0.45
            && a.suelo() == b.suelo();
        if alcanzo && !self.mascotas[ia].bicho.contando() {
            j.rondas += 1;
            (j.la_trae, j.huye) = (ib, ia);
            self.mascotas[ib].bicho.lo_agarraron();
            self.mascotas[ia].bicho.alcanzo();
            self.mascotas[ib].bicho.juega(xa, true, CUENTA);
            self.mascotas[ia].bicho.juega(xb, false, 0.0);
            return true;
        }
        self.mascotas[ia].bicho.juega(xb, true, CUENTA);
        self.mascotas[ib].bicho.juega(xa, false, 0.0);
        true
    }
}
