//! Actualizarse en Windows: baja `bicho-x86_64-windows.exe` de la release
//! `edge` (con el curl.exe que trae Windows), revisa su sha256, y se cambia:
//! un .exe que está corriendo no se puede borrar pero sí renombrar, así que
//! el viejo pasa a `bicho.exe.viejo` (se borra al siguiente arranque) y el
//! nuevo toma su lugar. Luego el nuevo arranca, espera a que el viejo se
//! cierre, y suelta a los animalitos.
//!
//! Desde el menú de la bandeja (que de vez en cuando revisa solo si hay
//! otra) o con `bicho actualiza` en una terminal.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};

const EDGE: &str = "https://github.com/daniiiqa/bicho/releases/download/edge";
const NOMBRE: &str = "bicho-x86_64-windows.exe";
/// El commit del que salió este (lo pone el CI; en uno compilado a mano no
/// hay).
pub(crate) const COMMIT: Option<&str> = option_env!("GITHUB_SHA");
/// Que curl no abra una consola negra.
const SIN_VENTANA: u32 = 0x0800_0000;

pub(crate) fn corto(c: &str) -> &str {
    &c[..c.len().min(7)]
}

fn curl() -> Command {
    let sys =
        std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from);
    let mut c = Command::new(sys.join("System32").join("curl.exe"));
    c.args(["-fsSL", "--retry", "2"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(SIN_VENTANA);
    c
}

fn texto(url: &str) -> Result<String, String> {
    let out = curl()
        .arg(url)
        .output()
        .map_err(|_| "no encontré curl.exe".to_string())?;
    if !out.status.success() {
        return Err("no se pudo conectar con GitHub".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn baja(url: &str, a: &Path) -> Result<(), String> {
    let ok = curl()
        .arg("-o")
        .arg(a)
        .arg(url)
        .stdout(Stdio::null())
        .status()
        .map_err(|_| "no encontré curl.exe".to_string())?
        .success();
    if ok {
        Ok(())
    } else {
        Err("no se pudo bajar la versión nueva".into())
    }
}

fn sha256(archivo: &Path) -> Option<String> {
    let datos = std::fs::read(archivo).ok()?;
    let mut h = [0u8; 32];
    let r = unsafe {
        BCryptHash(
            BCRYPT_SHA256_ALG_HANDLE,
            std::ptr::null(),
            0,
            datos.as_ptr(),
            datos.len() as u32,
            h.as_mut_ptr(),
            32,
        )
    };
    (r == 0).then(|| h.iter().map(|b| format!("{b:02x}")).collect())
}

/// ¿Hay otra versión? Some(su commit) si sí.
pub(crate) fn hay_nueva() -> Result<Option<String>, String> {
    // el de Windows (se sube después del .exe; el de Linux, antes)
    let ultimo = texto(&format!("{EDGE}/bicho-windows.commit"))?;
    if ultimo.len() < 7 {
        return Err("GitHub no dijo cuál es la última".into());
    }
    Ok((COMMIT != Some(ultimo.as_str())).then_some(ultimo))
}

/// Baja la nueva y se cambia por ella. Ok(commit nuevo), o Ok(None) si ya
/// estaba al día.
pub(crate) fn cambia() -> Result<Option<String>, String> {
    let Some(ultimo) = hay_nueva()? else {
        return Ok(None);
    };
    let exe = std::env::current_exe().map_err(|e| format!("¿dónde estoy? {e}"))?;
    let dir = exe.parent().ok_or("¿en qué carpeta estoy?")?;
    let nuevo = dir.join("bicho.exe.nuevo");
    let _ = std::fs::remove_file(&nuevo);
    baja(&format!("{EDGE}/{NOMBRE}"), &nuevo)?;
    let esperado = texto(&format!("{EDGE}/{NOMBRE}.sha256"))?;
    let esperado = esperado
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    if sha256(&nuevo).as_deref() != Some(esperado.as_str()) {
        let _ = std::fs::remove_file(&nuevo);
        return Err("la descarga llegó incompleta; no cambié nada".into());
    }
    let viejo = viejo(&exe);
    let _ = std::fs::remove_file(&viejo);
    std::fs::rename(&exe, &viejo).map_err(|e| format!("no pude apartar el viejo: {e}"))?;
    if let Err(e) = std::fs::rename(&nuevo, &exe) {
        // que no se quede sin bicho
        let _ = std::fs::rename(&viejo, &exe);
        return Err(format!("no pude poner el nuevo: {e}"));
    }
    Ok(Some(ultimo))
}

fn viejo(exe: &Path) -> PathBuf {
    exe.with_extension("exe.viejo")
}

/// Al arrancar: el viejo de la vez pasada, fuera.
pub(crate) fn limpia() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(viejo(&exe));
        let _ = std::fs::remove_file(exe.with_file_name("bicho.exe.nuevo"));
    }
}

/// Arranca el nuevo, que espera a que este se cierre (`_espera <pid>`).
pub(crate) fn arranca_nuevo() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .args(["_espera", &std::process::id().to_string()])
            .spawn();
    }
}

/// El nuevo, esperando al viejo (lo más 10 s).
pub(crate) fn espera(pid: u32) {
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            WaitForSingleObject(h, 10_000);
            CloseHandle(h);
        }
    }
}
