# Capri — Arquitectura

Capri es un **cuerpo digital**: un renderizador de escritorio ultra ligero que no piensa nada.
Una entidad externa (LLM, constructo de personalidad, script) le manda **datos** y el cuerpo **reacciona**.
Para la entidad, el cuerpo es una subrutina: aquí dentro no hay lógica de IA.

Referencia visual: Capri (*Cyberpunk: Edgerunners 2*): figura abstracta, deliberadamente "fuera de lugar",
silueta negra delgada, pelo rojo exagerado en mechones con manchas estrelladas, placa verde con dos ojos, cuerpo
que cambia de forma. No se usan imágenes: todo es geometría procedural.

```
 ┌────────────┐   streams (no continuos)   ┌───────────────────────────────────────────┐
 │  Entidad   │ ── face  ────────────────► │                  CAPRI                    │
 │ (LLM/etc.) │ ── body  ────────────────► │  link → core (ENTITY_BODY) → render       │
 │            │ ── morph ────────────────► │        ventana flotante, transparente,    │
 │            │ ── voice ────────────────► │        siempre encima, click-through      │
 └────────────┘   WebSocket local          └───────────────────────────────────────────┘
```

---

## 1. Stack

| Pieza         | Elección                         | Por qué |
|---------------|----------------------------------|---------|
| Lenguaje      | **Rust** (edition 2024)          | Binario nativo sin runtime ni GC. Release: **3.6 MB**. |
| GPU           | **wgpu 30** + WGSL               | DX12 / Metal / Vulkan con una sola API. Todo el cuerpo vive en un shader. |
| Ventana       | **winit 0.30**                   | Transparente, sin bordes, always-on-top, click-through en Windows/macOS/Linux. |
| Transporte    | **WebSocket** en `127.0.0.1:7777` (tungstenite) | Cualquier lenguaje puede ser la entidad. |
| Mensajes      | JSON                             | Legible para un LLM y para depurar. |

### Transparencia por plataforma

| SO      | Backend | Cómo |
|---------|---------|------|
| Windows | DX12    | Swapchain `DxgiFromVisual` (DirectComposition) + ventana sin bitmap de redirección. **Verificado.** |
| macOS   | Metal   | `CompositeAlphaMode` de la superficie (PostMultiplied/PreMultiplied). Sin verificar aún. |
| Linux   | Vulkan  | Igual; necesita un compositor (X11 con compositor o Wayland). Sin verificar aún. |

El render elige el modo de alfa según lo que ofrezca la superficie y lo imprime al arrancar.
Si no hay transparencia, el fondo sale negro y se avisa. Always-on-top no existe en Wayland
(limitación del protocolo).

---

## 2. Streams

Sobre común (ver [PROTOCOL.md](PROTOCOL.md) para todas las claves):

```jsonc
{ "s": "face", "seq": 1042, "ttl": 800, "set": { "eye.l.open": 0.2, "gaze": [0.3, -0.1] } }
```

- **Datos no continuos**: cada parámetro es un *objetivo* + un resorte críticamente amortiguado.
  Un dato suelto produce un movimiento suave; una ráfaga también.
- **`ttl`**: al vencer, el parámetro vuelve a su valor base. Si la entidad se calla, el cuerpo
  no se congela: respira y parpadea solo (idle local).
- **Presets o crudo**: la entidad puede mandar un nombre probado (`expr`, `pose`, `core`) o
  valores crudos. En un mismo mensaje, los presets se aplican primero y los valores crudos los
  corrigen después: `{"core":"eyes","silhouette":0.3}` = solo ojos + silueta tenue.

| Stream  | Para qué | Dueño de |
|---------|----------|----------|
| `face`  | Control facial | `Face/*` (ojos, mirada, brillo de la placa, boca) |
| `body`  | Control corporal | `Silhouette/*` + posición / rotación / yaw / escala globales |
| `morph` | Control polimórfico, a gusto de la entidad | núcleos, opacidades, `Extensions/*` |
| `voice` | Voz | nivel de voz (se **suma** a la apertura de la boca y al brillo de la placa) y visemas |

Una clave tiene **un solo stream dueño** (hay un test que lo garantiza): dos streams nunca se pelean.

---

## 3. Núcleos

