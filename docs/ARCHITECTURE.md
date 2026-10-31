# Capri — Arquitectura

Capri es un **cuerpo digital**: un renderizador de escritorio ultra ligero que no piensa nada.
Una entidad externa (LLM, constructo de personalidad, script) le manda **datos** y el cuerpo **reacciona**.
Para la entidad, el cuerpo es una subrutina: aquí dentro no hay lógica de IA.

Referencia visual: Capri (*Cyberpunk: Edgerunners 2*): figura abstracta, deliberadamente "fuera de lugar",
silueta negra delgada, peinado exagerado en púas rosas con punta verde, placa verde con dos ojos, cuerpo
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
  corrigen después: `{"core":"eyes","mask":true}` = solo ojos + cara.

| Stream  | Para qué | Dueño de |
|---------|----------|----------|
| `face`  | Control facial | `Face/*` (Mask brillo, ojos, mirada, boca) |
| `body`  | Control corporal | `Silhouette/*` + posición / rotación / yaw / escala globales |
| `morph` | Control polimórfico, a gusto de la entidad | núcleos, opacidades, máscara visible, `Extensions/*` |
| `voice` | Voz (previsto ya) | nivel de voz: se **suma** a la boca y al brillo |

Una clave tiene **un solo stream dueño** (hay un test que lo garantiza): dos streams nunca se pelean.

---

## 3. Núcleos

| Núcleo | Qué queda | Cómo pedirlo |
|--------|-----------|--------------|
| `full` | Cuerpo completo + púas + cara | `{"s":"morph","set":{"core":"full"}}` |
| `eyes` | Solo los ojos, grandes y brillantes | `{"core":"eyes"}` · con cara: `{"core":"eyes","mask":true}` |
| `bolt` | Rayo en zigzag fragmentado, ojos arriba | `{"core":"bolt"}` |

Los núcleos son **pesos** (`core.full`, `core.eyes`, `core.bolt`) que se mezclan: cambiar de núcleo
es un morph, no un corte, y la entidad puede dejar el cuerpo a medio camino (`core.full: 0.5, core.bolt: 0.5`).

---

## 4. El árbol del cuerpo (`ENTITY_BODY`)

Única fuente de verdad, en `capri-core/src/body/`. Los streams escriben aquí y el render solo lee.

```
ENTITY_BODY
├── TransformController      transform global + ShapeMorph (pesos de núcleos) + Opacity
├── Face                     Mask, EyeLeft, EyeRight, Mouth         → face.wgsl
├── Silhouette               Torso, Arms, Legs (+ opacidad propia)  → silhouette.wgsl
└── Extensions               Spikes (fusionadas), Fragments (sueltas) → extensions.wgsl
```

Cada rama declara sus parámetros en un único lugar (`visit`): stream dueño + clave + parámetro.
De ahí salen el ruteo de mensajes, el tick de resortes y la lista de claves válidas en los errores.

---

## 5. Render

**Un solo draw call** a pantalla completa. Las capas están separadas en módulos WGSL que siguen
el árbol y se componen en `main.wgsl`:

```
 core.tick()      resortes, TTL, idle (respiración, parpadeo)
 Frame::build()   el árbol → un uniform de 336 bytes
 GPU, por píxel:
   main.wgsl        TransformController: posición, rotación, escala, yaw
   silhouette.wgsl  esqueleto (FK) + SDF de torso/brazos/piernas
   extensions.wgsl  púas, fragmentos, rayo
   main.wgsl        mezcla de núcleos (mezcla de campos de distancia = morph orgánico)
                    volumen falso: normal del gradiente del SDF + luz que sigue al yaw
   face.wgsl        placa, ojos, boca: 2D nítido, pegado a la cabeza
   fx.wgsl          glitch, scanlines
```

### Ojos

Los dos ojos salen de una misma función (`eye_layer` en `face.wgsl`) que trabaja en espacio de
ojo: unidad = R (radio exterior del anillo), x positiva hacia el centro de la cara; el ojo derecho
es el espejo del izquierdo. Capas, de atrás hacia delante:

```
placa 8.08 × 3.31 R (2.44:1) · centros a ±2.03 R, 0.365 R bajo el centro de la placa
silueta   almendra (dos arcos entre esquinas en punta) + púa superior (hacia el centro)
          + púa inferior (hacia fuera) + talón en la esquina interior
anillo    0.905 R .. 1 R, mínimo 1.1 px para seguir legible a tamaño normal
iris      0.585 R          ┐ se desplazan juntos con `gaze` (máx. 0.27 R, siempre dentro del anillo)
centro    0.305 R          ┘ y los recorta una ventana de párpados que se cierra con `open`
```

- `open` (× parpadeo de idle) cierra la ventana de párpados: los círculos se recortan, no se
  aplastan. Con `open = 0` no asoma nada del iris ni del centro.
- `size` escala disco, anillo, iris y centro; la silueta crece la mitad. Con placa, todo el ojo
  queda recortado dentro de ella.
- Las proporciones son constantes al principio de `face.wgsl`, ajustadas contra la imagen de
  referencia. Los colores vienen del estilo (`mask`, `eye_shell`, `eye_ring`, `eyes`, `pupil`).
- Halo y brillo propios de la cara escalan con `style.glow`: con `glow: 0` los colores salen planos.

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
| `body`, `rim`, `hair`, `tip` | silueta, contorno, púas | valores de `construct` |
| `mask` | placa | `#39ff88` |
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
│  │   └─ shaders/{common,silhouette,extensions,face,fx,main}.wgsl
│  └─ capri-app/        binario `capri`: ventana + loop
├─ controllers/         binario `capri-ctl` + escenarios .jsonl de verificación visual
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

---

## 7. Pendiente

- Verificar transparencia y click-through en macOS y Linux.
- Visemas en `voice` (hoy solo `level`).
- Morph libre: que la entidad agregue nodos nuevos al árbol (hoy ajusta los existentes).
- Bajar RAM; partir passes si un estilo lo necesita; mods de render.
