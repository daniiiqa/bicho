//! KDE Plasma (KWin): en Wayland ninguna app puede ver las ventanas de las
//! demás; en Hyprland se las preguntamos por su socket, y en KDE se las
//! pregunta un script de KWin (corre dentro de KWin, que sí las ve) que nos
//! las manda por D-Bus cada vez que algo cambia: ventanas, escritorio, ventana
//! activa, cursor. De regreso, si hay que enfocar una (el "voy" del globo de
//! claudio), el script la enfoca.
//!
//! Con eso se arma el mismo `Mundo` que con Hyprland (hypr.rs pregunta aquí
//! primero), y lo demás funciona igual.

use crate::hypr::{Mundo, Ventana};
use serde_json::Value;
use std::sync::{Arc, Mutex, OnceLock};

const SERVICIO: &str = "org.discada.Bicho";
const RUTA: &str = "/bicho";
/// El nombre del script dentro de KWin.
const SCRIPT: &str = "bicho";

/// El script de KWin. Le cuenta a bicho dónde están las ventanas.
const JS: &str = r#"// bicho: le cuenta a bicho (por D-Bus) dónde están las ventanas
function ventanas() {
    const out = [];
    for (const w of workspace.stackingOrder) {
        if (!w.normalWindow && !w.dialog) continue;
        const g = w.frameGeometry;
        const aqui = w.onAllDesktops || w.desktops.length === 0
            || w.desktops.some(d => d.id === workspace.currentDesktop.id);
        out.push({ id: w.internalId.toString(), x: g.x, y: g.y, w: g.width, h: g.height,
                   pid: w.pid, clase: w.resourceClass, ve: aqui && !w.minimized, completa: w.fullScreen });
    }
    return out;
}
let ultimo = 0;
function enviar(nomas_cursor) {
    // el cursor solo, a lo más ~30 veces por segundo
    const ahora = Date.now();
    if (nomas_cursor === true && ahora - ultimo < 33) return;
    ultimo = ahora;
    const a = workspace.activeWindow;
    const c = workspace.cursorPos;
    const datos = { ventanas: ventanas(), activa: a ? a.internalId.toString() : "", cursor: [c.x, c.y] };
    callDBus("org.discada.Bicho", "/bicho", "org.discada.Bicho", "Estado", JSON.stringify(datos), function (id) {
        if (!id) return;
        for (const w of workspace.windowList()) {
            if (w.internalId.toString() === id) workspace.activeWindow = w;
        }
    });
}
function vigila(w) {
    w.frameGeometryChanged.connect(enviar);
    w.minimizedChanged.connect(enviar);
    w.fullScreenChanged.connect(enviar);
    w.desktopsChanged.connect(enviar);
}
workspace.windowList().forEach(vigila);
workspace.windowAdded.connect(function (w) { vigila(w); enviar(); });
workspace.windowRemoved.connect(enviar);
workspace.windowActivated.connect(enviar);
workspace.currentDesktopChanged.connect(enviar);
workspace.cursorPosChanged.connect(function () { enviar(true); });
enviar();
"#;

/// Una ventana como la cuenta KWin (coordenadas globales).
#[derive(Clone, Debug)]
struct VentanaKde {
    id: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pid: i32,
    /// en el escritorio de ahorita y no minimizada
    ve: bool,
    completa: bool,
}

#[derive(Default)]
struct Estado {
    /// de abajo hacia arriba (la última es la de hasta arriba)
    ventanas: Vec<VentanaKde>,
    activa: Option<String>,
    cursor: Option<(f32, f32)>,
    /// la que hay que enfocar (se la llevamos al script en su siguiente aviso)
    enfoca: Option<String>,
    /// dónde empieza el monitor de los animalitos
    origen: (f32, f32),
    /// ya llegó algo del script
    al_dia: bool,
}

static ESTADO: OnceLock<Arc<Mutex<Estado>>> = OnceLock::new();
static CONEXION: OnceLock<zbus::blocking::Connection> = OnceLock::new();

struct Servicio(Arc<Mutex<Estado>>);

#[zbus::interface(name = "org.discada.Bicho")]
impl Servicio {
    /// Lo que manda el script; devuelve la ventana que hay que enfocar ("" si
    /// ninguna).
    fn estado(&self, json: String) -> String {
        let Ok(mut e) = self.0.lock() else {
            return String::new();
        };
        lee(&mut e, &json);
        e.enfoca.take().unwrap_or_default()
    }
}

