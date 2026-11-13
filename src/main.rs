//! bicho: animalitos de pixeles que se pasean por el escritorio de discada.
//! Caminan por el piso, trepan las ventanas, se pasean por arriba de ellas y
//! se avientan por la orilla; se sientan, se duermen (más de noche), bailan
//! si suena música, a veces persiguen al cursor, y si se lo acercas brincan
//! con un corazón. Si dos se topan, se saludan. Toman los colores del look.
//!
//!   bicho [especie…]   los suelta (bicho, michi, baba, claudio; sin nombre, los de
//!                      la config). Si ya andan, los cambia por esos
//!   bicho más ESPECIE  suelta uno más
//!   bicho off          los guarda
//!   bicho toggle       una cosa o la otra (para un atajo)
//!   bicho ver          cómo se ven, en la terminal
//!   bicho menu         el menú (jale), SUPER+Shift+B
//!   bicho --auto       al iniciar sesión: los suelta si el menú dice que sí
//!
//! ~/.config/bicho/config.json:
//!   {"pets": ["bicho", "michi"], "scale": 3, "speed": 1.0, "climb": true,
//!    "dance": true, "login": false}
//! Solo su cuerpo agarra clics (lo demás de la capa deja pasar el mouse): con
//! un clic les das cariño; si los arrastras y los sueltas, salen volando.

mod arte;
mod bicho;
mod claude;
mod claudio;
mod dialogo;
mod globo;
mod hypr;
mod menu;
mod mouse;
mod texto;
mod ventanas;

use serde_json::Value;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::{
    wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface,
};
use smithay_client_toolkit::reexports::client::{Connection, Dispatch, QueueHandle};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, Region},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// un cuadro cuando nomás andan caminando
const CUADRO: Duration = Duration::from_millis(33);
/// y cuando algo se tiene que ver fluido (pegados a una ventana, agarrados,
/// el globo): al ritmo de la pantalla
const CUADRO_RAPIDO: Duration = Duration::from_millis(16);
const BOTON_IZQ: u32 = 0x110;

fn runtime() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, PathBuf::from)
}
fn pidfile() -> PathBuf {
    runtime().join("bicho.pid")
}
fn home(p: &str) -> PathBuf {
    std::env::var_os("HOME")
        .map_or_else(PathBuf::new, PathBuf::from)
        .join(p)
}

fn running() -> Option<i32> {
    let pid: i32 = std::fs::read_to_string(pidfile())
        .ok()?
        .trim()
        .parse()
        .ok()?;
    vivo(pid).then_some(pid)
}

/// Que sea un bicho y que no sea zombi: el menú es el papá de los que suelta
/// y uno guardado se quedaba de zombi mientras el menú seguía abierto (y el
/// menú creía que seguían sueltos: ya no los dejaba salir).
fn vivo(pid: i32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    // "pid (comm) estado …"
    let Some((izq, der)) = stat.rsplit_once(')') else {
        return false;
    };
    izq.ends_with("(bicho") && !der.trim_start().starts_with(['Z', 'X'])
}

fn logfile() -> PathBuf {
    home(".local/state/bicho/bicho.log")
}

/// Lo que truene queda en ~/.local/state/bicho/bicho.log (el proceso anda
/// suelto, sin terminal donde avisar).
fn bitacora(linea: &str) {
    use std::io::Write;
    let p = logfile();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // que no crezca sin fin
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > 256 * 1024) {
        let _ = std::fs::remove_file(&p);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
    {
        let ahora = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let _ = writeln!(f, "[{ahora}] {linea}");
    }
}

