//! `bicho menu`: el menú de los animalitos (SUPER+Shift+B), con jale como
//! los demás menús de discada (config-edit.rasi, el mismo que el centro de
//! ajustes). Cada cambio se guarda en ~/.config/bicho/config.json, los
//! sueltos se relanzan con él y el menú vuelve a abrir en la misma fila.

use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const I_POWER: &str = "\u{f0425}";
const I_PAW: &str = "\u{f03e9}";
const I_SPEED: &str = "\u{f04c5}";
const I_SIZE: &str = "\u{f0349}";
const I_LADDER: &str = "\u{f15a2}";
const I_MUSIC: &str = "\u{f075a}";
const I_LOGIN: &str = "\u{f0342}";
const I_MAS: &str = "\u{f0415}";
const I_MENOS: &str = "\u{f0374}";

fn icono(especie: &str) -> &'static str {
    match especie {
        "michi" => "\u{f011b}",
        "baba" => "\u{f058c}",
        "claudio" => "\u{f0674}",
        _ => "\u{f00e4}",
    }
}

/// El idioma de discada (hypr/discada.conf → LANGUAGE=es|en); inglés si no.
pub fn es() -> bool {
    let discada = std::fs::read_to_string(super::home(".config/hypr/discada.conf"))
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("LANGUAGE=")
                    .map(|v| v.split_whitespace().next().unwrap_or("").to_string())
            })
        });
    match discada {
        Some(v) => v == "es",
        // fuera de discada: el idioma del sistema
        None => ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
            .is_some_and(|v| v.starts_with("es")),
    }
}

struct L(bool);
impl L {
    fn t<'a>(&self, es: &'a str, en: &'a str) -> &'a str {
        if self.0 { es } else { en }
    }
    fn si(&self, v: bool) -> &'static str {
        match (v, self.0) {
            (true, true) => "sí",
            (true, false) => "yes",
            (false, _) => "no",
        }
    }
}

fn cfg_path() -> PathBuf {
    super::home(".config/bicho/config.json")
}

fn lee() -> Value {
    let v: Value = std::fs::read(cfg_path())
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or(Value::Null);
    if v.is_object() { v } else { json!({}) }
}

fn guarda(v: &Value) {
    let p = cfg_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(v) {
        let _ = std::fs::write(p, s + "\n");
    }
}

/// jale en modo dmenu; el índice elegido (None: Esc).
/// El menú: jale (el de discada); sin jale, el que haya (kdialog en KDE,
/// rofi, fuzzel, wofi, zenity). El índice elegido (None: Esc).
fn pick(prompt: &str, mesg: &str, filas: &[String], fila: usize) -> Option<usize> {
    if !super::home(".local/bin/jale").is_file() {
        return pick_otro(prompt, mesg, filas);
    }
    pick_jale(prompt, mesg, filas, fila)
}

/// ¿Está ese programa en el PATH?
fn hay(programa: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(programa).is_file()))
}

