---
name: exactitud-grafica
description: Proceso para que una forma del render de Capri (pelo, ojos, placa, silueta, extensiones) quede EXACTAMENTE como una imagen de referencia. Mide la referencia con números, la convierte en una receta en unidades R, la implementa, la renderiza sin ventana y la compara (IoU y mapa de diferencias) en todas las poses. Úsala cuando el usuario pase imágenes de referencia y pida "que se vea igual", "exacto ese estilo", "analiza la forma de verdad", "arregla la forma de X según esta imagen", o se queje de que una forma no se parece, se sale del cuadro o deja huecos al girar.
---

# Exactitud gráfica

Objetivo: que la forma del render coincida con la referencia **medida**, no con una impresión,
y que lo siga haciendo en cada pose. Los colores no importan salvo que se pidan: se compara la silueta.

Herramientas:
- `scripts/measure.py`: mide siluetas en unidades del render y compara referencia contra render.
  Necesita numpy, scipy y Pillow (`pip install numpy scipy pillow`).
- `cargo run -p capri-render --example snapshot -- out.png [--yaw D --pitch D --roll D] [--core full|eyes|bolt] [--grid --yaws a,b --pitches a,b] [--size WxH] [--ppr N]`:
  render sin ventana a PNG. Por defecto encuadra el pelo entero. `--grid` da una hoja de contacto
  con columnas = yaw y filas = pitch.

Unidades: R = radio del anillo del ojo. Origen = centro de la cabeza = centro de la placa, y hacia
arriba. La placa mide **8.08 × 3.308 R** y es el ancla natural para escalar cualquier referencia.

## 1. Preparar referencias

- Guarda cada referencia en una carpeta del scratchpad. Las URLs se bajan con `curl -sL`, y las
  `.webp` se pasan a PNG con Pillow (`Image.open(p).convert("RGBA").save(...)`), porque Read no las abre.
- Míralas todas con Read antes de medir. Anota qué es cada una (render 3D de frente, fotograma 2D,
  accesorio sin cabeza...). La más limpia y de frente será la **referencia principal**; las demás
  dan el estilo, no el tamaño.

## 2. Repartir el trabajo (subagentes en paralelo)

Lanza dos agentes `general-purpose` a la vez; ninguno depende del otro:

1. **Analista de forma**: lee las imágenes, mide con `measure.py` (o numpy/OpenCV propios) y
   devuelve el informe y la receta del paso 3. Pídele números en R relativos al ancla, no adjetivos,
   y que puntúe su receta contra la referencia (IoU) antes de entregarla. Que no toque el repo.
2. **Arnés de render**: si `examples/snapshot.rs` no cubre lo que hay que ver (otro núcleo, otra
   extensión, otra cámara), que lo amplíe sin tocar los archivos que vas a reescribir.

Mientras, lee tú el código de la forma (Rust + WGSL) para saber cómo se compone: capas, orden de
dibujo, qué va delante o detrás de la cara, qué transforma cada pose.

## 3. Medir la referencia

```
python .claude/skills/exactitud-grafica/scripts/measure.py stats REF.png \
  --shape "(r>110)&(r>2.2*g)&(r>1.6*b)" --anchor "(g>150)&(r<120)&(b<120)" --anchor-size 8.08x3.308
```

- Primero ajusta la expresión de `--shape` con `measure.py mask` hasta que la máscara sea solo la
  forma: incluye los brillos (`--close`) y excluye cuerpo y fondo (`--region`).
- Si algo tapa el borde de arriba del ancla, usa `--anchor-align bottom`. Mira el valor de
  `anchor_height_measured`: si se aleja más de un 5 % de 3.308, la escala vertical es dudosa.

El informe debe cubrir, con números:
- **Silueta**: extensión x/y, ancho/alto, dónde está la masa (sobre, al lado y bajo el ancla),
  `bbox_fill`, `hull_ratio` (compacta frente a espinosa) y el perfil polar por sectores (qué
  direcciones llegan más lejos).
- **Núcleo**: la forma que queda sin las púas (`core_fraction`) y su polígono aproximado.
- **Púas**: cuántas (principales y secundarias), histograma de direcciones, largos, y poblaciones
  por ángulo de vértice y ancho/largo. Microformas a nivel de vértice: colmillos, banderas,
  escalones, quiebres. Qué fracción lleva cada una.
- **Capas**: qué piezas van delante de la cara, a dónde apuntan y dónde caen sus puntas respecto a
  los ojos y la placa. Qué debe quedar siempre visible.
- **Huecos**: muescas, rendijas y su profundidad.
- **Invariantes**: reglas que hacen reconocible el estilo (p. ej. "nada apunta en vertical",
  "puntas de la corona a la misma altura", "flequillo barrido a −130°").

## 4. Receta: plantilla medida y variación acotada

Cuando el usuario pide "exacto", **no** inventes reglas procedurales genéricas (blobs, ondas,
abanicos): ninguna llega a la forma. Haz esto:
- **Plantilla**: triángulos (o primitivas) con coordenadas medidas en R. El analista la puntúa
  contra la referencia y debe llegar a IoU ≥ 0.9.
- **Variación**: si la forma debe cambiar (por pose, por tramo), desplaza vértices con una
  semilla. La amplitud depende del papel del vértice: casi nada en los bordes que abrazan la
  cara y en las puntas que deben caer en un sitio concreto, poco en la masa, más en las puntas
  de las púas. Calcula el desplazamiento **a partir de la posición del vértice**, así los vértices
  compartidos se mueven juntos y no se abren rendijas. La pose central es la plantilla exacta.
- **Capas explícitas**: detrás de la cara, delante de la cara (flequillo) y piezas que solo
  aparecen en ciertas poses (nuca de espalda).

