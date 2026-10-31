# Controladores de prueba

`capri-ctl` habla con el cuerpo por el mismo WebSocket que usaría la entidad. Valida cada mensaje
contra el protocolo antes de mandarlo. La verificación es **visual**: cada escenario imprime qué
mirar en pantalla (`VERIFICAR: ...`) mientras manda los mensajes.

```sh
cargo run -p capri-ctl -- all                                  # todos los escenarios, en orden
cargo run -p capri-ctl -- play controllers/scenarios/03_cores.jsonl --loop
cargo run -p capri-ctl -- send '{"s":"face","set":{"expr":"happy"}}'
some-entity | cargo run -p capri-ctl -- pipe                   # una línea JSON por mensaje
```

En PowerShell 5.1 usa `pipe` en vez de `send`, porque PowerShell quita las comillas al pasarle
argumentos a un `.exe`:

```powershell
'{"s":"face","set":{"expr":"happy"}}' | cargo run -p capri-ctl -- pipe
```

## Panel nativo de ojos

```sh
make test
```

Compila y abre **dos ventanas en procesos separados**: `capri-eyes` (controles) y `capri`
(el render real, ampliado a 640×640, con `assets/styles/capri-eyes.json`). El controlador vive
en `controllers/src/visual/`; su dependencia gráfica es opcional (`--features visual`) y no
se añade al cuerpo ni al controlador CLI. Usa el mismo WebSocket y los mismos streams.

- Apertura y tamaño independientes, o vinculados, con valores numéricos editables.
- Abrir, cerrar, entrecerrar, guiños, mirada por ejes y botones de dirección.
- Ojos, cuerpo o rayo y zoom para comparar el mismo ajuste en contexto.
- Claves exactas junto a cada control, JSON copiable y último mensaje enviado.
- **Aprobar y guardar** crea `controllers/captures/eyes-<marca de tiempo>.jsonl`, sin sobrescribir
  escenarios. Reprodúcelo con `cargo run -p capri-ctl -- play controllers/captures/<archivo>.jsonl`.
- Inspector del archivo de estilo al lanzar la prueba; edítalo en el IDE y Capri recarga los cambios.

Los números son **objetivos enviados**, no lecturas del estado interno: el protocolo actual no tiene
telemetría ni confirmación de aplicación. El cuerpo conserva sus resortes y parpadeo automático.
Los ajustes no llevan TTL; permanecen hasta otro mensaje o hasta cerrar el cuerpo. Guardar un ajuste
no cambia los valores base del cuerpo ni el archivo de estilo. En una conexión a un Capri ya abierto,
el panel envía sus objetivos iniciales al conectarse; **Reaplicar** vuelve a enviar los controles.

`make test` elige un puerto local libre y solo administra el Capri que acaba de lanzar. Cerrar el
panel cierra ese proceso; cerrar la ventana del cuerpo termina también la sesión del panel.
Una conexión interrumpida se reintenta sin bloquear la interfaz y reenvía los últimos objetivos.

GNU Make debe estar instalado y disponible en el `PATH`. Comandos equivalentes (también PowerShell):

```sh
cargo build -p capri-app --bin capri
cargo run -p capri-ctl --features visual --bin capri-eyes -- --spawn
```

Para controlar un Capri que ya está abierto (al cerrar el panel, ese proceso sigue vivo):

```sh
cargo run -p capri-ctl --features visual --bin capri-eyes -- --port 7777
```

Opciones: `--spawn`, `--port <puerto>` y `--style <archivo.json>` (estilo solo con `--spawn`).
Para las pruebas automáticas usa `make check` o `cargo test --workspace --all-features`.

Verificación manual: cerrar ambos ojos (sin iris visible), probar cada guiño, desligar tamaños,
mover la mirada a extremos, comparar núcleos y guardar/reproducir un ajuste.
El anillo y la silueta del ojo permanecen al cerrar los párpados; se recortan el iris y el centro.

## Escenarios de secuencias

| Archivo | Qué verifica |
|---------|--------------|
| `01_face.jsonl` | idle (parpadeo), mirada, los 7 presets de expresión, valores crudos |
| `02_body.jsonl` | las 6 poses, yaw (volumen), posición/rotación/escala, un brazo crudo |
| `03_cores.jsonl` | full → eyes → eyes + cara → bolt → mezcla cruda → full |
| `04_extensions.jsonl` | púas, fragmentos, opacidad de la silueta y global |
| `05_ttl_silence.jsonl` | vuelta sola tras el `ttl`, descarte de `seq` atrasado, ráfagas |
| `06_voice.jsonl` | pulsos de voz sobre el brillo de la placa (la boca aún no se dibuja) |
| `07_eyes.jsonl` | base de los ojos: mirada, guiños, cierre, tamaños, presets y transiciones de núcleo (mejor con `--style assets/styles/capri-eyes.json`) |

## Formato `.jsonl`

```
# texto                         → se imprime "VERIFICAR: texto"
{"wait":1000}                   → pausa
{"wait":500,"s":"face",...}     → espera 500 ms y manda el resto
{"s":"body","set":{...}}        → se manda tal cual
```
