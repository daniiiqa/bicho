//! Lo que el bicho sabe del escritorio, por el socket de Hyprland (sin
//! lanzar hyprctl 30 veces por segundo). Sin Hyprland: solo hay piso.

use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

fn socket() -> Option<PathBuf> {
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let run = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(
        PathBuf::from(run)
            .join("hypr")
            .join(sig)
            .join(".socket.sock"),
    )
}

pub fn pide(cmd: &str) -> Value {
    let Some(path) = socket() else {
        return Value::Null;
    };
    let Ok(mut s) = UnixStream::connect(path) else {
        return Value::Null;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(300)));
    if s.write_all(format!("j/{cmd}").as_bytes()).is_err() {
        return Value::Null;
    }
    let mut out = Vec::new();
    let _ = s.read_to_end(&mut out);
    serde_json::from_slice(&out).unwrap_or(Value::Null)
}

fn f(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}

pub fn monitor_enfocado() -> Option<String> {
    pide("monitors")
        .as_array()?
        .iter()
        .find(|m| m["focused"].as_bool() == Some(true))?["name"]
        .as_str()
        .map(String::from)
}

/// Una ventana, en coordenadas del monitor del bicho (lógicas).
#[derive(Clone, Debug)]
pub struct Ventana {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Default)]
pub struct Mundo {
    pub ventanas: Vec<Ventana>,
    /// lo que reservan las barras abajo
    pub abajo: f32,
    pub borde: f32,
    /// dónde empieza el monitor (para el cursor)
    pub origen: (f32, f32),
}

pub fn mundo(monitor: &str, borde: f32) -> Mundo {
    let mons = pide("monitors");
    let Some(m) = mons
        .as_array()
        .and_then(|a| a.iter().find(|m| m["name"] == monitor))
    else {
        return Mundo::default();
    };
    let (mx, my) = (f(&m["x"]), f(&m["y"]));
    let ws = m["activeWorkspace"]["id"].as_i64();
    let especial = m["specialWorkspace"]["name"]
        .as_str()
        .filter(|s| !s.is_empty());
    // con el scratchpad abierto se para sobre ese; si no, sobre el normal
    let cuenta = |c: &Value| match especial {
        Some(e) => c["workspace"]["name"].as_str() == Some(e),
        None => c["workspace"]["id"].as_i64() == ws,
    };
    let clientes = pide("clients");
    let ventanas = clientes
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| {
            c["mapped"].as_bool() == Some(true) && c["hidden"].as_bool() != Some(true) && cuenta(c)
        })
        // en pantalla completa no hay de dónde agarrarse
        .filter(|c| c["fullscreen"].as_i64().unwrap_or(0) == 0)
        .map(|c| Ventana {
            id: c["address"].as_str().unwrap_or_default().to_string(),
            x: f(&c["at"][0]) - mx,
            y: f(&c["at"][1]) - my,
            w: f(&c["size"][0]),
            h: f(&c["size"][1]),
        })
        .collect();
    Mundo {
        ventanas,
        abajo: f(&m["reserved"][3]),
        borde,
        origen: (mx, my),
    }
}

pub fn borde() -> f32 {
    pide("getoption general:border_size")["int"]
        .as_f64()
        .unwrap_or(2.0) as f32
}

/// El cursor en coordenadas del monitor que empieza en `origen`.
pub fn cursor(origen: (f32, f32)) -> Option<(f32, f32)> {
    let c = pide("cursorpos");
    c.is_object()
        .then(|| (f(&c["x"]) - origen.0, f(&c["y"]) - origen.1))
}
