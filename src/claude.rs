//! claudio y Claude Code: los hooks de Claude Code corren
//! `bicho claude <estado>` (trabajando, listo, pregunta, fin) y aquí queda el
//! estado de cada sesión en un archivito, con el pid de `claude` y de sus
//! papás (para saber en qué ventana anda). Los animalitos lo leen.
//!
//! Hooks (en ~/.claude/settings.json):
//!   UserPromptSubmit, PreToolUse, PostToolUse → bicho claude trabajando
//!   Stop                                      → bicho claude listo
//!   Notification (permiso, pregunta)          → bicho claude pregunta
//!   SessionEnd                                → bicho claude fin
//! En PreToolUse, si la herramienta es AskUserQuestion (te pregunta algo),
//! cuenta como pregunta.

use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Sin noticias de una sesión en tanto tiempo (s), ya no cuenta.
const VIEJA: u64 = 30 * 60;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Estado {
    Trabajando,
    Listo,
    Pregunta,
}

#[derive(Clone, Debug)]
pub struct Sesion {
    pub id: String,
    pub estado: Estado,
    /// cuándo cambió (segundos unix)
    pub t: u64,
    /// `claude` y sus papás: la terminal está entre ellos
    pub pids: Vec<i32>,
}

fn dir() -> PathBuf {
    super::runtime().join("bicho/claude")
}

fn ahora() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn papa(pid: i32) -> Option<i32> {
    // /proc/PID/stat: "pid (comm) estado ppid ..."; el comm puede traer espacios
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    s.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn comm(pid: i32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// De quien corre el hook hacia arriba: desde `claude` hasta antes de init.
fn papas() -> Vec<i32> {
    let mut cadena = Vec::new();
    let mut p = std::process::id() as i32;
    while let Some(pp) = papa(p).filter(|&pp| pp > 1) {
        cadena.push(pp);
        p = pp;
        if cadena.len() > 32 {
            break;
        }
    }
    // lo de antes de claude (el sh del hook) no sirve
    match cadena.iter().position(|&p| comm(p) == "claude") {
        Some(i) => cadena.split_off(i),
        None => cadena,
    }
}

/// `bicho claude <estado>`: lo que corre cada hook. No imprime nada (en
/// UserPromptSubmit lo impreso le llegaría a Claude) y siempre sale bien.
pub fn hook(estado: &str) -> i32 {
    let mut entrada = String::new();
    let _ = std::io::stdin().read_to_string(&mut entrada);
    let v: Value = serde_json::from_str(&entrada).unwrap_or(Value::Null);
    let id: String = v["session_id"]
        .as_str()
        .unwrap_or("sesion")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let id = if id.is_empty() {
        "sesion".to_string()
    } else {
        id
    };
    let archivo = dir().join(&id);
    let estado = match estado {
        // te pregunta algo con opciones: eso es preguntar, no trabajar
        "trabajando" if v["tool_name"].as_str() == Some("AskUserQuestion") => "pregunta",
        "trabajando" | "listo" | "pregunta" => estado,
        "fin" => {
            let _ = std::fs::remove_file(&archivo);
            return 0;
        }
        _ => return 0,
    };
    let pids: Vec<String> = papas().iter().map(|p| p.to_string()).collect();
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(
        &archivo,
        format!("{estado} {} {}\n", ahora(), pids.join(",")),
    );
    0
}

fn lee(id: &str, texto: &str) -> Option<Sesion> {
    let mut partes = texto.split_whitespace();
    let estado = match partes.next()? {
        "trabajando" => Estado::Trabajando,
        "listo" => Estado::Listo,
        "pregunta" => Estado::Pregunta,
        _ => return None,
    };
    let t = partes.next()?.parse().ok()?;
    let pids = partes
        .next()
        .unwrap_or("")
        .split(',')
        .filter_map(|p| p.parse().ok())
        .collect();
    Some(Sesion {
        id: id.to_string(),
        estado,
        t,
        pids,
    })
}

/// Las sesiones vivas. Las de un `claude` que ya no existe (o sin noticias
/// hace mucho) se borran.
pub fn sesiones() -> Vec<Sesion> {
    let Ok(entradas) = std::fs::read_dir(dir()) else {
        return Vec::new();
    };
    let ya = ahora();
    let mut out = Vec::new();
    for e in entradas.flatten() {
        let id = e.file_name().to_string_lossy().to_string();
        let sesion = std::fs::read_to_string(e.path())
            .ok()
            .and_then(|t| lee(&id, &t));
        let viva = sesion.as_ref().is_some_and(|s| {
            ya.saturating_sub(s.t) < VIEJA
                && s.pids
                    .first()
                    .is_some_and(|p| std::path::Path::new(&format!("/proc/{p}")).exists())
        });
        match sesion {
            Some(s) if viva => out.push(s),
            _ => {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    out.sort_by_key(|s| s.t);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_lo_que_escribe_el_hook() {
        let s = lee("abc", "pregunta 1700000000 123,45,1\n").unwrap();
        assert_eq!(
            (s.estado, s.t, s.pids),
            (Estado::Pregunta, 1700000000, vec![123, 45, 1])
        );
        assert!(lee("x", "quien-sabe 1 2").is_none());
        assert!(lee("x", "").is_none());
    }

    #[test]
    fn el_papa_de_uno_es_quien_lo_lanzo() {
        assert_eq!(
            papa(std::process::id() as i32),
            Some(std::os::unix::process::parent_id() as i32)
        );
    }
}