/// Sin los iconos de Nerd Font (fuera de jale salen como cuadritos).
fn sin_iconos(s: &str) -> String {
    s.chars()
        .filter(|&c| !matches!(c as u32, 0xE000..=0xF8FF | 0xF0000..=0x10FFFF))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Corre un menú y devuelve lo que imprimió (None: lo cerraste).
fn corre(programa: &str, args: &[String], entrada: Option<&str>) -> Option<String> {
    let mut cmd = Command::new(programa);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
    cmd.stdin(if entrada.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut hijo = cmd.spawn().ok()?;
    if let (Some(texto), Some(mut e)) = (entrada, hijo.stdin.take()) {
        let _ = e.write_all(texto.as_bytes());
    }
    let out = hijo.wait_with_output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn pick_otro(prompt: &str, mesg: &str, filas: &[String]) -> Option<usize> {
    let limpias: Vec<String> = filas.iter().map(|f| sin_iconos(f)).collect();
    let lista = limpias.join("\n");
    let titulo = if mesg.is_empty() {
        prompt.to_string()
    } else {
        format!("{prompt}: {mesg}")
    };
    let por_texto = |t: String| limpias.iter().position(|f| *f == t);
    let kde = std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.to_uppercase().contains("KDE"));
    let mut orden = vec!["rofi", "fuzzel", "wofi", "kdialog", "zenity"];
    if kde {
        orden.retain(|m| *m != "kdialog");
        orden.insert(0, "kdialog");
    }
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let Some(m) = orden.into_iter().find(|m| hay(m)) else {
        eprintln!("bicho: no hay menú (jale, rofi, fuzzel, wofi, kdialog o zenity)");
        return None;
    };
    match m {
        "rofi" => corre(
            m,
            &s(&[
                "-dmenu",
                "-i",
                "-format",
                "i",
                "-no-custom",
                "-p",
                prompt,
                "-mesg",
                mesg,
            ]),
            Some(&lista),
        )?
        .parse()
        .ok(),
        "fuzzel" => corre(
            m,
            &s(&["--dmenu", "--index", "--prompt", &format!("{prompt}> ")]),
            Some(&lista),
        )?
        .parse()
        .ok(),
        "wofi" => corre(m, &s(&["--dmenu", "--prompt", prompt]), Some(&lista)).and_then(por_texto),
        "kdialog" => {
            let mut a = s(&["--title", "bicho", "--menu", &titulo]);
            for (i, f) in limpias.iter().enumerate() {
                a.push(i.to_string());
                a.push(f.clone());
            }
            corre(m, &a, None)?.parse().ok()
        }
        _ => {
            let mut a = s(&[
                "--list",
                "--title",
                "bicho",
                "--text",
                &titulo,
                "--column",
                prompt,
                "--hide-header",
            ]);
            a.extend(limpias.iter().cloned());
            corre(m, &a, None).and_then(por_texto)
        }
    }
}

fn pick_jale(prompt: &str, mesg: &str, filas: &[String], fila: usize) -> Option<usize> {
    let jale = super::home(".local/bin/jale");
    let rasi = super::home(".config/jale/config-edit.rasi");
    let mut cmd = Command::new(&jale);
    cmd.args([
        "-dmenu",
        "-i",
        "-format",
        "i",
        "-no-custom",
        "-p",
        prompt,
        "-selected-row",
        &fila.to_string(),
    ]);
    if rasi.exists() {
        cmd.arg("-config").arg(&rasi);
    }
    if !mesg.is_empty() {
        cmd.args(["-mesg", mesg]);
    }
    let mut hijo = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let Some(mut entrada) = hijo.stdin.take() {
        let _ = entrada.write_all(filas.join("\n").as_bytes());
    }
    let out = hijo.wait_with_output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn siguiente(lista: &[f64], ahora: f64) -> f64 {
    let i = lista.iter().position(|&x| (x - ahora).abs() < 1e-6);
    lista[i.map_or(0, |i| (i + 1) % lista.len())]
}

/// Los que andan sueltos; si no hay, los de la config.
fn pets(v: &Value) -> Vec<String> {
    if let Some(sueltos) = super::andando() {
        return sueltos;
    }
    v["pets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| vec!["bicho".into()])
}

/// Si andan sueltos, que tomen la config nueva.
/// Si andan sueltos, que salgan otra vez con `lista` (y la config nueva).
fn relanza(lista: &[String]) {
    if super::running().is_some() {
        super::suelta(lista);
    }
}

/// Abre a Claude (`claude`) en una terminal, en `dir` (o en tu home). Con
/// Hyprland la lanza Hyprland (así no queda como hija de los animalitos);
/// sin Hyprland (KDE, por ejemplo) la abre bicho, suelta.
pub fn abre_claude(dir: Option<PathBuf>) {
    let dir = dir
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| super::home(""));
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        abre_sin_hyprland(&dir);
        return;
    }
    let term = std::env::var("TERMINAL")
        .ok()
        .filter(|t| !t.is_empty() && !t.contains(['"', '\\', '\'']))
        .unwrap_or_else(|| "kitty".into());
    // va dentro de un string de Lua y luego de sh: nada de comillas raras
    let dir = Some(dir)
        .filter(|d| !d.to_string_lossy().contains(['"', '\\', '\'', '\n']))
        .unwrap_or_else(|| super::home(""));
    let cmd = if term == "kitty" {
        format!("kitty --directory '{}' -e claude", dir.display())
    } else {
        format!("cd '{}' && {term} -e claude", dir.display())
    };
    let lua = format!("hl.dsp.exec_cmd(\"{cmd}\")");
    let _ = Command::new("hyprctl")
        .args(["dispatch", &lua])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// La terminal que haya: $TERMINAL, o la de KDE en KDE, o la primera que
/// encuentre. Con lo que hay que darle para que corra `claude` en `dir`.
fn terminal(dir: &std::path::Path) -> Option<(String, Vec<String>)> {
    let d = dir.display().to_string();
    let args = |t: &str| -> Vec<String> {
        let v: Vec<&str> = match t {
            "konsole" => vec!["--workdir", &d, "-e", "claude"],
            "kitty" => vec!["--directory", &d, "-e", "claude"],
            "foot" => vec!["claude"],
            "alacritty" => vec!["--working-directory", &d, "-e", "claude"],
            "wezterm" => vec!["start", "--cwd", &d, "--", "claude"],
            "ghostty" => vec!["-e", "claude"],
            "gnome-terminal" | "kgx" => vec!["--", "claude"],
            _ => vec!["-e", "claude"],
        };
        v.into_iter().map(String::from).collect()
    };
    let hay = |t: &str| {
        std::env::var_os("PATH")
            .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(t).is_file()))
    };
    let kde = std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.to_uppercase().contains("KDE"));
    let pedida = std::env::var("TERMINAL")
        .ok()
        .filter(|t| !t.is_empty() && hay(t));
    let primero = if kde { "konsole" } else { "kitty" };
    let candidatas = [
        primero,
        "konsole",
        "kitty",
        "foot",
        "alacritty",
        "wezterm",
        "ghostty",
        "gnome-terminal",
        "kgx",
        "xterm",
    ];
    let t = pedida.or_else(|| candidatas.iter().find(|t| hay(t)).map(|t| t.to_string()))?;
    let a = args(&t);
    Some((t, a))
}

