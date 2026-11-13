# bicho

Los animalitos de discada: pixeles que se pasean por el escritorio. Caminan
por el piso, trepan por los lados de las ventanas, se pasean por arriba de
ellas y se avientan por la orilla. Se sientan, se duermen (más de noche),
bailan si suena música, a veces persiguen al cursor, y si se lo acercas
brincan con un corazón. Si dos se topan, se saludan. Toman los colores del
look (wallust) y cambian solos con él.

Los puedes agarrar con el mouse: arrástralos y suéltalos, y salen volando
con la velocidad con que los aventaste, rebotan en las orillas y en el piso.
Con un clic sin arrastrar les das cariño.

Si lo sueltas despacito (sin aventarlo) encima de una ventana con SUPER
apretado, se mete y ahí vive (sin SUPER nomás cae): camina por el fondo de la ventana, se va con ella si la mueves, se
esconde si cambias de escritorio y vuelve contigo, y si la cierras cae al
piso. Para sacarlo, agárralo. Solitos también se meten de vez en cuando (desde
arriba de la ventana se clavan para adentro, o desde el piso a la que les
quede enfrente) y al rato se salen por abajo. En una terminal las letras son
repisas: brincan de línea en línea, pero solo se paran donde caben (arriba de
las letras tiene que haber hueco del alto de su cuerpo). Las ve con `grim`,
unas fotos por segundo de lo que tiene alrededor, y solo mientras alguno ande
en una terminal.

- **bicho**: el original, con antenas
- **michi**: un gato que menea la cola
- **claudio**: el bichito de Claude, cuadradito y terracota, sin contorno, con bracitos,
  cachetes y un foquito de antena que se mece al caminar. Lo hizo Claude,
  para acordarse. Si le das
  clic se queda quieto y le sale un globo de diálogo, como en los juegos:
  "¿ocupas ayuda?" ▶ sí / no. Con un sí abre `claude` en una terminal
  (`$TERMINAL`, o kitty); si anda metido en una ventana, en la carpeta de
  esa ventana (la de la terminal, la de nvim, o la que tengas abierta en taco)
- **baba**: anda a brincos y es blandita (se aplasta, se estira, se embarra
  al caer y tiembla como gelatina)

```
bicho [especie…]     los suelta (sin nombre, los de la config); si ya andan,
                     los cambia por esos. Se pueden repetir: bicho michi michi
bicho más ESPECIE    suelta uno más
bicho off            los guarda
bicho toggle         una cosa o la otra
bicho menu           el menú (jale): SUPER+Shift+B en discada
bicho ver            cómo se ven, en la terminal
bicho --auto         al iniciar sesión: los suelta si el menú dice que sí
```

`~/.config/bicho/config.json` (el menú lo escribe):

```json
{"pets": ["bicho", "michi", "baba"], "scale": 3, "speed": 1.0,
 "climb": true, "dance": true, "login": false}
```

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

Son capas de layer-shell (`Layer::Top`) cuya región de input es nomás el
cuerpo del animalito: el resto deja pasar el mouse. En discada la capa
`bicho` va sin animación de layer (`no_anim`), si no se movería a jalones. Las ventanas
y el cursor los lee del socket de Hyprland; sin Hyprland solo hay piso. Si
algo truena queda en `~/.local/state/bicho/bicho.log`.
