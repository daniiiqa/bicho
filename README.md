# bicho

Los animalitos de discada: pixeles que se pasean por el escritorio. Caminan
por el piso, trepan por los lados de las ventanas, se pasean por arriba de
ellas y se avientan por la orilla. Se sientan, se duermen (más de noche),
bailan si suena música, a veces persiguen al cursor, y si se lo acercas
brincan con un corazón. Si dos se topan, se saludan. Toman los colores del
look (wallust) y cambian solos con él.

- **bicho**: el original, con antenas
- **michi**: un gato que menea la cola
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

No agarran clics: son capas de layer-shell (`Layer::Top`) con la región de
input vacía, así que tampoco tapan las apps en pantalla completa. Las ventanas
y el cursor los lee del socket de Hyprland; sin Hyprland solo hay piso. Si
algo truena queda en `~/.local/state/bicho/bicho.log`.