| Núcleo | Qué queda | Cómo pedirlo |
|--------|-----------|--------------|
| `full` | Cuerpo completo + cabeza (pelo, cara, ojos) | `{"s":"morph","set":{"core":"full"}}` |
| `eyes` | Solo los ojos con su placa, grandes | `{"core":"eyes"}` |
| `bolt` | Rayo en zigzag fragmentado, ojos arriba | `{"core":"bolt"}` |

Los núcleos son **pesos** (`core.full`, `core.eyes`, `core.bolt`) que se mezclan: cambiar de núcleo
es un morph, no un corte, y la entidad puede dejar el cuerpo a medio camino (`core.full: 0.5, core.bolt: 0.5`).

---

## 4. El árbol del cuerpo (`ENTITY_BODY`)

Única fuente de verdad, en `capri-core/src/body/`. Los streams escriben aquí y el render solo lee.

```
ENTITY_BODY
├── TransformController      transform global + ShapeMorph (pesos de núcleos) + Opacity
├── Face                     EyeLeft, EyeRight + placa (Mask), Mouth + cara oscura → face.wgsl
├── Silhouette               Torso, Arms, Legs (+ opacidad propia)  → silhouette.wgsl
└── Extensions               Spikes (= el pelo, hair.wgsl), Fragments (sueltas) → extensions.wgsl
```

Cada rama declara sus parámetros en un único lugar (`visit`): stream dueño + clave + parámetro.
De ahí salen el ruteo de mensajes, el tick de resortes y la lista de claves válidas en los errores.

---

## 5. Render

**Un solo draw call** a pantalla completa. Las capas están separadas en módulos WGSL que siguen
el árbol y se componen en `main.wgsl`:

```
 core.tick()      resortes, TTL, idle (respiración, parpadeo)
 Frame::build()   el árbol → un uniform de 592 bytes (incluye el polígono de la boca)
 Hair::update()   pelo procedural → segundo uniform, solo al cruzar un umbral de giro
 GPU, por píxel:
   main.wgsl        TransformController: posición, rotación, escala, yaw
   silhouette.wgsl  esqueleto (FK) + SDF de torso/brazos/piernas
   extensions.wgsl  fragmentos, rayo
   main.wgsl        mezcla de núcleos (mezcla de campos de distancia = morph orgánico)
                    volumen falso: normal del gradiente del SDF + luz que sigue al yaw
   face.wgsl        cabeza: pelo (hair.wgsl), cara oscura, ojos con su placa
                    2D nítido, pegado a la cabeza, con la boca bajo la placa
   fx.wgsl          glitch, scanlines
```

### Ojos

Los dos ojos salen de una misma función (`eye_layer` en `face.wgsl`) que trabaja en espacio de
ojo: unidad = R (radio exterior del anillo), x positiva hacia el centro de la cara; el ojo derecho
es el espejo del izquierdo. La placa es parte de los ojos: se dibuja siempre con ellos, en todos
los núcleos (no hay interruptor en `morph`). Capas, de atrás hacia delante:

```
placa 8.08 × 3.31 R (2.44:1) · centros a ±2.03 R, 0.365 R bajo el centro de la placa
silueta   almendra (dos arcos entre esquinas en punta) + púa superior (hacia el centro)
          + púa inferior (hacia fuera) + talón en la esquina interior
anillo    0.905 R .. 1 R   ┐ mínimo 1.1 px para seguir legible a tamaño normal
iris      0.585 R         │ los tres se desplazan juntos con `gaze`, siempre concéntricos
centro    0.305 R         ┘ y quedan recortados dentro de la almendra
```

- `open` (× parpadeo de idle) cierra toda la almendra y sus púas hacia una raya horizontal,
  sin curvatura. El anillo, el iris y el centro conservan su forma detrás del contorno y se
  recortan por la apertura de la almendra. Con `open = 0` solo queda la raya, sin círculos visibles.
- `size` escala disco, anillo, iris y centro; la silueta crece la mitad. Todo el ojo queda
  recortado dentro de la placa.
- `gaze` alcanza el borde de la almendra abierta cuando su longitud llega a 1. El único
  límite de recorrido es que el círculo central quepa entero (con margen de antialias);
  el anillo y el azul pueden ocultarse tras el contorno. El límite se calcula por ojo según
  su tamaño y dirección. Cerrar los párpados oculta los círculos sin desplazar la mirada.