fn abre_sin_hyprland(dir: &std::path::Path) {
    let Some((t, args)) = terminal(dir) else {
        return;
    };
    let mut cmd = Command::new(&t);
    cmd.args(&args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // que sea su propia sesión: si guardas a los animalitos, la terminal sigue
    use std::os::unix::process::CommandExt;
    // SAFETY: setsid es async-signal-safe
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    // se espera en su hilo (si no, quedaría de zombi al cerrarla)
    if let Ok(mut hijo) = cmd.spawn() {
        std::thread::spawn(move || {
            let _ = hijo.wait();
        });
    }
}

pub fn menu() -> i32 {
    // como los demás menús: si ya hay uno abierto, se cierra
    let _ = Command::new("pkill").args(["-x", "jale"]).status();
    let l = L(es());
    let mut fila = 0;
    loop {
        let mut v = lee();
        let sueltos = super::running().is_some();
        let lista = pets(&v);
        let vel = v["speed"].as_f64().unwrap_or(1.0);
        let tam = v["scale"].as_f64().unwrap_or(3.0);
        let trepa = v["climb"].as_bool().unwrap_or(true);
        let baila = v["dance"].as_bool().unwrap_or(true);
        let login = v["login"].as_bool().unwrap_or(false);
        let filas = vec![
            format!(
                "{I_POWER}  {}",
                if sueltos {
                    l.t("andan sueltos — guardarlos", "out and about — put away")
                } else {
                    l.t("guardados — soltarlos", "put away — let them out")
                }
            ),
            format!("{I_PAW}  {}: {} ›", l.t("quiénes", "who"), lista.join(", ")),
            format!("{I_SPEED}  {}: {vel}×", l.t("velocidad", "speed")),
            format!("{I_SIZE}  {}: {tam}", l.t("tamaño", "size")),
            format!(
                "{I_LADDER}  {}: {}",
                l.t("trepan ventanas", "climb windows"),
                l.si(trepa)
            ),
            format!(
                "{I_MUSIC}  {}: {}",
                l.t("bailan con música", "dance to music"),
                l.si(baila)
            ),
            format!(
                "{I_LOGIN}  {}: {}",
                l.t("al iniciar sesión", "at login"),
                l.si(login)
            ),
            format!("‹ {}", l.t("cerrar", "close")),
        ];
        let mesg = l.t("los animalitos de discada", "discada's little critters");
        let Some(i) = pick("bicho", mesg, &filas, fila) else {
            return 0;
        };
        fila = i;
        match i {
            0 => {
                if sueltos {
                    super::guarda();
                } else {
                    super::suelta(&lista);
                }
                // que se alcance a ver el cambio en el menú
                for _ in 0..40 {
                    if (super::running().is_some()) != sueltos {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
            }
            1 => {
                quienes(&l);
                continue;
            }
            2 => v["speed"] = json!(siguiente(&[0.5, 1.0, 1.5, 2.0], vel)),
            3 => v["scale"] = json!(siguiente(&[2.0, 3.0, 4.0, 5.0], tam) as u64),
            4 => v["climb"] = json!(!trepa),
            5 => v["dance"] = json!(!baila),
            6 => {
                v["login"] = json!(!login);
                guarda(&v);
                continue;
            }
            _ => return 0,
        }
        if (2..=5).contains(&i) {
            guarda(&v);
            relanza(&lista);
        }
    }
}

/// Quién sale: quitar a uno o agregar otro.
fn quienes(l: &L) {
    let mut fila = 0;
    loop {
        let mut v = lee();
        let mut lista = pets(&v);
        let mut filas: Vec<String> = lista
            .iter()
            .map(|p| format!("{I_MENOS}  {} {}  {p}", l.t("quitar", "remove"), icono(p)))
            .collect();
        let nuevos: Vec<&str> = crate::arte::todas().iter().map(|e| e.nombre).collect();
        filas.extend(
            nuevos
                .iter()
                .map(|p| format!("{I_MAS}  {} {}  {p}", l.t("agregar", "add"), icono(p))),
        );
        filas.push(format!("‹ {}", l.t("volver", "back")));
        let mesg = l.t(
            "puedes repetir: tres michis, por qué no",
            "repeats are fine: why not three cats",
        );
        let Some(i) = pick(l.t("quiénes", "who"), mesg, &filas, fila) else {
            return;
        };
        fila = i;
        if i < lista.len() {
            // siempre queda uno
            if lista.len() == 1 {
                continue;
            }
            lista.remove(i);
            fila = i.min(lista.len() - 1);
        } else if let Some(p) = nuevos.get(i - lista.len()) {
            lista.push((*p).to_string());
            fila = i + 1;
        } else {
            return;
        }
        v["pets"] = json!(lista);
        guarda(&v);
        relanza(&lista);
    }
}

/// `bicho --auto` (al iniciar sesión): solo si el menú dice que sí.
pub fn al_iniciar() -> bool {
    lee()["login"].as_bool().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuera_de_jale_sin_iconos() {
        assert_eq!(sin_iconos("\u{f0415}  más bichos"), "más bichos");
        assert_eq!(sin_iconos("\u{e62b} sí"), "sí");
        assert_eq!(sin_iconos("¿ocupas ayuda?"), "¿ocupas ayuda?");
    }
}
