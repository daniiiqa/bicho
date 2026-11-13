//! `bicho shell instala|quita`: una línea en ~/.bashrc para que los
//! animalitos sepan cuándo truena un comando en su terminal. Antes de cada
//! prompt ve `$?` y, solo si falló (no Ctrl+C ni Ctrl+Z), escribe
//! `$XDG_RUNTIME_DIR/bicho/truena.<pid del shell>` con el código y la hora.
//! Puro bash, sin lanzar nada.

/// Lo que va en el .bashrc (la marca del final es para encontrarla).
const LINEA: &str = r#"__bicho() { local rc=$?; ((rc && rc != 130 && rc != 148)) && { printf '%s %(%s)T\n' "$rc" -1 >"${XDG_RUNTIME_DIR:-/tmp}/bicho/truena.$$"; } 2>/dev/null; return $rc; }; PROMPT_COMMAND="__bicho${PROMPT_COMMAND:+;$PROMPT_COMMAND}"  # bicho"#;
const MARCA: &str = "# bicho";

/// El .bashrc con la línea puesta o quitada (None: ya estaba así).
fn arregla(bashrc: &str, poner: bool) -> Option<String> {
    let tiene = bashrc.lines().any(|l| l.trim_end().ends_with(MARCA));
    match (poner, tiene) {
        (true, false) => {
            let mut s = bashrc.to_string();
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s += "\n# los animalitos se asustan si truena un comando (bicho shell quita)\n";
            s += LINEA;
            s.push('\n');
            Some(s)
        }
        (false, true) => {
            let s: Vec<&str> = bashrc
                .lines()
                .filter(|l| !l.trim_end().ends_with(MARCA) && !l.contains("(bicho shell quita)"))
                .collect();
            Some(s.join("\n").trim_end().to_string() + "\n")
        }
        _ => None,
    }
}

pub fn instala(poner: bool) -> i32 {
    let archivo = super::home(".bashrc");
    let texto = std::fs::read_to_string(&archivo).unwrap_or_default();
    match arregla(&texto, poner) {
        None => println!(
            "{}",
            if poner {
                "la línea de bicho ya estaba en ~/.bashrc"
            } else {
                "no había línea de bicho en ~/.bashrc"
            }
        ),
        Some(nuevo) => {
            if let Err(e) = std::fs::write(&archivo, nuevo) {
                eprintln!("bicho: no pude escribir {}: {e}", archivo.display());
                return 1;
            }
            println!(
                "{} ~/.bashrc (en las terminales nuevas)",
                if poner { "listo:" } else { "quitada de" }
            );
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pone_una_vez_y_quita_sin_tocar_lo_demas() {
        let orig = "alias ls='ls --color=auto'\nfastfetch";
        let puesto = arregla(orig, true).unwrap();
        assert!(puesto.starts_with(orig) && puesto.contains(LINEA));
        assert!(arregla(&puesto, true).is_none(), "no la duplica");
        assert_eq!(arregla(&puesto, false).unwrap(), format!("{orig}\n"));
        assert!(arregla(orig, false).is_none());
    }
}
