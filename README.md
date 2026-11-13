# bicho

Los animalitos de discada: pixeles que se pasean por el escritorio. Caminan
por el piso, trepan por los lados de las ventanas, se pasean por arriba de
ellas y se avientan por la orilla. Se sientan, se duermen (más de noche),
bailan si suena música, a veces persiguen al cursor, y si lo dejas quieto
cerca brincan con un corazón. Si dos se topan, se saludan. Toman los colores
del look (wallust) y cambian solos con él.

- **bicho**: el original, con antenas
- **michi**: un gato que menea la cola
- **baba**: anda a brincos y es blandita (se aplasta, se estira, se embarra
  al caer y tiembla como gelatina)
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

## Agarrarlos

Arrástralos y suéltalos: salen volando con la velocidad con que los
aventaste y rebotan en las orillas y en el piso. Con un clic sin arrastrar
les das cariño.

## En las ventanas

Si lo sueltas despacito encima de una ventana con **SUPER** apretado, se mete
y ahí vive (sin SUPER nomás cae): camina por el fondo de la ventana, se va
con ella si la mueves, se esconde si cambias de escritorio y vuelve contigo,
y si la cierras cae al piso. Para sacarlo, agárralo. Solitos también se meten
de vez en cuando (desde arriba de la ventana se clavan para adentro, o desde
el piso a la que les quede enfrente) y al rato se salen por abajo.

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
  otro escritorio, se sale y cae donde estás tú

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
`b` cuerpo, `h` brillo, `w` ojo, `p` pupila, `n` cachete; renglones de 16.

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

## Por dentro

Son capas de layer-shell (`Layer::Top`) cuya región de input es nomás el
cuerpo del animalito: el resto deja pasar el mouse. En discada la capa
`bicho` va sin animación de layer (`no_anim`); si no, se movería a jalones.
Las ventanas, el cursor y las barras los lee del socket de Hyprland; sin
Hyprland solo hay piso. Si algo truena queda en
`~/.local/state/bicho/bicho.log`.