fn petsfile() -> PathBuf {
    runtime().join("bicho.pets")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let resto = &args[args.len().min(2)..];
    let code = match args.get(1).map(String::as_str) {
        None | Some("on") => suelta(&[]),
        Some("off") => guarda(),
        Some("toggle") => {
            if running().is_some() {
                guarda()
            } else {
                suelta(&[])
            }
        }
        Some("más" | "mas" | "add") => {
            let mut lista = andando().unwrap_or_else(|| config().pets);
            lista.extend(resto.iter().cloned());
            if resto.is_empty() {
                lista.push("bicho".into());
            }
            suelta(&lista)
        }
        Some("menu") => menu::menu(),
        // lo que corren los hooks de Claude Code
        Some("claude") => claude::hook(resto.first().map_or("", String::as_str)),
        Some("--auto") => {
            if menu::al_iniciar() {
                suelta(&[])
            } else {
                0
            }
        }
        Some("ver") => {
            ver();
            0
        }
        Some("_run") => match run(resto) {
            Ok(()) => 0,
            Err(e) => {
                bitacora(&e);
                eprintln!("bicho: {e}");
                1
            }
        },
        Some("-h" | "--help" | "ayuda") => {
            uso();
            0
        }
        Some(_) => suelta(&args[1..]),
    };
    std::process::exit(code);
}

fn uso() {
    let nombres: Vec<_> = arte::todas().iter().map(|e| e.nombre).collect();
    eprintln!(
        "uso: bicho [especie…] | más ESPECIE | off | toggle | ver | menu\n      bicho claude trabajando|listo|pregunta|fin   (para los hooks de Claude Code)\nespecies: {}",
        nombres.join(", ")
    );
}

/// Los que andan sueltos ahorita.
fn andando() -> Option<Vec<String>> {
    running()?;
    let s = std::fs::read_to_string(petsfile()).ok()?;
    Some(s.split_whitespace().map(String::from).collect())
}

