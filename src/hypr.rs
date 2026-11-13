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
    /// todas las ventanas abiertas, en cualquier escritorio (su address):
    /// para saber si una que no se ve está en otro lado o ya la cerraron
    pub todas: Vec<String>,
    /// las que son terminales (ahí las letras son repisas)
    pub clis: Vec<String>,
}

/// ¿La ventana de ese pid es una terminal?
fn es_terminal(pid: i64) -> bool {
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
    matches!(
        comm.trim(),
        "kitty"
            | "foot"
            | "alacritty"
            | "wezterm-gui"
            | "ghostty"
            | "konsole"
            | "gnome-terminal-"
            | "xterm"
            | "st"
            | "urxvt"
    )
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
    let todas = clientes
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c["address"].as_str().map(String::from))
        .collect();
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
    let clis = clientes
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| cuenta(c) && es_terminal(c["pid"].as_i64().unwrap_or(0)))
        .filter_map(|c| c["address"].as_str().map(String::from))
        .collect();
    Mundo {
        ventanas,
        abajo: f(&m["reserved"][3]),
        borde,
        origen: (mx, my),
        todas,
        clis,
    }
}

/// La ventana que se ve en (x, y) (coordenadas del monitor) en el escritorio
/// de ahorita: su address. Si se enciman, la que tuvo el foco más reciente.
pub fn ventana_en(monitor: &str, x: f32, y: f32) -> Option<String> {
    let mons = pide("monitors");
    let m = mons.as_array()?.iter().find(|m| m["name"] == monitor)?;
    let (gx, gy) = (x + f(&m["x"]), y + f(&m["y"]));
    let ws = m["activeWorkspace"]["id"].as_i64();
    let especial = m["specialWorkspace"]["name"]
        .as_str()
        .filter(|s| !s.is_empty());
    let clientes = pide("clients");
    clientes
        .as_array()?
        .iter()
        .filter(|c| c["mapped"].as_bool() == Some(true) && c["hidden"].as_bool() != Some(true))
        .filter(|c| match especial {
            Some(e) => c["workspace"]["name"].as_str() == Some(e),
            None => c["workspace"]["id"].as_i64() == ws,
        })
        .filter(|c| {
            let (cx, cy, w, h) = (
                f(&c["at"][0]),
                f(&c["at"][1]),
                f(&c["size"][0]),
                f(&c["size"][1]),
            );
            gx >= cx && gx < cx + w && gy >= cy && gy < cy + h
        })
        .filter(|c| c["fullscreen"].as_i64().unwrap_or(0) == 0)
        .min_by_key(|c| c["focusHistoryID"].as_i64().unwrap_or(i64::MAX))
        .and_then(|c| c["address"].as_str())
        .map(String::from)
}

/// El pid de la ventana con ese address.
pub fn pid_de(id: &str) -> Option<i32> {
    pide("clients")
        .as_array()?
        .iter()
        .find(|c| c["address"].as_str() == Some(id))?["pid"]
        .as_i64()
        .map(|p| p as i32)
}

/// En qué carpeta anda una ventana: la del proceso más adentro (la terminal
/// → el shell → nvim, claude, taco…). Los ayudantes de kitty no cuentan, ni
/// los procesos que ya se murieron.
pub fn carpeta(pid: i32) -> Option<PathBuf> {
    fn hijos(pid: i32) -> Vec<i32> {
        std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"))
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|p| p.parse().ok())
            .collect()
    }
    fn cwd(pid: i32) -> Option<PathBuf> {
        std::fs::read_link(format!("/proc/{pid}/cwd"))
            .ok()
            .filter(|p| p.is_dir())
    }
    fn cuenta(pid: i32) -> bool {
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        comm.trim() != "kitten" && cwd(pid).is_some()
    }
    // a lo hondo; el más hondo gana (y entre iguales, el más nuevo)
    let mut mejor = (0usize, pid);
    let mut pila = vec![(pid, 0usize)];
    while let Some((p, hondo)) = pila.pop() {
        if hondo > 0 && cuenta(p) && (hondo, p) > mejor {
            mejor = (hondo, p);
        }
        if hondo < 12 {
            pila.extend(hijos(p).into_iter().map(|h| (h, hondo + 1)));
        }
    }
    cwd(mejor.1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_carpeta_es_la_del_proceso_mas_adentro() {
        // yo → sh (en /tmp) → sleep (en /usr): gana el más hondo
        let mut hijo = std::process::Command::new("sh")
            .args(["-c", "cd /usr && exec sleep 5 & wait"])
            .current_dir("/tmp")
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(200));
        let dir = carpeta(hijo.id() as i32);
        let _ = hijo.kill();
        let _ = hijo.wait();
        assert_eq!(dir, Some(PathBuf::from("/usr")));
    }
}