- Las proporciones son constantes al principio de `face.wgsl`, ajustadas contra la imagen de
  referencia. Los colores vienen del estilo (`mask`, `eye_shell`, `eye_ring`, `eyes`, `pupil`).
- Halo y brillo propios de la cara escalan con `style.glow`: con `glow: 0` los colores salen planos.

### Cabeza: pelo y cara

La cabeza tiene cuatro partes: ojos (arriba), pelo, cara y boca. De atrás hacia
delante: pelo, cara oscura (la base), placa, boca, ojos.

- **Boca** (`src/mouth.rs` + `sd_mouth` en `face.wgsl`): un polígono blanco de aristas rectas bajo
  la placa, con las 8 formas de la hoja de visemas de referencia (X, MBP, A, E, I, O, U, F/L).
  Todas comparten topología (borde superior e inferior con el mismo número de vértices), así la
  mezcla de pesos `viseme.*` es una media de vértices y pasar de un visema a otro es un morph.
  Lo que no cubren los visemas es la boca de reposo: X, que `mouth.open` abre hacia la A.
  `mouth.curve` dobla la forma con una parábola (comisuras arriba o abajo) y la ensancha o
  estrecha; pesa entero en X, MBP y A. La CPU manda el polígono ya mezclado; el shader solo lo
  dibuja sobre la esfera de la cabeza, en todos los núcleos.

- **Pelo** (`src/hair.rs` + `hair.wgsl`): una **plantilla** de ~75 triángulos planos medida sobre
  las referencias del personaje (render low-poly de frente; coincide con su silueta en un 94 %).
  Aristas rectas, puntas afiladas, sin curvas. Unidades R con la placa en (0, 0):
  - **Masa**: arco de ~32 R de ancho sobre la placa, con dos lóbulos que bajan junto a la cara y
    abrazan la placa casi sin margen. En total ~41 R de ancho (5 placas) y ~14 R sobre la placa.
  - **Corona**: cuatro aletas de tiburón con las puntas a la misma altura (borde superior plano),
    inclinadas hacia fuera. Nada apunta en vertical hacia arriba: el contorno es una pajarita, más
    largo en las diagonales.
  - **Púas**: ~55 % hojas finas (vértice ~33°), ~25 % aletas o muñones anchos, ~20 % agujas.
    Algunas con detalle: colmillo que vuelve hacia la raíz, bandera (punta doblada) o escalón.
  - **Capa delantera** (flequillo y mechones, dibujada encima de la placa): todo barrido hacia
    abajo a la izquierda (~-130°). Una punta cae sobre el anillo del ojo izquierdo y otra entre
    los ojos; los ojos y la parte de abajo de la placa quedan siempre libres.
  - **Tramos de giro**: yaw cada 20°, pitch y roll cada 15° (el central es |ángulo| < paso / 2).
    Solo se regenera, y se sube a la GPU, cuando un eje cruza un umbral, con 1.5° de histéresis.
    Cada tramo mueve un poco los vértices al azar (0.15 R los bordes del hueco y las puntas del
    flequillo, 0.4 R la masa, 0.7 R las púas; los vértices compartidos se mueven juntos). El
    tramo central es la plantilla exacta y el mismo tramo da siempre el mismo pelo.
  - **Detrás de la cara siempre hay pelo**: una tapa cierra el hueco de la cara en la masa (va
    con ella) y un relleno en el plano de la placa va pegado a la cara. Así, gire como gire, no
    queda espacio entre la placa y la masa. Se dibujan antes que la masa para no tapar sus facetas.
  - **Giro**: la masa vive detrás de la cabeza, así se corre al lado contrario de la cara y se
    estrecha un poco de perfil; de espalda se ve en espejo. El flequillo va pegado a la cara y
    solo se dibuja delante mientras la cara mira al frente. En cuanto deja de mirar al frente se
    añade la nuca, un disco que tapa toda la cabeza.
  - En el shader cada capa es la unión de sus triángulos (sin costuras entre vecinos) y el tono es
    el del último triángulo que cubre el píxel, así se ven las facetas.
  - `spikes` y `spikes.len` lo hacen crecer.
