# Protocolo de streams

WebSocket en `ws://127.0.0.1:7777`. Cada mensaje de texto es **un sobre** o **una lista de sobres**.

```jsonc
{
  "s": "face",          // stream: face | body | morph | voice
  "seq": 12,            // opcional. Por stream: si es <= al último visto, se descarta. 0/ausente = sin control
  "ttl": 800,           // opcional, ms. Al vencer, los valores vuelven a su base. Ausente = se quedan
  "set": { ... }        // solo lo que cambia
}
```

- Los arrays se expanden a ejes: `"gaze": [0.3, -0.1]` = `gaze.x`, `gaze.y`. `"pos": [x, y]` igual.
- Los booleanos valen 0/1: `"spikes": false`.
- Una clave desconocida, o que pertenece a otro stream, se ignora y se avisa **una vez** en la consola
  del cuerpo, junto con la lista de claves válidas.
- Presets y valores crudos pueden ir juntos: los presets se aplican primero y los crudos los corrigen.

## `face`

| Clave | Rango | Base | Qué hace |
|-------|-------|------|----------|
| `expr` | `neutral` `focus` `happy` `angry` `sad` `surprised` `sleep` | — | preset de expresión |
| `eye.l.open`, `eye.r.open` | 0..1 | 1 | apertura de la almendra de cada ojo: 0 = raya horizontal sin círculos visibles (se combina con el parpadeo automático) |
| `eye.l.size`, `eye.r.size` | ~0.5..1.5 | 1 | tamaño de cada ojo (disco e iris; la silueta crece la mitad) |
| `gaze.x`, `gaze.y` (o `gaze: [x,y]`) | -1..1 | 0 | mirada: mueve juntos anillo, iris y centro; longitud 1 alcanza el límite donde el círculo central aún cabe entero en la almendra abierta |
| `mask.glow` | 0..1 | 0.6 | brillo de la placa |
| `mouth.open` | 0..1 | 0 | apertura de la boca (aún no se dibuja) |
| `mouth.curve` | -1..1 | 0 | triste … sonrisa (aún no se dibuja) |

## `body`

Ángulos en grados. Brazos y piernas: 0 = colgando; positivo = giro antihorario en pantalla.

| Clave | Base | Qué hace |
|-------|------|----------|
| `pose` | — | preset: `idle` `arms_up` `wave` `guard` `lean_left` `lean_right` |
| `neck` | 0 | inclinación de la cabeza |
| `torso.lean` | 0 | inclinación del torso |
| `arm.l`, `arm.r` | 0 | brazos (`arm.r: 150` = brazo derecho arriba) |
| `leg.l`, `leg.r` | 0 | piernas |
| `pos.x`, `pos.y` (o `pos: [x,y]`) | 0 | posición en la ventana, -1..1 |
| `rot` | 0 | rotación global |
| `yaw` | 0 | giro sobre el eje vertical (sensación de volumen) |
| `scale` | 1 | escala global |

## `morph`

| Clave | Base | Qué hace |
|-------|------|----------|
| `core` | — | preset de núcleo: `full` `eyes` `bolt` |
| `core.full`, `core.eyes`, `core.bolt` | 1, 0, 0 | pesos crudos de cada núcleo (se mezclan) |
| `silhouette` | 1 | opacidad del cuerpo (la cara queda) |
| `opacity` | 1 | opacidad de todo |
| `spikes`, `spikes.len` | 1, 1 | presencia y largo de las púas |
| `fragments`, `fragments.spread` | 0, 1 | fragmentos en órbita y su dispersión |

## `voice`

| Clave | Rango | Qué hace |
|-------|-------|----------|
| `level` | 0..1 | nivel de voz: se suma a `mouth.open` y aviva la placa (con `glow` > 0 en el estilo). Mandar con `ttl` corto (~120 ms) |

Reservado: `viseme`.

## Ejemplos

```jsonc
// sorpresa que dura 1.5 s y vuelve sola
{"s":"face","ttl":1500,"set":{"expr":"surprised"}}
// solo ojos (con su placa)
{"s":"morph","set":{"core":"eyes"}}
// varios streams en un frame
[{"s":"body","set":{"pose":"wave","yaw":30}},{"s":"face","set":{"expr":"happy"}}]
```
