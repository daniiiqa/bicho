//! Que reaccionen a lo que pasa: a discada (cambio de look, chapa, las
//! notificaciones de arre) y a la terminal donde viven (un comando que
//! truena, algo compilando).

use super::*;

/// Cada cuánto se fija (s).
const CADA_REACCION: f32 = 0.5;
/// Cada cuánto busca a chapa entre los procesos (s): es recorrer todo /proc,
/// y bloqueada ni se ven, así que sin prisa.
const CADA_CHAPA: f32 = 2.0;
/// Compilando por más de esto (s): se sientan a esperar con su relojito.
const ESPERA_DESDE: f32 = 1.5;

impl App {
    pub(crate) fn tick_reacciones(&mut self, dt: f32) {
        // los compilando llevan la cuenta de cuánto tiempo llevan
        for (_, desde) in self.compilando.values_mut() {
            *desde += dt;
        }
        self.cada_reaccion -= dt;
        if self.cada_reaccion > 0.0 {
            return;
        }
        self.cada_reaccion = CADA_REACCION;
        self.cada_chapa -= CADA_REACCION;
        self.susto_de_look();
        if self.cada_chapa <= 0.0 {
            self.cada_chapa = CADA_CHAPA;
            self.chapa();
        }
        self.arre();
        self.compila();
        self.truenos();
    }

    /// Cambió el look (los colores): un susto, ya que se quitó el telón de
    /// farula (con el telón puesto ni se vería).
    fn susto_de_look(&mut self) {
        if self.susto_pendiente && !hypr::hay_capa("farula") {
            self.susto_pendiente = false;
            for p in &mut self.mascotas {
                p.bicho.asusta();
            }
        }
    }

    /// Bloqueada la sesión (chapa o hyprlock): a dormir; al volver, se
    /// despiertan.
    fn chapa(&mut self) {
        let bloqueada = proceso_vivo(&["chapa", "hyprlock"]);
        if bloqueada != self.bloqueada {
            for p in &mut self.mascotas {
                if bloqueada {
                    p.bicho.a_dormir();
                } else {
                    p.bicho.despierta();
                }
            }
            self.bloqueada = bloqueada;
        }
    }

    /// Llegó una notificación de arre (y no estás en "no molestar"): uno se
    /// asoma hacia la esquina donde salen.
    fn arre(&mut self) {
        let archivo = home(".local/state/arre/state.json");
        let m = mtime(&archivo);
        if m == self.arre_mtime {
            return;
        }
        self.arre_mtime = m;
        let v: Value = std::fs::read(&archivo)
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or(Value::Null);
        let Some(next) = v["next"].as_u64() else {
            return;
        };
        let nueva = self.arre_next.is_some_and(|antes| next > antes);
        self.arre_next = Some(next);
        if !nueva || v["dnd"].as_bool() == Some(true) {
            return;
        }
        // el que ande más a la derecha (las notificaciones salen arriba a la derecha)
        if let Some(p) = self
            .mascotas
            .iter_mut()
            .filter(|p| !p.bicho.oculto)
            .max_by(|a, b| a.bicho.x.total_cmp(&b.bicho.x))
        {
            p.bicho.avisa(true);
        }
    }

    /// Algo compilando (o actualizando) en una terminal: los que viven ahí
    /// esperan con su relojito, y cuando acaba celebran.
    fn compila(&mut self) {
        let con_bichos: Vec<String> = self
            .mascotas
            .iter()
            .filter(|p| !p.bicho.oculto)
            .filter_map(|p| p.bicho.dentro.clone())
            .filter(|id| self.mundo.clis.contains(id))
            .collect();
        let mut nuevo = HashMap::new();
        for id in con_bichos {
            if nuevo.contains_key(&id) {
                continue;
            }
            let Some(&(_, pid)) = self.mundo.pids.iter().find(|(v, _)| v == &id) else {
                continue;
            };
            let si = hypr::compilando(pid);
            let antes = self.compilando.get(&id).copied();
            let desde = match antes {
                Some((true, d)) if si => d,
                _ => 0.0,
            };
            // acabó de compilar después de esperar: ¡a celebrar!
            let acabo = !si && antes.is_some_and(|(c, d)| c && d >= ESPERA_DESDE);
            for p in self
                .mascotas
                .iter_mut()
                .filter(|p| p.bicho.dentro.as_ref() == Some(&id))
            {
                p.bicho.espera = si && desde >= ESPERA_DESDE;
                if acabo {
                    p.bicho.celebra();
                }
            }
            nuevo.insert(id, (si, desde));
        }
        // los que ya no andan en una terminal compilando no esperan
        for p in &mut self.mascotas {
            if !p
                .bicho
                .dentro
                .as_ref()
                .is_some_and(|id| nuevo.contains_key(id))
            {
                p.bicho.espera = false;
            }
        }
        self.compilando = nuevo;
    }

    /// Tronó un comando en una terminal (lo avisa el shell, ver
    /// `bicho shell`): los que viven ahí se asustan.
    fn truenos(&mut self) {
        let dir = runtime().join("bicho");
        let Ok(entradas) = std::fs::read_dir(&dir) else {
            return;
        };
        for e in entradas.flatten() {
            let nombre = e.file_name().to_string_lossy().to_string();
            let Some(pid) = nombre
                .strip_prefix("truena.")
                .and_then(|p| p.parse::<i32>().ok())
            else {
                continue;
            };
            // el shell ya no existe: su archivo sobra
            if !std::path::Path::new(&format!("/proc/{pid}")).exists() {
                let _ = std::fs::remove_file(e.path());
                self.truenos.remove(&pid);
                continue;
            }
            let Some(hora) = std::fs::read_to_string(e.path())
                .ok()
                .and_then(|t| t.split_whitespace().nth(1)?.parse::<u64>().ok())
            else {
                continue;
            };
            let antes = self.truenos.insert(pid, hora);
            // el primero que vemos de cada shell es de antes de que lo viéramos
            if antes.is_none_or(|a| a >= hora) {
                continue;
            }
            let Some(ventana) = hypr::ventana_del_proceso(&self.mundo, pid) else {
                continue;
            };
            for p in self
                .mascotas
                .iter_mut()
                .filter(|p| p.bicho.dentro.as_ref() == Some(&ventana))
            {
                p.bicho.asusta();
            }
        }
    }
}

/// ¿Corre algún proceso con alguno de esos nombres?
fn proceso_vivo(nombres: &[&str]) -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return false;
    };
    dir.flatten().any(|e| {
        let n = e.file_name();
        n.to_string_lossy().bytes().all(|b| b.is_ascii_digit())
            && std::fs::read_to_string(e.path().join("comm"))
                .is_ok_and(|c| nombres.contains(&c.trim()))
    })
}