## 5. Implementar

- Marca la capa por primitiva en el uniform y compón en el shader:
  `fondo → capa de detrás → cara y placa → capa de delante`.
- Silueta de una capa = **unión** de sus triángulos (distancia mínima, antialias solo en el borde
  exterior). Si compones cada triángulo con `over`, salen costuras en las aristas compartidas.
  Para ver las facetas, el tono es el del último triángulo que cubre el píxel.
- Si sube el número de primitivas, cambia a la vez la constante de Rust, el tamaño del array en
  WGSL y el assert de tamaño del uniform en `renderer.rs`. Descarta cada primitiva por su caja
  antes de evaluar su SDF.

## 6. Verificar (no lo des por bueno sin esto)

1. **Frente contra referencia**:
   ```
   cargo run -q -p capri-render --example snapshot -- SCRATCH/front.png
   python .../measure.py compare REF.png SCRATCH/front.png --shape "..." --render-shape "..." \
     --anchor "..." --anchor-size 8.08x3.308 --render-anchor-align bottom --diff SCRATCH/diff.png
   ```
   Mira el `diff.png`: rojo = falta en el render, azul = sobra, borde fino de un solo color =
   desfase de escala o posición, no de forma. Explica cada mancha grande; si es intencional (un
   relleno que pidió el usuario), dilo.
2. **Todas las poses**: hojas de contacto con extremos, no solo el frente:
   `--grid` (yaw ±40, pitch ±15), luego `--yaws 60,90,120,180 --pitches 0,-30` y
   `--yaws -40,0,40,60 --pitches 30,0,-30`. Con `--core full` y con `--core eyes`. Busca huecos
   entre piezas, recortes contra el borde del cuadro, piezas que se despegan y facetas raras.
3. **Invariantes como tests** en el módulo de la forma, recorriendo todos los tramos:
   - qué nunca se tapa (centros de los ojos, solo con la capa de delante);
   - qué siempre está cubierto (rejilla de puntos alrededor de la placa, proyectada con el giro);
   - que cabe en el uniform;
   - determinismo (mismo tramo, misma forma).
   En las pruebas de punto en triángulo usa tolerancia (`1e-3`): un punto sobre una arista
   compartida puede caer fuera de ambos triángulos por redondeo.
4. `cargo test -p capri-render`, `cargo clippy -p capri-render --all-targets`, `cargo build -p capri-app`.

## 7. Lecciones del pelo (no repetir)

- **Cosas pegadas a anclas distintas se separan al girar.** La masa del pelo vive detrás de la
  cabeza y se corre al lado contrario; la placa va con la cara. El hueco entre ambas se cierra con
  dos rellenos detrás de la cara: uno pegado al plano de la placa y una tapa que cierra el hueco en
  la masa. Cada uno sigue a su ancla y se solapan de sobra.
- **Los rellenos se dibujan primero**; si no, sus triángulos grandes pisan las facetas de la masa
  y se ven como planchas rectas.
- **Pitch fuerte** mueve la cara y la masa en sentidos opuestos en vertical: el relleno debe subir
  bajo la masa (y≈9) y bajar hasta el borde de los lóbulos.
- **La nuca** (lo que tapa la cabeza de espaldas) debe aparecer en cuanto la cara deja de mirar al
  frente (facing < 0.5), no a partir del perfil.
- **El encuadre del arnés** tiene que seguir al tamaño de la forma. Si la forma crece, actualiza
  el encuadre por defecto para que la hoja de contacto no recorte.
- **Morfología con radios grandes**: usa transformadas de distancia, no `binary_opening` con un
  disco de cientos de píxeles (tarda minutos).

## 8. Lecciones de la cara oscura (forma 3D desde un refsheet de ángulos)

- **Refsheet con varios ángulos → sólido de piezas convexas 3D**, no una plantilla 2D por pose.
  La proyección de un casco convexo es el casco de los puntos proyectados: la CPU proyecta,
  toma el casco 2D y sube polígonos. El analista optimiza los puntos contra todas las vistas a la
  vez (IoU ponderado, solo traslación por vista) con las restricciones de contención (placa, boca)
  dentro del optimizador.
- **Pon la proyección exacta del motor en el encargo** (orden yaw/pitch/roll y signos) y avisa si
  la referencia usa el signo contrario (la hoja vertical tenía "−45 = mirando arriba").
- **Las referencias IA no son coherentes entre vistas** (el 3/4 casi tan ancho como el perfil, la
  hoja vertical sin la púa central). Mide la simetría propia de la ref (espejo −90/+90) y decide
  qué vista manda; dilo en el informe.
- **La unión de triángulos de un abanico deja costuras** aunque uses distancia mínima: sobre la
  arista compartida las dos distancias valen ~0 y el antialias da alfa 0.5 (en el pelo las tapa
  el tono por faceta; en un color plano se ven). Manda polígonos convexos enteros y calcula su
  distancia exacta; entre piezas que se solapan no hay costura.
- **Una forma que se sale de la esfera de la cabeza** choca con capas pensadas para la esfera
  (el pelo repintado encima al girar): acótalas al disco de la cabeza.
- **Verifica el render real del motor**, no solo el raster del analista: `--head-only --bg ffffff
  --ppr <px/R de la ref>` deja el render a la escala de la referencia y se compara píxel a píxel.

## 9. Informe al usuario

En español y corto:
- qué se midió (dos o tres números que definen el estilo);
- cómo se trasladó a la lógica;
- IoU frente a la referencia;
- qué cambia en otras poses;
- qué quedó distinto a propósito.

Da la ruta absoluta de los PNG de frente, de la hoja de contacto y del diff. No hagas commit salvo
que lo pidan.
