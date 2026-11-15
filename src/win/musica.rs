//! ¿Está sonando algo? En Windows lo dicen los controles de medios del
//! sistema (los del recuadro que sale al subirle al volumen): Spotify,
//! YouTube en el navegador, el reproductor… Igual que playerctl en Linux:
//! un hilo que pregunta cada 2 s.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager as Controles,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Estado,
};
use windows_sys::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

fn suena_algo(c: &Controles) -> windows::core::Result<bool> {
    let sesiones = c.GetSessions()?;
    for i in 0..sesiones.Size()? {
        let s = sesiones.GetAt(i)?;
        if s.GetPlaybackInfo()?.PlaybackStatus()? == Estado::Playing {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Arranca el hilo que oye; `suena` dice si hay música.
pub(crate) fn oye(suena: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        unsafe { CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32) };
        let mut controles: Option<Controles> = None;
        loop {
            if controles.is_none() {
                controles = Controles::RequestAsync().and_then(|op| op.join()).ok();
            }
            let si = match &controles {
                Some(c) => suena_algo(c).unwrap_or_else(|_| {
                    // que lo vuelva a pedir la próxima
                    controles = None;
                    false
                }),
                None => false,
            };
            suena.store(si, Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(if controles.is_some() {
                2
            } else {
                30
            }));
        }
    });
}