fn suelta(pets: &[String]) -> i32 {
    if let Some(malo) = pets.iter().find(|p| arte::especie(p).is_none()) {
        eprintln!("bicho: no conozco a '{malo}'");
        uso();
        return 2;
    }
    if running().is_some() {
        if pets.is_empty() {
            return 0;
        }
        guarda();
        for _ in 0..40 {
            if running().is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    let Ok(exe) = std::env::current_exe() else {
        return 1;
    };
    let mut cmd = Command::new(exe);
    cmd.arg("_run")
        .args(pets)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    unsafe {
        use std::os::unix::process::CommandExt;
        // doble fork: el que se queda es nieto, no hijo de quien lo suelta. Si
        // fuera hijo del menú (que sigue abierto), al guardarlo se quedaba de
        // zombi y el menú creía que seguía suelto
        cmd.pre_exec(|| {
            if libc::fork() != 0 {
                libc::_exit(0);
            }
            libc::setsid();
            Ok(())
        });
    }
    match cmd.spawn() {
        // el de en medio sale luego luego: recogerlo
        Ok(mut hijo) => i32::from(hijo.wait().is_err()),
        Err(_) => 1,
    }
}

fn guarda() -> i32 {
    if let Some(pid) = running() {
        unsafe { libc::kill(pid, libc::SIGTERM) };
    }
    0
}

fn ver() {
    use arte::{Look, Pose};
    let poses = [
        ("camina", Pose::Camina(false), None, None, None),
        ("paso", Pose::Camina(true), None, None, None),
        ("contento", Pose::Brinca, Some(0.3), None, None),
        ("baila", Pose::Parado, None, None, Some(0.2)),
        ("duerme", Pose::Duerme, None, Some(0.3), None),
    ];
    for &e in arte::todas() {
        let colores = colores(e);
        let grids: Vec<Vec<String>> = poses
            .iter()
            .map(|&(_, pose, corazon, zzz, notas)| {
                let look = Look {
                    pose,
                    derecha: true,
                    parpadeo: false,
                    corazon,
                    zzz,
                    notas,
                    piensa: None,
                    forma: arte::Forma::Normal,
                };
                arte::terminal(&arte::grid(e, &look), &colores)
                    .lines()
                    .map(String::from)
                    .collect()
            })
            .collect();
        println!("  {}", e.nombre);
        for fila in 0..grids[0].len() {
            let linea: Vec<&str> = grids.iter().map(|g| g[fila].as_str()).collect();
            println!("  {}", linea.join("   "));
        }
        let nombres: Vec<String> = poses.iter().map(|p| format!("{:<16}", p.0)).collect();
        println!("  \x1b[2m{}\x1b[0m\n", nombres.join("   "));
        if e.brinca {
            use arte::Forma;
            let formas = [
                ("plasta", Forma::Plasta),
                ("aplastada", Forma::Aplastada),
                ("normal", Forma::Normal),
                ("ancha", Forma::Ancha),
                ("alta", Forma::Alta),
                ("estirada", Forma::Estirada),
            ];
            let grids: Vec<Vec<String>> = formas
                .iter()
                .map(|&(_, forma)| {
                    let look = Look {
                        pose: Pose::Parado,
                        derecha: true,
                        parpadeo: false,
                        corazon: None,
                        zzz: None,
                        notas: None,
                        piensa: None,
                        forma,
                    };
                    arte::terminal(&arte::grid(e, &look), &colores)
                        .lines()
                        .map(String::from)
                        .collect()
                })
                .collect();
            for fila in 0..grids[0].len() {
                let linea: Vec<&str> = grids.iter().map(|g| g[fila].as_str()).collect();
                println!("  {}", linea.join("   "));
            }
            let nombres: Vec<String> = formas.iter().map(|f| format!("{:<16}", f.0)).collect();
            println!("  \x1b[2m{}\x1b[0m\n", nombres.join("   "));
        }
    }
}

struct Config {
    pets: Vec<String>,
    escala: u32,
    baila: bool,
    velocidad: f32,
    trepa: bool,
}

fn config() -> Config {
    let v: Value = std::fs::read(home(".config/bicho/config.json"))
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or(Value::Null);
    let pets: Vec<String> = v["pets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p.as_str())
        .filter(|p| arte::especie(p).is_some())
        .map(String::from)
        .collect();
    Config {
        pets: if pets.is_empty() {
            vec!["bicho".into()]
        } else {
            pets
        },
        baila: v["dance"].as_bool().unwrap_or(true),
        escala: v["scale"].as_u64().unwrap_or(3).clamp(1, 8) as u32,
        velocidad: v["speed"].as_f64().unwrap_or(1.0).clamp(0.1, 5.0) as f32,
        trepa: v["climb"].as_bool().unwrap_or(true),
    }
}

fn colores_path() -> PathBuf {
    home(".config/empalme/colors.json")
}

/// Los colores del look: el cuerpo, el color de la especie; los ojos, el
/// primer plano.
fn colores(e: &arte::Especie) -> [u32; arte::COLORES] {
    let c: Value = std::fs::read(colores_path())
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or(Value::Null);
    let rgb = |keys: &[&str], def: u32| -> u32 {
        keys.iter()
            .find_map(|k| {
                if k.starts_with('#') {
                    Some(*k)
                } else {
                    c[k].as_str()
                }
            })
            .and_then(|h| u32::from_str_radix(h.trim_start_matches('#'), 16).ok())
            .unwrap_or(def)
    };
    let cuerpo = rgb(e.color, 0xc55e12);
    let frente = rgb(&["foreground"], 0xfef7e7);
    let fondo = rgb(&["background"], 0x180904);
    let mezcla = |a: u32, b: u32, t: f32| -> u32 {
        let ch = |sh: u32| {
            let (x, y) = (((a >> sh) & 0xff) as f32, ((b >> sh) & 0xff) as f32);
            ((x + (y - x) * t).round() as u32) << sh
        };
        ch(16) | ch(8) | ch(0)
    };
    let opaco = |rgb: u32| 0xff00_0000 | rgb;
    let mut p = [0u32; arte::COLORES];
    p[arte::CONTORNO as usize] = opaco(mezcla(fondo, 0, 0.3));
    p[arte::CUERPO as usize] = opaco(cuerpo);
    p[arte::BRILLO as usize] = opaco(mezcla(cuerpo, frente, 0.55));
    p[arte::OJO as usize] = opaco(frente);
    p[arte::PUPILA as usize] = opaco(fondo);
    p[arte::CORAZON as usize] = opaco(rgb(&["critical", "color1"], 0xe06c75));
    p[arte::ZZZ as usize] = opaco(frente);
    p[arte::NARIZ as usize] = opaco(mezcla(rgb(&["critical", "color1"], 0xe06c75), frente, 0.35));
    p
}

fn mtime(p: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// ¿Está sonando algo? (playerctl, cada 2 s, en su hilo)
fn oye_musica(suena: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        loop {
            let si = Command::new("playerctl")
                .args(["-a", "status"])
                .stderr(Stdio::null())
                .output()
                .is_ok_and(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .any(|l| l.trim() == "Playing")
                });
            suena.store(si, Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

fn es_de_noche() -> bool {
    let hora = unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm.tm_hour
    };
    !(6..23).contains(&hora)
}

/// Un animalito en pantalla: su capa y su cabeza.
struct Mascota {
    especie: &'static arte::Especie,
    layer: LayerSurface,
    pool: Option<SlotPool>,
    bicho: bicho::Bicho,
    colores: [u32; arte::COLORES],
    margen: (i32, i32),
    /// el rectángulo que agarra clics (x, y, ancho, alto en la capa)
    region: Option<(i32, i32, i32, i32)>,
    /// su cuerpo parado dentro del lienzo: columnas (x0, x1) y alto, en px
    cuerpo: (f32, f32, f32),
    /// lo último que se pintó (dibujo, colores, escala): si no cambia, no se
    /// vuelve a pintar
    pintado: Option<(arte::Grid, [u32; arte::COLORES], i32)>,
}

/// Dónde queda el cuerpo de un animalito parado, dentro de su lienzo.
fn cuerpo(e: &'static arte::Especie, celda: f32) -> (f32, f32, f32) {
    let look = arte::Look {
        pose: arte::Pose::Parado,
        derecha: true,
        parpadeo: false,
        corazon: None,
        zzz: None,
        notas: None,
        piensa: None,
        forma: arte::Forma::Normal,
    };
    let (x0, y0, x1, y1) =
        arte::caja(&arte::grid(e, &look)).unwrap_or((0, 0, arte::ANCHO - 1, arte::ALTO - 1));
    (
        x0 as f32 * celda,
        (x1 + 1) as f32 * celda - 1.0,
        (y1 - y0 + 1) as f32 * celda,
    )
}

struct App {
    registry: RegistryState,
    output: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    seat: SeatState,
    pointer: Option<wl_pointer::WlPointer>,
    arrastre: Option<mouse::Arrastre>,
    dialogo: Option<dialogo::Dialogo>,
    teclado: Option<wl_keyboard::WlKeyboard>,
    /// SUPER apretado (solo se sabe mientras un animalito tiene el teclado,
    /// o sea, mientras lo agarras)
    super_apretado: bool,
    /// le quitamos el teclado al soltarlo; en tanto tiempo puede volver a
    /// tomarlo
    foco_vuelve: Option<f32>,
    /// cada cuánto ve cómo va Claude, cómo iba la vez pasada, y la pregunta
    /// que dejaste para luego
    cada_claude: f32,
    claude_antes: std::collections::HashMap<String, claude::Estado>,
    claude_ignorada: Option<(String, u64)>,
    /// el hilo que ve las letras de las terminales
    texto: texto::Compartido,
    shm: Shm,
    qh: QueueHandle<App>,
    cfg: Config,
    /// el monitor donde viven (el enfocado al soltarlos)
    monitor: Option<String>,
    mascotas: Vec<Mascota>,
    pantalla: (f32, f32),
    configurado: bool,
    escala_buffer: i32,
    semilla: u64,
    mundo: hypr::Mundo,
    borde: f32,
    cursor: Option<(f32, f32)>,
    colores_mtime: Option<SystemTime>,
    musica: Arc<AtomicBool>,
    ultimo: Instant,
    cada_mundo: f32,
    /// lo que no urge (los colores, si es de noche)
    cada_resto: f32,
    /// dónde están las ventanas de veras (`mundo` se va acercando a esto)
    reales: Vec<hypr::Ventana>,
    /// las que vienen deslizándose a su lugar
    deslizando: std::collections::HashSet<String>,
    /// segundos que quedan de ir rápido porque se movió una ventana
    se_movio: f32,
    cada_cursor: f32,
    exit: bool,
}

impl App {
    fn crea(&mut self, output: wl_output::WlOutput) {
        if !self.mascotas.is_empty() {
            return;
        }
        let Some(info) = self.output.info(&output) else {
            return;
        };
        let Some((w, h)) = info.logical_size else {
            return;
        };
        if let Some(m) = &self.monitor
            && info.name.as_deref() != Some(m.as_str())
        {
            return;
        }
        self.monitor = info.name.clone();
        self.pantalla = (w as f32, h as f32);
        let px = self.cfg.escala;
        for (i, nombre) in self.cfg.pets.clone().iter().enumerate() {
            let Some(especie) = arte::especie(nombre) else {
                continue;
            };
            let surface = self.compositor.create_surface(&self.qh);
            // vacía hasta que se pinte: luego es nomás su cuerpo
            if let Ok(region) = Region::new(&self.compositor) {
                surface.set_input_region(Some(region.wl_region()));
            }
            let layer = self.layer_shell.create_layer_surface(
                &self.qh,
                surface,
                Layer::Top,
                Some("bicho"),
                Some(&output),
            );
            layer.set_anchor(Anchor::TOP | Anchor::LEFT);
            layer.set_exclusive_zone(-1);
            // al darle clic toma el teclado: así se sabe si traes SUPER apretado
            layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
            layer.set_size(arte::ANCHO as u32 * px, arte::ALTO as u32 * px);
            layer.commit();
            let semilla = self
                .semilla
                .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                .wrapping_add(i as u64 * 0x2545_f491);
            let mut b = bicho::Bicho::new(
                px as f32,
                (arte::ANCHO as u32 * px) as f32,
                (arte::ALTO as u32 * px) as f32,
                semilla,
            );
            b.velocidad = self.cfg.velocidad;
            b.trepa = self.cfg.trepa;
            b.salta = especie.brinca;
            b.aparece(self.pantalla.0);
            self.mascotas.push(Mascota {
                especie,
                layer,
                pool: None,
                bicho: b,
                colores: colores(especie),
                margen: (i32::MIN, i32::MIN),
                region: None,
                cuerpo: cuerpo(especie, px as f32),
                pintado: None,
            });
        }
    }

    fn tick(&mut self) {
        let dt = self.ultimo.elapsed().as_secs_f32();
        self.ultimo = Instant::now();
        let Some(monitor) = self.monitor.clone() else {
            return;
        };
        if !self.configurado {
            return;
        }
        self.cada_mundo -= dt;
        self.cada_resto -= dt;
        let mut noche = None;
        // pegados a una ventana: en cada cuadro, para que se vayan con ella
        let pegados = self.mascotas.iter().any(|p| p.bicho.pegado());
        if self.cada_mundo <= 0.0 || pegados {
            self.cada_mundo = 0.4;
            self.pon_mundo(hypr::mundo(&monitor, self.borde));
        }
        self.suaviza(dt);
        self.se_movio = (self.se_movio - dt).max(0.0);
        if self.cada_resto <= 0.0 {
            self.cada_resto = 0.4;
            noche = Some(es_de_noche());
            let m = mtime(&colores_path());
            if m != self.colores_mtime {
                self.colores_mtime = m;
                for p in &mut self.mascotas {
                    p.colores = colores(p.especie);
                }
            }
        }
        self.cada_cursor -= dt;
        if self.cada_cursor <= 0.0 {
            self.cada_cursor = 0.1;
            self.cursor = hypr::cursor(self.mundo.origen);
        }
        let musica = self.cfg.baila && self.musica.load(Ordering::Relaxed);
        let (w, h) = self.pantalla;
        self.letras();
        self.tick_teclado(dt);
        self.tick_claude(dt);
        let mut looks = Vec::with_capacity(self.mascotas.len());
        for p in &mut self.mascotas {
            p.bicho.musica = musica;
            if let Some(n) = noche {
                p.bicho.noche = n;
            }
            looks.push(p.bicho.tick(dt, &self.mundo, w, h, self.cursor));
        }
        // si dos se topan en el mismo lugar, se saludan
        for i in 0..self.mascotas.len() {
            for j in i + 1..self.mascotas.len() {
                let (a, b) = (&self.mascotas[i].bicho, &self.mascotas[j].bicho);
                let (Some((xa, sa)), Some((xb, sb))) = (a.saludable(), b.saludable()) else {
                    continue;
                };
                if sa == sb && (xa - xb).abs() < a.cw * 0.7 {
                    self.mascotas[i].bicho.cariño(xb > xa);
                    self.mascotas[j].bicho.cariño(xa > xb);
                    looks[i] = self.mascotas[i].bicho.tick(0.0, &self.mundo, w, h, None);
                    looks[j] = self.mascotas[j].bicho.tick(0.0, &self.mundo, w, h, None);
                }
            }
        }
        for (i, look) in looks.iter().enumerate() {
            let g = arte::grid(self.mascotas[i].especie, look);
            self.pinta(i, &g);
        }
        self.tick_dialogo(dt);
    }

    fn pinta(&mut self, i: usize, g: &arte::Grid) {
        let px = (self.cfg.escala as i32 * self.escala_buffer) as usize;
        let (w, h) = ((arte::ANCHO * px) as i32, (arte::ALTO * px) as i32);
        let shm = &self.shm;
        let celda = self.cfg.escala as i32;
        let p = &mut self.mascotas[i];
        // su ventana está en otro escritorio: nada (ni agarra clics)
        let vacio = [[0; arte::ANCHO]; arte::ALTO];
        let g = if p.bicho.oculto { &vacio } else { g };
        // solo el cuerpo agarra clics (no el corazón ni los zzz)
        let caja = arte::caja(g).map(|(x0, y0, x1, y1)| {
            (
                x0 as i32 * celda,
                y0 as i32 * celda,
                (x1 - x0 + 1) as i32 * celda,
                (y1 - y0 + 1) as i32 * celda,
            )
        });
        if caja != p.region {
            if let Ok(region) = Region::new(&self.compositor) {
                if let Some((x, y, w, h)) = caja {
                    region.add(x, y, w, h);
                }
                p.layer
                    .wl_surface()
                    .set_input_region(Some(region.wl_region()));
            }
            p.region = caja;
        }
        let margen = (
            (p.bicho.y - p.bicho.dy).round() as i32,
            p.bicho.x.round() as i32,
        );
        // el mismo dibujo de antes: nomás se mueve la capa (si se movió)
        let igual = p
            .pintado
            .as_ref()
            .is_some_and(|(gg, c, e)| gg == g && c == &p.colores && *e == self.escala_buffer);
        if igual && p.pool.is_some() {
            if margen != p.margen {
                p.layer.set_margin(margen.0, 0, 0, margen.1);
                p.margen = margen;
                p.layer.commit();
            }
            return;
        }
        let mut pool = match p.pool.take() {
            Some(pool) => pool,
            None => match SlotPool::new((w * h * 4) as usize, shm) {
                Ok(pool) => pool,
                Err(_) => return,
            },
        };
        if let Ok((buffer, canvas)) = pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) {
            arte::pinta(g, &p.colores, px, canvas);
            p.pintado = Some((*g, p.colores, self.escala_buffer));
            if margen != p.margen {
                p.layer.set_margin(margen.0, 0, 0, margen.1);
                p.margen = margen;
            }
            let surface = p.layer.wl_surface();
            surface.set_buffer_scale(self.escala_buffer);
            surface.damage_buffer(0, 0, w, h);
            if buffer.attach_to(surface).is_ok() {
                p.layer.commit();
            }
        }
        p.pool = Some(pool);
    }
}

fn run(pets: &[String]) -> Result<(), String> {
    std::panic::set_hook(Box::new(|info| bitacora(&format!("tronó: {info}"))));
    // nunca dos a la vez
    if let Some(otro) = running()
        && otro != std::process::id() as i32
    {
        return Err(format!("ya anda otro (pid {otro})"));
    }
    let mut cfg = config();
    if !pets.is_empty() {
        cfg.pets = pets.to_vec();
    }
    std::fs::write(pidfile(), std::process::id().to_string()).map_err(|e| e.to_string())?;
    let _ = std::fs::write(petsfile(), cfg.pets.join(" "));
    let r = vive(cfg);
    let _ = std::fs::remove_file(pidfile());
    r
}

fn vive(cfg: Config) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("Wayland: {e}"))?;
    let (globals, queue) = registry_queue_init(&conn).map_err(|e| format!("Wayland: {e}"))?;
    let qh = queue.handle();
    let mut event_loop: EventLoop<App> = EventLoop::try_new().map_err(|e| e.to_string())?;
    let handle = event_loop.handle();
    WaylandSource::new(conn.clone(), queue)
        .insert(handle.clone())
        .map_err(|e| e.to_string())?;
    handle
        .insert_source(Timer::from_duration(CUADRO), |_, _, app: &mut App| {
            app.tick();
            TimeoutAction::ToDuration(if app.rapido() { CUADRO_RAPIDO } else { CUADRO })
        })
        .map_err(|e| e.to_string())?;
    let musica = Arc::new(AtomicBool::new(false));
    if cfg.baila {
        oye_musica(musica.clone());
    }

    let mut app = App {
        registry: RegistryState::new(&globals),
        output: OutputState::new(&globals, &qh),
        compositor: CompositorState::bind(&globals, &qh)
            .map_err(|_| "el compositor no tiene wl_compositor")?,
        layer_shell: LayerShell::bind(&globals, &qh)
            .map_err(|_| "el compositor no tiene layer-shell")?,
        seat: SeatState::new(&globals, &qh),
        pointer: None,
        arrastre: None,
        dialogo: None,
        teclado: None,
        super_apretado: false,
        foco_vuelve: None,
        cada_claude: 0.0,
        claude_antes: Default::default(),
        claude_ignorada: None,
        texto: texto::arranca(),
        shm: Shm::bind(&globals, &qh).map_err(|_| "el compositor no tiene wl_shm")?,
        qh,
        cfg,
        monitor: hypr::monitor_enfocado(),
        mascotas: Vec::new(),
        pantalla: (0.0, 0.0),
        configurado: false,
        escala_buffer: 1,
        semilla: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(7, |d| d.as_nanos() as u64),
        mundo: hypr::Mundo::default(),
        borde: hypr::borde(),
        cursor: None,
        colores_mtime: mtime(&colores_path()),
        musica,
        ultimo: Instant::now(),
        cada_mundo: 0.0,
        cada_resto: 0.0,
        reales: Vec::new(),
        deslizando: Default::default(),
        se_movio: 0.0,
        cada_cursor: 0.0,
        exit: false,
    };
    while !app.exit {
        event_loop
            .dispatch(Some(Duration::from_millis(100)), &mut app)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        factor: i32,
    ) {
        if factor.max(1) != self.escala_buffer {
            self.escala_buffer = factor.max(1);
            for p in &mut self.mascotas {
                p.pool = None;
            }
        }
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.crea(output);
    }
    fn update_output(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output.info(&output)
            && info.name == self.monitor
            && let Some((w, h)) = info.logical_size
        {
            self.pantalla = (w as f32, h as f32);
        }
        self.crea(output);
    }
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if self.dialogo.as_ref().is_some_and(|d| &d.layer == layer) {
            self.cierra_dialogo();
        } else {
            self.exit = true;
        }
    }
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        _: LayerSurfaceConfigure,
        _: u32,
    ) {
        match self.dialogo.as_mut() {
            Some(d) if &d.layer == layer => d.configurado = true,
            _ => self.configurado = true,
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_registry!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_dispatch2!(App);
