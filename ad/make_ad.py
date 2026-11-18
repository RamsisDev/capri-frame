"""Anuncio de Capri: voz (edge-tts), lipsync por visemas, música sintetizada y montaje (ffmpeg).

    target/ad/venv/Scripts/python ad/make_ad.py      # deja ad/capri-ad.mp4

Requiere en el venv: edge-tts, imageio-ffmpeg, numpy. Los intermedios van a target/ad/build.
La cara la dibuja el ejemplo `film` de capri-render a partir de una línea de tiempo de mensajes
del protocolo (docs/PROTOCOL.md), así que el video usa el mismo render que la ventana.
"""

import asyncio
import json
import math
import shutil
import subprocess
import sys
import unicodedata
import wave
from pathlib import Path

import edge_tts
import imageio_ffmpeg
import numpy as np

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "target" / "ad" / "build"
OUT = ROOT / "ad" / "capri-ad.mp4"
FFMPEG = imageio_ffmpeg.get_ffmpeg_exe()

VOICE, RATE, PITCH = "es-MX-DaliaNeural", "+6%", "+4Hz"
W, H, FPS, SR = 1920, 1080, 60, 48000
GREEN = "0x11e965"
LEAD = 0.045  # los visemas salen un poco antes: el resorte de la boca tarda en llegar

# (texto hablado, pausa antes en s, subtítulo, mensajes al empezar la frase)
PHRASES = [
    ("Hola.", 0.5, "Hola.", [{"s": "face", "set": {"expr": "neutral", "mouth.curve": 0.6}}]),
    ("Soy Capri.", 0.25, "Soy Capri.", []),
    ("Tu inteligencia artificial ya piensa, y ya habla.", 0.55, "Tu IA ya piensa, y ya habla.",
     [{"s": "face", "set": {"expr": "neutral"}}]),
    ("Pero nadie la ha visto.", 0.3, "Pero nadie la ha visto.", [{"s": "face", "set": {"expr": "sad"}}]),
    ("Yo soy su cara.", 0.55, "Yo soy su cara.",
     [{"s": "face", "set": {"expr": "neutral", "mouth.curve": 0.7}}, {"s": "body", "set": {"scale": 1.18}}]),
    ("Un cuerpo digital, ultra ligero, que flota sobre tu pantalla.", 0.5,
     "Un cuerpo digital, ultraligero, que flota sobre tu pantalla.",
     [{"s": "face", "set": {"mouth.curve": 0.3}}, {"s": "body", "set": {"scale": 1.0}}]),
    ("Puedo mirar.", 0.5, "Puedo mirar.", [{"s": "face", "set": {"expr": "neutral"}}]),
    ("Sonreír.", 0.45, "Sonreír.", [{"s": "face", "set": {"expr": "happy", "gaze": [0, 0]}}]),
    ("Enojarme.", 0.45, "Enojarme.", [{"s": "face", "set": {"expr": "angry"}}]),
    ("Y sorprenderme.", 0.45, "Y sorprenderme.", [{"s": "face", "set": {"expr": "surprised"}}]),
    ("Conéctame por WebSocket, mándame un JSON, y cobro vida.", 0.6,
     "Conéctame por WebSocket, mándame un JSON, y cobro vida.",
     [{"s": "face", "set": {"expr": "neutral", "mouth.curve": 0.4}}]),
    ("Capri.", 0.7, "", [{"s": "face", "set": {"expr": "neutral", "mouth.curve": 0.5}},
                         {"s": "body", "set": {"scale": 0.55, "pos.y": 0.42}}]),
    ("Dale un cuerpo a tu inteligencia artificial.", 0.3, "", []),
]
START = 2.2  # la voz entra tras el arranque (ojos cerrados → abiertos)
TAIL = 2.6   # cierre con el título tras la última palabra


def ffmpeg(*args, **kw):
    return subprocess.run([FFMPEG, "-hide_banner", "-loglevel", "error", "-y", *args], check=True, **kw)


# ---------------------------------------------------------------- voz

async def synth(text, path):
    """Sintetiza una frase; devuelve [(inicio, duración, palabra)] en segundos."""
    words = []
    com = edge_tts.Communicate(text, VOICE, rate=RATE, pitch=PITCH, boundary="WordBoundary")
    with open(path, "wb") as f:
        async for chunk in com.stream():
            if chunk["type"] == "audio":
                f.write(chunk["data"])
            elif chunk["type"] == "WordBoundary":
                words.append((chunk["offset"] / 1e7, chunk["duration"] / 1e7, chunk["text"]))
    return words


