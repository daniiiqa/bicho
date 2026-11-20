//! Las visitas al jardín (el de discada, `jardin`): el animalito que se mete
//! a la ventana del jardín (solito, o porque tú lo metiste con SUPER) se va a
//! pasear allá adentro, en 3D, y aquí no se ve.
//!
//! Se platican por dos archivos en `$XDG_RUNTIME_DIR/jardin/`:
//! - `visitas.json` (lo escribe bicho): quién anda allá, con su arte (cada
//!   pose como cuadrícula de colores) y sus colores del look.
//! - `salen.json` (lo escribe el jardín): a quién ya le tocó salir. bicho lo
//!   saca brincando de la ventana y borra el archivo.
//!
//! Si cierras el jardín, salen solos (su ventana ya no está). Mientras haya
//! visitas, `visitas.json` se reescribe cada 2 s: si bicho se guarda, el
//! jardín ve que ya no se actualiza y deja de dibujarlos.

use super::*;

fn carpeta() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("jardin"))
}

/// Cómo se llama cada uno para el jardín: su lugar y su especie.
fn clave(i: usize, e: &arte::Especie) -> String {
    format!("{i}:{}", e.nombre)
}

/// Una pose, mirando a la derecha, como renglones de dígitos (el color de
/// cada celda: 0 vacío, 1 contorno, 2 cuerpo…).
fn cuadro(e: &arte::Especie, pose: arte::Pose) -> Vec<String> {
    let look = arte::Look {
        pose,
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
    };
    arte::grid(e, &look)
        .iter()
        .map(|fila| fila.iter().map(|&c| char::from_digit(c as u32, 36).unwrap_or('0')).collect())
        .collect()
}

impl App {
    pub(crate) fn tick_jardin(&mut self) {
        let Some(dir) = carpeta() else { return };
        // ¿a quién ya le tocó salir?
        let salen = dir.join("salen.json");
        if let Ok(t) = std::fs::read_to_string(&salen) {
            let _ = std::fs::remove_file(&salen);
            let ids: Vec<String> = serde_json::from_str(&t).unwrap_or_default();
            for (i, p) in self.mascotas.iter_mut().enumerate() {
                if p.bicho.en_jardin && ids.contains(&clave(i, p.especie)) {
                    p.bicho.sal_del_jardin();
                }
            }
        }
        // quién anda allá (solo se reescribe si cambió)
        let visitas: Vec<Value> = self
            .mascotas
            .iter()
            .enumerate()
            .filter(|(_, p)| p.bicho.en_jardin)
            .map(|(i, p)| {
                let e = p.especie;
                serde_json::json!({
                    "id": clave(i, e),
                    "especie": e.nombre,
                    "nombre": p.nombre,
                    "de_casa": p.bicho.de_casa,
                    "brinca": e.brinca,
                    "colores": p.colores.iter().map(|c| format!("#{:06x}", c & 0xff_ffff)).collect::<Vec<_>>(),
                    "poses": {
                        "camina_a": cuadro(e, arte::Pose::Camina(false)),
                        "camina_b": cuadro(e, arte::Pose::Camina(true)),
                        "parado": cuadro(e, arte::Pose::Parado),
                        "sentado": cuadro(e, arte::Pose::Sentado),
                        "duerme": cuadro(e, arte::Pose::Duerme),
                    },
                })
            })
            .collect();
        // con visitas, se reescribe cada 2 s aunque no cambie: es su latido
        // (si bicho se guarda o truena, el jardín ve el archivo viejo y ya no
        // los dibuja)
        let hay = !visitas.is_empty();
        let texto = Value::Array(visitas).to_string();
        let latido = hay && self.jardin_cuando.elapsed().as_secs_f32() > 2.0;
        if self.jardin_escrito.as_deref() != Some(texto.as_str()) || latido {
            self.jardin_cuando = Instant::now();
            let _ = std::fs::create_dir_all(&dir);
            let tmp = dir.join("visitas.json.tmp");
            if std::fs::write(&tmp, &texto).is_ok() {
                let _ = std::fs::rename(&tmp, dir.join("visitas.json"));
            }
            self.jardin_escrito = Some(texto);
        }
    }
}
