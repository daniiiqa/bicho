//! `bicho actualiza`: baja el bicho ya compilado de la release `edge` de
//! GitHub (la que se publica en cada push), revisa su sha256, reemplaza a
//! este, y si los animalitos andan sueltos los vuelve a soltar con el nuevo.
//! Usa `curl` y `sha256sum`, que vienen en cualquier distro.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EDGE: &str = "https://github.com/daniiiqa/bicho/releases/download/edge";
/// El commit del que salió este binario (lo pone el CI al compilar; en uno
/// compilado a mano no hay).
const COMMIT: Option<&str> = option_env!("GITHUB_SHA");

fn baja(url: &str, a: &Path) -> Result<(), String> {
    let ok = Command::new("curl")
        .args(["-fsSL", "--retry", "2", "-o"])
        .arg(a)
        .arg(url)
        .status()
        .map_err(|_| "no está curl".to_string())?
        .success();
    if ok {
        Ok(())
    } else {
        Err(format!("no se pudo bajar {url}"))
    }
}

fn texto(url: &str) -> Result<String, String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--retry", "2", url])
        .stderr(Stdio::null())
        .output()
        .map_err(|_| "no está curl".to_string())?;
    if !out.status.success() {
        return Err(format!("no se pudo bajar {url}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// El sha256 de un archivo (con sha256sum).
fn sha256(archivo: &Path) -> Option<String> {
    let out = Command::new("sha256sum").arg(archivo).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .map(String::from)
}

/// Lo que va a decir el binario de dónde está (el que corre ahorita).
fn este() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("¿dónde estoy? {e}"))
}

pub fn actualiza() -> i32 {
    match intenta() {
        Ok(msj) => {
            println!("{msj}");
            0
        }
        Err(e) => {
            eprintln!("bicho: {e}");
            1
        }
    }
}

fn intenta() -> Result<String, String> {
    if std::env::consts::ARCH != "x86_64" {
        return Err("el compilado es para x86_64; aquí hay que compilarlo (cargo install --git https://github.com/daniiiqa/bicho)".into());
    }
    let nombre = format!("bicho-{}-linux", std::env::consts::ARCH);
    let ultimo = texto(&format!("{EDGE}/bicho.commit"))?;
    if COMMIT == Some(ultimo.as_str()) {
        return Ok(format!(
            "ya está al día ({})",
            &ultimo[..ultimo.len().min(7)]
        ));
    }
    let exe = este()?;
    let dir = exe.parent().ok_or("¿en qué carpeta estoy?")?;
    // se baja junto al de ahorita (misma carpeta: el cambio es de un jalón)
    let nuevo = dir.join(".bicho.nuevo");
    let _ = std::fs::remove_file(&nuevo);
    baja(&format!("{EDGE}/{nombre}"), &nuevo)?;
    let esperado = texto(&format!("{EDGE}/{nombre}.sha256"))?;
    let esperado = esperado.split_whitespace().next().unwrap_or("");
    if sha256(&nuevo).as_deref() != Some(esperado) {
        let _ = std::fs::remove_file(&nuevo);
        return Err(
            "el que se bajó no coincide con su sha256 (¿se cortó la descarga?); no cambié nada"
                .into(),
        );
    }
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&nuevo, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| e.to_string())?;
    // ¿sí corre aquí? (glibc muy vieja, por ejemplo)
    let corre = Command::new(&nuevo)
        .arg("ayuda")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    if let Ok(o) = &corre
        && String::from_utf8_lossy(&o.stderr).contains("GLIBC")
    {
        let _ = std::fs::remove_file(&nuevo);
        return Err(
            "el compilado necesita una glibc más nueva que la de esta distro; no cambié nada"
                .into(),
        );
    }
    std::fs::rename(&nuevo, &exe)
        .map_err(|e| format!("no pude reemplazar {}: {e}", exe.display()))?;
    let mut msj = format!(
        "listo: {} → {}",
        COMMIT.map_or("compilado a mano", |c| &c[..c.len().min(7)]),
        &ultimo[..ultimo.len().min(7)]
    );
    // si andaban sueltos, otra vez, ya con el nuevo
    if let Some(pets) = super::andando() {
        let _ = Command::new(&exe).args(&pets).status();
        msj += " (y los animalitos ya andan con el nuevo)";
    }
    Ok(msj)
}

/// `bicho --version`: el commit del que salió.
pub fn version() -> i32 {
    println!(
        "bicho {}",
        COMMIT.map_or("(compilado a mano)", |c| &c[..c.len().min(7)])
    );
    0
}