def decode(path):
    raw = ffmpeg("-i", str(path), "-f", "s16le", "-ac", "1", "-ar", str(SR), "-", stdout=subprocess.PIPE).stdout
    return np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0


def trim(x, floor=0.004):
    """Quita el silencio de los extremos; devuelve (audio, segundos quitados al principio)."""
    loud = np.flatnonzero(np.abs(x) > floor)
    if loud.size == 0:
        return x, 0.0
    a, b = max(loud[0] - SR // 100, 0), min(loud[-1] + SR // 10, len(x))
    return x[a:b], a / SR


# ---------------------------------------------------------------- lipsync

ZERO = dict.fromkeys(["x", "mbp", "a", "e", "i", "o", "u", "fl"], 0.0)


def visemes(word):
    """Grafemas del español → [(pesos de visema, unidades de tiempo)]."""
    w = "".join(c for c in unicodedata.normalize("NFD", word.lower()) if c.isalpha() and not unicodedata.combining(c))
    out = []
    for i, c in enumerate(w):
        prev, nxt = w[i - 1] if i else "", w[i + 1] if i + 1 < len(w) else ""
        if c == "h" or (c == "u" and nxt in "ei" and prev in "qg"):
            continue
        if c in "aeiou":
            out.append(({c: 1.0}, 2.0))
        elif c == "y" and nxt not in "aeiou":
            out.append(({"i": 1.0}, 2.0))
        elif c == "w":
            out.append(({"u": 1.0}, 1.0))
        elif c in "mbpv":
            out.append(({"mbp": 1.0}, 1.0))
        elif c in "fl":
            out.append(({"fl": 1.0}, 1.0))
        else:
            # Consonante sin forma propia: la boca se entrecierra hacia la vocal que viene.
            vowel = next((v for v in w[i + 1:] if v in "aeiou"), "e")
            out.append(({vowel: 0.35}, 1.0))
    return out


def voice_msg(weights, level):
    keys = {f"viseme.{k}": round(v, 3) for k, v in {**ZERO, **weights}.items()}
    return {"s": "voice", "set": {**keys, "level": level}}


def lipsync(words):
    """Palabras [(inicio, duración, texto)] en tiempo absoluto → eventos (t, mensaje)."""
    ev = []
    for n, (t0, dur, text) in enumerate(words):
        segs = visemes(text)
        total = sum(u for _, u in segs) or 1.0
        t = t0
        for weights, units in segs:
            ev.append((t - LEAD, voice_msg(weights, 0.12 if max(weights.values()) == 1.0 else 0.0)))
            t += dur * units / total
        gap = words[n + 1][0] - (t0 + dur) if n + 1 < len(words) else 1.0
        if gap > 0.07:
            ev.append((t0 + dur - LEAD, voice_msg({}, 0.0)))
    return ev


# ---------------------------------------------------------------- música

def music(secs, title_at):
    """Base sintetizada: pad, bajo en corcheas, bombo y un barrido en el arranque. Estéreo."""
    n = int(secs * SR)
    t = np.arange(n) / SR
    beat = 60 / 96
    bar = 4 * beat
    chords = [(45, 52, 57, 60, 64), (41, 48, 53, 57, 60), (48, 55, 60, 64, 67), (43, 50, 55, 59, 62)]  # Am F C G
    hz = lambda m: 440 * 2 ** ((m - 69) / 12)
    left, right = np.zeros(n), np.zeros(n)
    first = START - 0.2  # el ritmo entra con la voz
    k = 0
    while first + k * bar < secs:
        a = first + k * bar
        i0, i1 = int(max(a, 0) * SR), min(int((a + bar) * SR), n)
        tt = t[i0:i1] - a
        env = np.minimum(tt / 0.4, 1) * np.minimum((bar - tt) / 0.4, 1)
        chord = chords[k % 4]
        for j, m in enumerate(chord[1:]):
            for det, side in ((-0.004, 0), (0.004, 1)):
                f = hz(m + 12) * (1 + det)
                tone = (np.sin(2 * math.pi * f * tt) + 0.3 * np.sin(4 * math.pi * f * tt)) * env * 0.035
                (left if side == 0 else right)[i0:i1] += tone * (1.0 if (j + side) % 2 else 0.6)
        # bajo en corcheas y bombo en negras
        for e in range(8):
            b0 = a + e * beat / 2
            j0, j1 = int(b0 * SR), min(int((b0 + beat / 2) * SR), n)
            if j0 >= n or j0 < 0:
                continue
            bt = t[j0:j1] - b0
            f = hz(chord[0] - 12)
            bass = np.tanh(2.5 * np.sin(2 * math.pi * f * bt)) * np.exp(-bt * 9) * 0.11
            left[j0:j1] += bass
            right[j0:j1] += bass
            if e % 2 == 0:
                kick = np.sin(2 * math.pi * (45 * bt + 14 * (1 - np.exp(-bt * 28)))) * np.exp(-bt * 13) * 0.3
                left[j0:j1] += kick
                right[j0:j1] += kick
            else:
                hat = np.random.default_rng(e + 8 * k).standard_normal(j1 - j0) * np.exp(-bt * 70) * 0.025
                left[j0:j1] += hat * 0.7
                right[j0:j1] += hat
        k += 1
    # arranque: barrido que sube hasta que abre los ojos
    s = t[: int(1.1 * SR)]
    sweep = np.sin(2 * math.pi * (70 * s + 260 * s * s)) * (s / 1.1) ** 2 * 0.1
    left[: len(s)] += sweep
    right[: len(s)] += sweep
    gain = np.where(t < title_at, 0.75, 1.0)  # sube un poco en el cierre
    fade = np.minimum(t / 0.3, 1) * np.minimum((secs - t) / 1.2, 1)
    return np.stack([left, right], axis=1) * (gain * fade)[:, None]


def write_wav(path, x):
    x = np.atleast_2d(x.T).T
    with wave.open(str(path), "wb") as f:
        f.setnchannels(x.shape[1])
        f.setsampwidth(2)
        f.setframerate(SR)
        f.writeframes((np.clip(x, -1, 1) * 32767).astype(np.int16).tobytes())


# ---------------------------------------------------------------- montaje

def drawtext(name, text, font, size, color, y, a, b, x="(w-text_w)/2"):
    (BUILD / f"{name}.txt").write_text(text, encoding="utf-8")
    alpha = f"clip(min((t-{a:.3f})/0.18\\,({b:.3f}-t)/0.18)\\,0\\,1)"
    return (f"drawtext=fontfile={font}:textfile={name}.txt:expansion=none:fontsize={size}:fontcolor={color}"
            f":x={x}:y={y}:alpha='{alpha}':enable='between(t\\,{a:.3f}\\,{b:.3f})'")


async def main():
    BUILD.mkdir(parents=True, exist_ok=True)
    for font in ("bahnschrift.ttf", "consolab.ttf"):
        shutil.copy(Path("C:/Windows/Fonts") / font, BUILD / font)

    # Voz: una síntesis por frase, colocadas en la pista con sus pausas.
    events, captions, clips, words_all = [], [], [], []
    t = START
    starts = []
    for n, (text, gap, caption, msgs) in enumerate(PHRASES):
        mp3 = BUILD / f"p{n:02}.mp3"
        words = await synth(text, mp3)
        audio, cut = trim(decode(mp3))
        t += gap
        starts.append(t)
        clips.append((t, audio))
        events += [(t - 0.12, m) for m in msgs]
        words_all += [(t + w0 - cut, d, txt) for w0, d, txt in words]
        end = t + len(audio) / SR
        if caption:
            captions.append((caption, t - 0.1, end + 0.15))
        t = end
    secs = t + TAIL
    title_at = starts[11]
    events += lipsync(words_all)

    voice = np.zeros(int(secs * SR), dtype=np.float32)
    for at, audio in clips:
        i = int(at * SR)
        voice[i:i + len(audio)] += audio[: len(voice) - i]
    write_wav(BUILD / "voice.wav", voice / max(np.abs(voice).max(), 1e-6) * 0.9)
    write_wav(BUILD / "music.wav", music(secs, title_at))

    # Actuación: arranque, miradas y un vaivén continuo de la cabeza.
    events += [
        (0.0, {"s": "face", "set": {"expr": "sleep"}}),
        (0.0, {"s": "body", "set": {"pos.y": 0.1}}),  # sitio para los subtítulos
        (1.0, {"s": "face", "set": {"expr": "neutral"}}),
        (1.35, {"s": "face", "set": {"gaze": [-0.8, 0.1]}}),
        (1.75, {"s": "face", "set": {"gaze": [0.8, 0.1]}}),
        (2.1, {"s": "face", "set": {"gaze": [0, 0]}}),
        # "Puedo mirar."
        (starts[6] + 0.15, {"s": "face", "set": {"gaze": [-0.9, 0]}}),
        (starts[6] + 0.55, {"s": "face", "set": {"gaze": [0.9, 0]}}),
        (starts[6] + 0.95, {"s": "face", "set": {"gaze": [0, 0.8]}}),
        (starts[6] + 1.3, {"s": "face", "set": {"gaze": [0, 0]}}),
        # cierre: sonrisa con el título
        (starts[12] + 2.3, {"s": "face", "set": {"expr": "happy"}}),
    ]
    wide = (starts[5], starts[6])  # "que flota sobre tu pantalla": gira más
    for i in range(int(secs / 0.15)):
        s = i * 0.15
        amp = 2.4 if wide[0] <= s < wide[1] else 1.0
        still = 0.0 if s < 1.0 else 1.0
        events.append((s, {"s": "body", "set": {
            "yaw": round(still * amp * 9 * math.sin(0.9 * s), 2),
            "pitch": round(still * 4 * math.sin(0.63 * s + 1.0), 2),
            "roll": round(still * amp * 2.5 * math.sin(0.47 * s + 2.0), 2),
        }}))
    events.sort(key=lambda e: e[0])
    (BUILD / "timeline.tsv").write_text(
        "".join(f"{max(s, 0):.4f}\t{json.dumps(m, separators=(',', ':'))}\n" for s, m in events), encoding="utf-8")

    # Texto en pantalla.
    filters = ["vignette=PI/4.5"]
    for n, (text, a, b) in enumerate(captions):
        filters.append(drawtext(f"cap{n}", text, "bahnschrift.ttf", 46, "white", "h-150", a, b))
    js = '{"s":"face","set":{"expr":"happy"}}'
    filters.append(drawtext("json", "ws://127.0.0.1:7777   " + js, "consolab.ttf", 30, GREEN, "h-225",
                            starts[10] + 0.9, starts[11] - 0.25))
    end = secs + 1
    filters.append(drawtext("title", "CAPRI", "bahnschrift.ttf", 170, GREEN, "h*0.60", title_at + 0.25, end))
    filters.append(drawtext("tag", "Dale un cuerpo a tu IA.", "bahnschrift.ttf", 54, "white", "h*0.60+190",
                            starts[12] + 0.1, end))
    filters.append(drawtext("how", "cargo run --release -p capri-app", "consolab.ttf", 30, "0x8a8a96", "h*0.60+285",
                            starts[12] + 1.6, end))
    filters += ["fade=t=in:st=0:d=0.5", f"fade=t=out:st={secs - 0.7:.3f}:d=0.7", "format=yuv420p"]
    audio = ("[1:a]highpass=f=90,aecho=0.8:0.5:24:0.18,pan=stereo|c0=c0|c1=c0[v];"
             "[v][2:a]amix=inputs=2:normalize=0,alimiter=limit=0.95[a]")

    subprocess.run(["cargo", "build", "--release", "-q", "-p", "capri-render", "--example", "film"], cwd=ROOT, check=True)
    film = subprocess.Popen(
        [str(ROOT / "target/release/examples/film.exe"), str(BUILD / "timeline.tsv"),
         "--size", f"{W}x{H}", "--fps", str(FPS), "--secs", f"{secs:.3f}"],
        cwd=ROOT, stdout=subprocess.PIPE)
    OUT.parent.mkdir(exist_ok=True)
    ffmpeg("-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-",
           "-i", "voice.wav", "-i", "music.wav",
           "-filter_complex", f"[0:v]{','.join(filters)}[vid];{audio}", "-map", "[vid]", "-map", "[a]",
           "-c:v", "libx264", "-preset", "slow", "-crf", "17", "-c:a", "aac", "-b:a", "192k",
           "-movflags", "+faststart", "-t", f"{secs:.3f}", str(OUT), cwd=BUILD, stdin=film.stdout)
    if film.wait() != 0:
        sys.exit("film falló")
    print(f"{OUT} · {secs:.1f} s")


if __name__ == "__main__":
    asyncio.run(main())
