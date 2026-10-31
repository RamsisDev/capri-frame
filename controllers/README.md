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

## Escenarios

| Archivo | Qué verifica |
|---------|--------------|
| `01_face.jsonl` | idle (parpadeo), mirada, los 7 presets de expresión, valores crudos |
| `02_body.jsonl` | las 6 poses, yaw (volumen), posición/rotación/escala, un brazo crudo |
| `03_cores.jsonl` | full → eyes → eyes + cara → bolt → mezcla cruda → full |
| `04_extensions.jsonl` | púas, fragmentos, opacidad de la silueta y global |
| `05_ttl_silence.jsonl` | vuelta sola tras el `ttl`, descarte de `seq` atrasado, ráfagas |
| `06_voice.jsonl` | pulsos de voz sobre la boca y la placa |

## Formato `.jsonl`

```
# texto                         → se imprime "VERIFICAR: texto"
{"wait":1000}                   → pausa
{"wait":500,"s":"face",...}     → espera 500 ms y manda el resto
{"s":"body","set":{...}}        → se manda tal cual
```
