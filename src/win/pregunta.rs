//! Una cajita para escribir algo (el nombre de un animalito): un texto, el
//! renglón para escribir y Aceptar / Cancelar. Windows no trae una lista,
//! así que se arma con una ventana y sus controles; Enter acepta y Esc
//! cancela (IsDialogMessage).

use super::w;
use std::cell::RefCell;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HFONT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASE: &str = "bicho-pregunta";
const ID_TEXTO: i32 = 100;

thread_local! {
    /// lo que contestaste: None mientras sigue abierta
    static RESPUESTA: RefCell<Option<Option<String>>> = const { RefCell::new(None) };
}

unsafe extern "system" fn ventana(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wp & 0xffff) as i32;
            if id == IDOK || id == IDCANCEL {
                let texto = (id == IDOK).then(|| unsafe {
                    let edit = GetDlgItem(h, ID_TEXTO);
                    let mut buf = [0u16; 64];
                    let n = GetWindowTextW(edit, buf.as_mut_ptr(), buf.len() as i32);
                    String::from_utf16_lossy(&buf[..n.max(0) as usize])
                });
                RESPUESTA.with(|r| *r.borrow_mut() = Some(texto));
                unsafe { DestroyWindow(h) };
            }
            0
        }
        WM_CLOSE => {
            RESPUESTA.with(|r| *r.borrow_mut() = Some(None));
            unsafe { DestroyWindow(h) };
            0
        }
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

/// La letra de los mensajes de Windows (Segoe UI), del tamaño de la escala.
fn letra(escala: f32) -> HFONT {
    unsafe {
        let mut m: NONCLIENTMETRICSW = std::mem::zeroed();
        m.cbSize = size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            m.cbSize,
            (&mut m as *mut NONCLIENTMETRICSW).cast(),
            0,
        );
        let mut lf = m.lfMessageFont;
        lf.lfHeight = (lf.lfHeight as f32 * escala).round() as i32;
        CreateFontIndirectW(&lf)
    }
}

/// Pregunta y espera: Some(lo escrito) con Aceptar o Enter; None con
/// Cancelar, Esc o la tachita. `escala`: la de la pantalla (1.25 al 125 %).
pub(crate) fn pregunta(
    titulo: &str,
    mensaje: &str,
    actual: &str,
    centro: (i32, i32),
    escala: f32,
) -> Option<String> {
    unsafe {
        let hinst = super::instancia();
        let clase = w(CLASE);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(ventana),
            hInstance: hinst,
            lpszClassName: clase.as_ptr(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            // COLOR_BTNFACE + 1: el gris de los diálogos
            hbrBackground: (15 + 1) as usize as _,
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let e = |v: i32| (v as f32 * escala).round() as i32;
        let (ancho, alto) = (e(360), e(150));
        let t = w(titulo);
        let h = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_DLGMODALFRAME,
            clase.as_ptr(),
            t.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            centro.0 - ancho / 2,
            centro.1 - alto / 2,
            ancho,
            alto,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if h.is_null() {
            return None;
        }
        let fuente = letra(escala);
        let hijo = |clase: &str, texto: &str, estilo: u32, x, y, cw, ch, id: i32| {
            let (c, t) = (w(clase), w(texto));
            let v = CreateWindowExW(
                if clase == "EDIT" { WS_EX_CLIENTEDGE } else { 0 },
                c.as_ptr(),
                t.as_ptr(),
                WS_CHILD | WS_VISIBLE | estilo,
                e(x),
                e(y),
                e(cw),
                e(ch),
                h,
                id as usize as HMENU,
                hinst,
                std::ptr::null(),
            );
            SendMessageW(v, WM_SETFONT, fuente as usize, 1);
            v
        };
        let es = super::bandeja::es();
        hijo("STATIC", mensaje, 0, 14, 12, 320, 20, -1);
        let edit = hijo(
            "EDIT",
            actual,
            WS_TABSTOP | ES_AUTOHSCROLL as u32,
            14,
            38,
            320,
            26,
            ID_TEXTO,
        );
        hijo(
            "BUTTON",
            if es { "Aceptar" } else { "OK" },
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
            150,
            76,
            88,
            28,
            IDOK,
        );
        hijo(
            "BUTTON",
            if es { "Cancelar" } else { "Cancel" },
            WS_TABSTOP | BS_PUSHBUTTON as u32,
            246,
            76,
            88,
            28,
            IDCANCEL,
        );
        ShowWindow(h, SW_SHOW);
        SetForegroundWindow(h);
        SetFocus(edit);
        // todo seleccionado: escribes encima
        // EM_SETSEL
        SendMessageW(edit, 0x00B1, 0, -1isize as LPARAM);
        RESPUESTA.with(|r| *r.borrow_mut() = None);
        let mut msg: MSG = std::mem::zeroed();
        // su propio ciclo (el reloj de los animalitos también pasa por aquí)
        while RESPUESTA.with(|r| r.borrow().is_none()) {
            if GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) <= 0 {
                // se está cerrando todo: que lo vuelva a ver el ciclo de afuera
                PostQuitMessage(0);
                break;
            }
            if IsDialogMessageW(h, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        DeleteObject(fuente);
        RESPUESTA.with(|r| r.borrow_mut().take()).flatten()
    }
}
