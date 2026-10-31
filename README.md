# Capri

Cuerpo digital ultra ligero para una entidad (LLM, constructo de personalidad). Es solo un display:
una ventana flotante, transparente, siempre encima y que deja pasar los clics. Dibuja un cuerpo
abstracto que reacciona a streams de datos. Inspirado en Capri (*Cyberpunk: Edgerunners 2*).

```sh
cargo run --release -p capri-app            # el cuerpo (se cierra con Ctrl+C en la terminal)
cargo run -p capri-ctl -- all               # en otra terminal: todos los escenarios de prueba
make test                                  # prueba manual: panel de ojos + Capri en ventanas separadas
```

El panel permite abrir/cerrar cada ojo, hacer guiños, ajustar tamaños y mirada, consultar el JSON
y aprobar un ajuste guardándolo como escenario. Requiere GNU Make en el `PATH`; los comandos Cargo
equivalentes están en [controllers/README.md](controllers/README.md). `make check` ejecuta las pruebas
automáticas y Clippy, sin abrir ventanas.

Opciones de `capri`: `--port 7777` · `--size 360x520` · `--style assets/styles/mono.json` ·
`--interactive` (la ventana recibe el ratón) · `--no-top`.

La entidad se conecta a `ws://127.0.0.1:7777` y manda JSON:

```json
{"s":"face","set":{"expr":"happy"}}
{"s":"morph","set":{"core":"bolt"}}
```

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): cómo está armado.
- [docs/PROTOCOL.md](docs/PROTOCOL.md): todas las claves de los streams.
- [controllers/README.md](controllers/README.md): controladores de prueba y verificación visual.
