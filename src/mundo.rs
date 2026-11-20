//! Lo que los animalitos saben del escritorio, venga de donde venga
//! (Hyprland, KWin o Windows): las ventanas, las barras y las repisas.

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
    /// y arriba (la barra de arriba, empalme)
    pub arriba: f32,
    pub borde: f32,
    /// dónde empieza el monitor (para el cursor)
    pub origen: (f32, f32),
    /// todas las ventanas abiertas, en cualquier escritorio (su address):
    /// para saber si una que no se ve está en otro lado o ya la cerraron
    pub todas: Vec<String>,
    /// las que son terminales (ahí las letras son repisas)
    pub clis: Vec<String>,
    /// el pid de cada ventana (de todas): para saber en cuál anda un proceso
    pub pids: Vec<(String, i32)>,
    /// las ventanas de jardín (el de discada): el que se mete ahí se va a
    /// pasear al jardín (allá se dibuja en 3D) hasta que el jardín lo saque
    pub jardines: Vec<String>,
}

/// Donde se puede parar: `y` es la del lienzo parado ahí, y `x0..x1` las x
/// del lienzo que caben, contando desde la esquina de la ventana (así, si la
/// mueves, las repisas se van con ella).
#[derive(Clone, Debug, PartialEq)]
pub struct Repisa {
    pub y: f32,
    pub x0: f32,
    pub x1: f32,
    /// arriba hay hueco para su cuerpo (solitos solo se paran en estas; si
    /// tú lo pones en otra línea, ahí se queda)
    pub cabe: bool,
}