- **Manchas** (`hair_glint`): puntos sobre el pelo, más densos y grandes cerca de un foco. En el
  tramo central el foco está en el centro del pelo; en los demás, hacia un punto cardinal del
  plano del pelo, casi siempre arriba a la derecha y nunca por debajo.
- **Cara** (`face`): la base de la placa. Contorno normal: frente y sienes rectas, mandíbula
  redondeada hasta la barbilla. Va en la misma esfera que la placa y la contiene con margen, así
  al girar la placa nunca se sale de la cara. Pesa con el núcleo `full`; en `eyes` y `bolt` no se
  dibuja.
- **Pelo delante de la cara al girar**: el pelo envuelve la cabeza como una cáscara mayor (8 R).
  Donde el punto visible de esa cáscara cae a más de ~68° del frente de la cara, el pelo se pinta
  encima de la cara; pasado el perfil termina de cerrarla. De frente nunca la tapa, y de espalda
  no asoman tiras de cara por el borde de la cabeza.

Partir esto en passes reales (silueta a media resolución, etc.) es posible sin tocar el árbol.
Hoy no hace falta.

**Ritmo**: 60 fps mientras algo se mueve, 30 fps en reposo.
Medido en Windows (RTX 3090, release, reposo): **~0.3 % de CPU de la máquina, 3.6 MB de binario**.
La RAM (~165 MB de working set) es casi toda del driver DX12. Bajarla queda pendiente
(probar Vulkan/GL, medir private bytes).

### Estilos gráficos

La forma (árbol) y el look están separados. El look es un preset JSON en `assets/styles/`
(paleta, contorno, glow, scanlines, glitch) que **se recarga en caliente** al guardar el archivo.

| Clave de `palette` | Qué pinta | Si falta |
|--------------------|-----------|----------|
| `body`, `rim` | silueta, contorno | valores de `construct` |
| `hair` | base del pelo | `#ef003d` |
| `hair_glint` | manchas del pelo | `#edc4ea` |
| `face` | superficie oscura de la cara | `#070211` |
| `mask` | placa de los ojos | `#11e965` |
| `eyes` | iris (y borde del rayo) | `#7fe8ff` |
| `eye_shell` | silueta y disco de cada ojo | = `body` |
| `eye_ring` | anillo de cada ojo | = `mask` |
| `pupil` | centro de cada ojo | `#9a9a9a` |

`capri-eyes.json` reproduce la referencia de los ojos con colores planos y efectos apagados.
Futuro: mods de render = passes WGSL extra que solo leen el frame y su preset, nunca el árbol.

---

## 6. Estructura del repo

```
capri/
├─ crates/
│  ├─ capri-protocol/   sobres de los streams, aplanado de claves, decode tolerante
│  ├─ capri-link/       servidor WebSocket → canal. No conoce el cuerpo.
│  ├─ capri-core/       ENTITY_BODY + resortes + TTL + idle + presets
│  │   └─ src/body/{transform,face,silhouette,extensions}.rs
│  ├─ capri-render/     wgpu + estilos
│  │   └─ shaders/{common,silhouette,extensions,hair,face,fx,main}.wgsl
│  └─ capri-app/        binario `capri`: ventana + loop
├─ controllers/         `capri-ctl` (CLI), `capri-eyes` (panel nativo opcional) + escenarios .jsonl
├─ assets/styles/       presets gráficos (construct.json, mono.json, capri-eyes.json)
└─ docs/
```

Dependencias en una sola dirección:

```
protocol ◄── link
   ▲
   └──── core ◄── render
                     ▲
app ─── usa link, core y render (es el único que los cablea)
```

Los controladores están fuera del cuerpo: `capri-eyes` usa egui/eframe solo con la feature
`visual` y manda objetivos por WebSocket; no enlaza `core`, `link` ni `render` en producción.
Su modelo guarda objetivos de prueba, no una copia del estado animado. `make test` abre el panel
y un proceso `capri` independiente con un puerto local libre. El cierre administra únicamente
ese proceso hijo. `make check` queda reservado para verificaciones automáticas.

---

## 7. Pendiente

- Verificar transparencia y click-through en macOS y Linux.
- Morph libre: que la entidad agregue nodos nuevos al árbol (hoy ajusta los existentes).
- Bajar RAM; partir passes si un estilo lo necesita; mods de render.
