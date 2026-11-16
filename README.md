# bicho

Los animalitos de discada: pixeles que se pasean por el escritorio. Caminan
por el piso, trepan por los lados de las ventanas, se pasean por arriba de
ellas y se avientan por la orilla. Se sientan, se duermen (más de noche),
bailan si suena música, a veces persiguen al cursor, y si lo dejas quieto
cerca brincan con un corazón. Si dos se topan, se saludan, y a veces juegan a la
correteada. Toman los colores del look (wallust) y cambian solos con él.

- **bicho**: el original, con antenas
- **michi**: un gato que menea la cola
- **baba**: anda a brincos y es blandita (se aplasta, se estira, se embarra
  al caer y tiembla como gelatina)
- **bashira**: una husky dorada de orejas paradas, máscara y pecho blancos y
  la cola enroscada, que menea al caminar
- **claudio**: el bichito de Claude, cuadradito y terracota, sin contorno,
  con bracitos, cachetes y un foquito de antena que se mece al caminar. Lo
  hizo Claude, para acordarse ([abajo](#claudio-y-claude-code) lo que sabe
  hacer)

```
bicho [especie…]     los suelta (sin nombre, los de la config); si ya andan,
                     los cambia por esos. Se pueden repetir: bicho michi michi
bicho más ESPECIE    suelta uno más
bicho off            los guarda
bicho toggle         una cosa o la otra
bicho menu           el menú (jale): SUPER+Shift+B en discada
bicho ver            cómo se ven, en la terminal
bicho --auto         al iniciar sesión: los suelta si el menú dice que sí
bicho claude instala los hooks de Claude Code, para claudio (quita: los quita)
bicho shell instala  una línea en ~/.bashrc para que se asusten si truena un
                     comando en su terminal (quita: la quita)
```

`~/.config/bicho/config.json` (el menú lo escribe):

```json
{"pets": ["bicho", "michi", "baba"], "scale": 3, "speed": 1.0,
 "climb": true, "dance": true, "login": false}
```

## Nombres

Cada uno puede tener nombre: en la config va como `"michi:Canela"` (o
`bicho michi:Canela baba` al soltarlos), y desde el menú («ponerles nombre»
en Linux, «Nombres» en la bandeja de Windows) se los pones o se los quitas.
El nombre sale en una etiquetita encima de él cuando le pasas el cursor, al
soltarlos y al darle cariño.

## Caga palos y contraste

**Caga palos** (`"mischief"`, en el menú; viene prendido en Windows y
apagado en Linux): persiguen más al cursor, brincan con un «!» para
llamarte la atención, a veces se te escapan si les llegas rápido con el
mouse, si uno te alcanza persiguiéndote se roba el cursor un ratito (jala
fuerte y lo suelta), y de vez en cuando uno le da de pisotones a la ventana
donde anda parado (se hunde tantito y regresa). En Linux los pisotones son
nomás en ventanas flotantes, y se roban el cursor solo si estaba quieto (en
Wayland no se sabe si traes un botón apretado); en KDE nomás se escapan.

**Contraste** (`"contrast"`, prendido): si uno se confunde con lo que tiene
atrás (uno oscuro en una ventana oscura), le sale un halo claro u oscuro. En
Linux lo de atrás se ve con grim (Hyprland).

## Entre ellos

Si dos se topan, se saludan. Y de vez en cuando, dos libres en el mismo suelo
(el piso, encima de la misma ventana o dentro de la misma) juegan a la
correteada: uno la trae y corretea al otro; cuando lo alcanza cambian (al
agarrado le sale un "!", el que lo agarró se lleva un corazón, y el nuevo
cuenta un ratito antes de salir). Unas rondas y a descansar, contentos. Si
andan michi y la baba, michi empieza correteando a la baba. No juegan
dormidos, platicando, ni claudio con su laptop.

## Varios monitores

Con varios monitores se pasan de uno a otro por las orillas que se tocan:
paseando por el piso, persiguiendo al cursor, o si los avientas para allá.
Cada uno vive en el suyo (su propia capa en esa salida, con su escala), y
si desconectas el monitor donde anda, regresa al de casa (donde los
soltaste). En Linux, con Hyprland (en KDE se quedan en el suyo); en
Windows, siempre.

## Agarrarlos

Arrástralos y suéltalos: salen volando con la velocidad con que los
aventaste y rebotan en las orillas y en el piso. Con un clic sin arrastrar
les das cariño.

## En las ventanas

Si lo sueltas despacito encima de una ventana con **SUPER** apretado, se mete
y ahí vive (sin SUPER nomás cae): camina por el fondo de la ventana, se va
con ella si la mueves, se esconde si cambias de escritorio y vuelve contigo,
y si la cierras cae al piso. Los que metes tú son **de casa**: no se salen
solos (solo si los sacas arrastrándolos o cierras la ventana). Para saber
cuáles son, pásales el mouse encima: a los de casa les sale una casita. Solitos también
se meten de vez en cuando (desde arriba de la ventana se clavan para adentro,
o desde el piso a la que les quede enfrente), pero esos van de paso: al rato
se salen por abajo.

En una terminal las letras son repisas: brincan de línea en línea, pero solo
se paran donde caben (arriba de las letras tiene que haber hueco del alto de
su cuerpo). Las ve con `grim`, unas fotos por segundo de lo que tienen
alrededor, y solo mientras alguno ande en una terminal.

## claudio y Claude Code

Con un clic, claudio se queda quieto y le sale un globo de diálogo, como en
los juegos: "¿ocupas ayuda?" ▶ sí / no. Con un sí abre `claude` en una
terminal (`$TERMINAL`, o kitty); si anda metido en una ventana, en la carpeta
de esa ventana (la de la terminal, la de nvim, o la que tengas abierta en
taco).

Con los hooks de Claude Code (`bicho claude instala`) sabe cómo va Claude:

- mientras trabaja, se sienta con su laptop a teclear
- cuando acaba, brinca con un corazón
- si Claude te pregunta algo (un permiso, una pregunta) y andas en otra
  ventana, le sale un globo: "¡claude te pregunta algo!" ▶ voy / luego.
  "voy" te lleva a esa ventana. Si claudio anda escondido en una ventana de
  otro escritorio, se sale y cae donde estás tú (y si es de casa, al acabar
  regresa a su ventana)

Si claudio anda metido en la terminal de una sesión, se fija en esa.
`bicho claude instala` pone esto en `~/.claude/settings.json` sin tocar lo
demás (`bicho claude quita` lo quita):

```json
"hooks": {
  "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "bicho claude trabajando 2>/dev/null || true" }] }],
  "PreToolUse":       [{ "hooks": [{ "type": "command", "command": "bicho claude trabajando 2>/dev/null || true" }] }],
  "PostToolUse":      [{ "hooks": [{ "type": "command", "command": "bicho claude trabajando 2>/dev/null || true" }] }],
  "Stop":             [{ "hooks": [{ "type": "command", "command": "bicho claude listo 2>/dev/null || true" }] }],
  "Notification":     [{ "matcher": "permission_prompt|elicitation_dialog",
                         "hooks": [{ "type": "command", "command": "bicho claude pregunta 2>/dev/null || true" }] }],
  "SessionEnd":       [{ "hooks": [{ "type": "command", "command": "bicho claude fin 2>/dev/null || true" }] }]
}
```

El estado de cada sesión queda en `$XDG_RUNTIME_DIR/bicho/claude/`.

## Reacciones

- **Cambio de look:** cuando se quita el telón de farula, brincan del susto
  con un "!".
- **chapa:** bloqueada la sesión se duermen; al volver se despiertan
  contentos, cada quien a su tiempo.
- **arre:** cuando llega una notificación (y no estás en "no molestar"), el
  que ande más a la derecha se asoma hacia allá con un "!".
- **Su terminal:** si truena un comando ahí, se asustan (con
  `bicho shell instala`: una línea en `~/.bashrc` que, solo cuando un comando
  falla, se lo avisa; no va por colores, porque con wallust el "rojo" de la
  terminal puede no ser rojo). Si algo compila o se actualiza ahí (cargo,
  make, pacman, paru, carbon…), se sientan a esperar con un relojito y
  celebran cuando acaba.

## Disfraces

Si existe `~/.config/bicho/<especie>.txt`, ese dibujo reemplaza al de la
especie (nomás en esa máquina). Las letras son las del código: `o` contorno,
`b` cuerpo, `h` brillo, `w` ojo, `p` pupila, `n` cachete, `c` crema (pelo
blanco, no parpadea); renglones de 16.

```text
// comentarios así
color = #d97757
[cuerpo]
..bbbbbbbbbbbb..
..bbpbbbbbbpbb..
bbbbbbbbbbbbbbbb
..bbbbbbbbbbbb..
[cuerpo_b]   (opcional: el otro cuadro al caminar)
[patas_a]    (opcionales; sin patas_b usa las mismas)
...b.b....b.b...
[patas_b]
```

Si el archivo está mal hecho se queda el dibujo de siempre. Lo blandito de la
baba no se disfraza.

## Fuera de discada (KDE Plasma y otros)

Funciona en cualquier compositor de Wayland con layer-shell (KDE Plasma,
Sway, Hyprland…). El binario ya compilado está en la release `edge`:

```sh
mkdir -p ~/.local/bin
curl -L https://github.com/daniiiqa/bicho/releases/download/edge/bicho-x86_64-linux -o ~/.local/bin/bicho
chmod +x ~/.local/bin/bicho
bicho baba michi claudio
```

Para actualizarlo: `bicho actualiza` (baja el compilado nuevo, revisa su
sha256 y, si andan sueltos, los vuelve a soltar con el nuevo).

Para que salgan al entrar: en KDE, *Preferencias del sistema → Inicio
automático → Añadir → Añadir programa* y pon `bicho`. Con Claude Code,
`bicho claude instala`; con bash, `bicho shell instala`.

En **KDE Plasma** bicho le carga a KWin un script chiquito (las apps de
Wayland no pueden ver las ventanas de las demás; KWin sí) que le va contando
por D-Bus dónde están las ventanas y el cursor. Con eso trepan, caminan
encima, se meten (y se quedan, si los metes tú), se van con la ventana, y el
"voy" del globo de claudio enfoca la ventana. Lo que no hay en KDE: brincar en
las letras de las terminales (KWin no deja fotografiarlas) y el menú
(`bicho menu` usa jale, de discada; la config se edita en
`~/.config/bicho/config.json`).

En otros compositores sin Hyprland ni KWin caminan por el piso (encima del
panel), se agarran, se avientan, juegan y se duermen con el bloqueo de
pantalla, y claudio abre `claude` en la terminal que haya.

Si algo no sale, `BICHO_DEBUG=1 bicho` escribe cada 2 s en
`$XDG_RUNTIME_DIR/bicho/debug.txt` lo que sabe del mundo y dónde anda cada
quien.

## En Windows

También andan en Windows 10 y 11: bájate `bicho-x86_64-windows.exe` de la
[release edge](../../releases/tag/edge), ponlo en una carpeta donde se vaya
a quedar y dale doble clic. No se instala nada. La primera vez Windows avisa
que no conoce al programa (no está firmado): «Más información» →
«Ejecutar de todas formas».

Hacen lo mismo que en Linux: caminan por arriba de la barra de tareas, trepan
y se pasean por las ventanas (solo por la orilla que se ve, si otra la
tapa), se duermen, juegan a la correteada, se saludan y bailan si suena
música (lo que salga en los controles de medios de Windows: Spotify,
YouTube…). Los agarras, los avientas y les das cariño con un clic. Para
meterlos a una ventana, suéltalos con **Ctrl** apretado: ahí viven, y si
otra ventana la tapa, los tapa también. Si algo está en pantalla completa
(un juego, un video), en esa pantalla se esconden.

Con varias pantallas se pasan de una a otra por las orillas que se tocan:
paseando, persiguiendo al cursor, o si los avientas para allá (o los llevas
arrastrando). Con **contraste**, si se confunden con lo que tienen atrás (uno
oscuro en una ventana oscura), les sale un halo claro u oscuro.

**Caga palos** (viene prendido en Windows): persiguen más al cursor, brincan
con un «!» para llamarte la atención, a veces se te escapan si les llegas
rápido con el mouse, si uno te alcanza persiguiéndote se roba el cursor un
ratito (jala fuerte y lo suelta), y de vez en cuando uno le da de pisotones
a la ventana donde anda parado (se sacude, no se mueve de lugar). Nunca
mientras traes apretado un botón del mouse, ni con ventanas maximizadas.

El menú es el ícono junto al reloj: qué animalitos andan (y uno más), el
tamaño, si trepan, si bailan, el contraste, caga palos, si arrancan con
Windows, actualizar y guardarlos. Habla español o inglés, según Windows. Lo
que cambies se guarda en `%APPDATA%\bicho\config.json` (`"dance"`,
`"contrast"`, `"mischief"`, como los demás). Ahí también van los disfraces
(`<especie>.txt`) y, si quieres otros colores, un `colors.json` con las
mismas claves que el de wallust. Si algo truena, queda en
`%LOCALAPPDATA%\bicho\bicho.log`.

**Actualizar**: cada tanto revisa solo y, si hay versión nueva, avisa con
una notificación; en el menú, «Actualizar» la baja (revisa su sha256), se
cambia y vuelve a soltar a los animalitos. O `bicho actualiza` en una
terminal.

Desde una terminal: `bicho off`, `bicho toggle`, `bicho michi baba`,
`bicho ver`, `bicho actualiza`.

Al bloquear la sesión (Win+L) se duermen y al volver se despiertan
desperezándose. Con la barra de tareas en auto-ocultar caminan arriba de
donde sale la barra (para que no los tape).

Allá no hay (todavía): repisas en las letras de las terminales, claudio con
Claude Code ni las demás reacciones a discada.

## Por dentro

Son capas de layer-shell (`Layer::Top`) cuya región de input es nomás el
cuerpo del animalito: el resto deja pasar el mouse. En discada la capa
`bicho` va sin animación de layer (`no_anim`); si no, se movería a jalones.
Las ventanas, el cursor y las barras los lee del socket de Hyprland; sin
Hyprland solo hay piso. Si algo truena queda en
`~/.local/state/bicho/bicho.log`.

En Windows (`src/win/`) cada animalito es una ventanita en capas
(`UpdateLayeredWindow`), siempre encima, sin botón en la barra; Windows deja
pasar el mouse por lo transparente. Las ventanas salen de `EnumWindows`, con
su borde real (DWM). Las de otros escritorios virtuales vienen "encubiertas"
y se brincan. El cerebro (`bicho.rs`), el dibujo (`arte.rs`) y la correteada
(`juego.rs`) son los mismos; lo que comparten del escritorio vive en
`mundo.rs`.