fn lee(e: &mut Estado, json: &str) {
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return;
    };
    let f = |v: &Value| v.as_f64().unwrap_or(0.0) as f32;
    // KWin también cuenta como ventanas las capas de los animalitos y el
    // globo (clase "bicho"; a veces aún las de un bicho anterior) y la capa
    // que mide los paneles (pid -1): esas no son ventanas
    let yo = std::process::id() as i64;
    e.ventanas = v["ventanas"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|w| {
            let pid = w["pid"].as_i64().unwrap_or(0);
            pid > 0 && pid != yo && w["clase"].as_str() != Some("bicho")
        })
        .filter_map(|w| {
            Some(VentanaKde {
                id: w["id"].as_str()?.to_string(),
                x: f(&w["x"]),
                y: f(&w["y"]),
                w: f(&w["w"]),
                h: f(&w["h"]),
                pid: w["pid"].as_i64().unwrap_or(0) as i32,
                ve: w["ve"].as_bool().unwrap_or(false),
                completa: w["completa"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    e.activa = v["activa"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from);
    e.cursor = v["cursor"]
        .as_array()
        .filter(|c| c.len() == 2)
        .map(|c| (f(&c[0]), f(&c[1])));
    e.al_dia = true;
}

/// ¿Estamos con KWin (y su script)?
pub fn activo() -> bool {
    ESTADO.get().is_some()
}

/// Sin Hyprland: si hay KWin, se registra el servicio y se le carga el
/// script. false si no hay KWin (o no se pudo).
pub fn arranca() -> bool {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() || activo() {
        return activo();
    }
    match conecta() {
        Ok(()) => true,
        Err(e) => {
            crate::bitacora(&format!("kde: {e}"));
            false
        }
    }
}

fn conecta() -> zbus::Result<()> {
    let estado: Arc<Mutex<Estado>> = Arc::default();
    let conn = zbus::blocking::connection::Builder::session()?
        .name(SERVICIO)?
        .serve_at(RUTA, Servicio(estado.clone()))?
        .build()?;
    // ¿hay KWin?
    let dbus = zbus::blocking::fdo::DBusProxy::new(&conn)?;
    if !dbus.name_has_owner("org.kde.KWin".try_into()?)? {
        return Err(zbus::Error::Failure("no hay KWin".into()));
    }
    // el script, en un archivo (KWin lo carga de ahí)
    let archivo = crate::runtime().join("bicho/kwin-bicho.js");
    let _ = std::fs::create_dir_all(archivo.parent().unwrap_or(std::path::Path::new("/tmp")));
    std::fs::write(&archivo, JS).map_err(|e| zbus::Error::Failure(e.to_string()))?;
    let scripting = zbus::blocking::Proxy::new(
        &conn,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
    )?;
    // el de una vez anterior (si quedó), fuera
    let _: zbus::Result<bool> = scripting.call("unloadScript", &(SCRIPT,));
    let id: i32 = scripting.call(
        "loadScript",
        &(archivo.to_string_lossy().to_string(), SCRIPT),
    )?;
    // Plasma 6: /Scripting/ScriptN; Plasma 5: /N
    let corre = |ruta: String| -> zbus::Result<()> {
        let s = zbus::blocking::Proxy::new(&conn, "org.kde.KWin", ruta, "org.kde.kwin.Script")?;
        s.call::<_, _, ()>("run", &())
    };
    corre(format!("/Scripting/Script{id}")).or_else(|_| corre(format!("/{id}")))?;
    let _ = ESTADO.set(estado);
    let _ = CONEXION.set(conn);
    Ok(())
}

/// Al guardarlos: el script fuera de KWin.
pub fn termina() {
    if let Some(conn) = CONEXION.get()
        && let Ok(s) =
            zbus::blocking::Proxy::new(conn, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting")
    {
        let _: zbus::Result<bool> = s.call("unloadScript", &(SCRIPT,));
    }
}

fn con<T>(f: impl FnOnce(&mut Estado) -> T) -> Option<T> {
    ESTADO.get()?.lock().ok().map(|mut e| f(&mut e))
}

/// Dónde empieza el monitor de los animalitos (lo sabe Wayland).
pub fn pon_origen(origen: (f32, f32)) {
    con(|e| e.origen = origen);
}

/// El mundo según KWin (None: no es KDE, o el script aún no avisa nada).
pub fn mundo() -> Option<Mundo> {
    con(|e| {
        if !e.al_dia {
            return None;
        }
        let (ox, oy) = e.origen;
        let ventanas = e
            .ventanas
            .iter()
            .filter(|v| v.ve && !v.completa)
            .map(|v| Ventana {
                id: v.id.clone(),
                x: v.x - ox,
                y: v.y - oy,
                w: v.w,
                h: v.h,
            })
            .collect();
        let clis = e
            .ventanas
            .iter()
            .filter(|v| v.ve && crate::hypr::es_terminal(v.pid as i64))
            .map(|v| v.id.clone())
            .collect();
        Some(Mundo {
            ventanas,
            abajo: 0.0,
            arriba: 0.0,
            borde: 0.0,
            origen: e.origen,
            todas: e.ventanas.iter().map(|v| v.id.clone()).collect(),
            clis,
            pids: e.ventanas.iter().map(|v| (v.id.clone(), v.pid)).collect(),
            jardines: Vec::new(),
        })
    })
    .flatten()
}

/// El cursor, en coordenadas del monitor.
pub fn cursor() -> Option<(f32, f32)> {
    con(|e| e.cursor.map(|(x, y)| (x - e.origen.0, y - e.origen.1))).flatten()
}

pub fn activa() -> Option<String> {
    con(|e| e.activa.clone()).flatten()
}

/// Que el script la enfoque en su siguiente aviso.
pub fn enfoca(id: &str) {
    con(|e| e.enfoca = Some(id.to_string()));
}

/// La de hasta arriba que se ve en (x, y) (coordenadas del monitor).
pub fn ventana_en(x: f32, y: f32) -> Option<String> {
    con(|e| {
        let (gx, gy) = (x + e.origen.0, y + e.origen.1);
        e.ventanas
            .iter()
            .rev()
            .find(|v| {
                v.ve && !v.completa && gx >= v.x && gx < v.x + v.w && gy >= v.y && gy < v.y + v.h
            })
            .map(|v| v.id.clone())
    })
    .flatten()
}

pub fn pid_de(id: &str) -> Option<i32> {
    con(|e| e.ventanas.iter().find(|v| v.id == id).map(|v| v.pid)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_lo_que_manda_el_script() {
        let mut e = Estado {
            origen: (100.0, 0.0),
            ..Default::default()
        };
        lee(
            &mut e,
            r#"{"ventanas":[{"id":"{a}","x":100,"y":50,"w":800,"h":600,"pid":1,"ve":true,"completa":false},
                            {"id":"{b}","x":300,"y":80,"w":200,"h":200,"pid":2,"ve":false,"completa":false}],
                "activa":"{a}","cursor":[150,60]}"#,
        );
        assert!(e.al_dia);
        assert_eq!(e.ventanas.len(), 2);
        assert_eq!(e.activa.as_deref(), Some("{a}"));
        assert_eq!(e.cursor, Some((150.0, 60.0)));
        // algo roto no rompe lo que ya había
        lee(&mut e, "no es json");
        assert_eq!(e.ventanas.len(), 2);
        // las capas de los animalitos (nuestro pid) no son ventanas
        let yo = std::process::id();
        lee(
            &mut e,
            &format!(
                r#"{{"ventanas":[{{"id":"{{c}}","x":0,"y":0,"w":48,"h":60,"pid":{yo},"ve":true,"completa":false}}],"activa":"","cursor":[0,0]}}"#
            ),
        );
        assert!(e.ventanas.is_empty());
        // ni las de otro bicho (clase bicho), ni la que mide los paneles (pid -1)
        lee(
            &mut e,
            r#"{"ventanas":[{"id":"{d}","x":0,"y":0,"w":48,"h":60,"pid":12345,"clase":"bicho","ve":true,"completa":false},
                                    {"id":"{m}","x":0,"y":0,"w":900,"h":500,"pid":-1,"clase":"","ve":true,"completa":false},
                                    {"id":"{t}","x":9,"y":9,"w":300,"h":200,"pid":12346,"clase":"kitty","ve":true,"completa":false}],"activa":"","cursor":[0,0]}"#,
        );
        assert_eq!(
            e.ventanas.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["{t}"]
        );
    }
}
