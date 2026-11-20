"""Refresh the factory kits' samples and CREDITS.md from their sources.

Both sources are CC0 and both are named in the drum machine's design spec:

  * smpldsnds/sonic-pi-samples — CC0 per its README, which links a freesound page per sample
  * sgossner/VCSL              — CC0 per its README

This is the only place the mapping from a voice to a recording is written down: CREDITS.md is
generated from the same table, so a file's credit and the bytes beside it cannot disagree. To change
a voice's sound, change this table and re-run; do not hand-edit a WAV or the credits.

Usage:  python3 rust/crates/acid-app/kits/refresh.py [--measure-only] [--cache DIR]

Every voice becomes 16-bit mono 44.1 kHz, trimmed to at most MAX_SECONDS with a fade at the tail,
and peak-normalised — `include_bytes!` puts every byte in the binary, and a clip plays back as a
click.
"""

import argparse

import os
import struct
import subprocess
import sys
import tempfile
import wave

MAX_SECONDS = 0.6
TARGET_RATE = 44100
PEAK = 0.89
FADE = 0.010  # seconds of fade at the tail, so a cut sample cannot click

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = HERE
SONIC_PI = "https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/{name}.ogg"
VCSL = "https://raw.githubusercontent.com/sgossner/vcsl/master/{path}"
SONIC_PI_LICENCE = "CC0 (Sonic Pi's sample set)"
VCSL_LICENCE = "CC0 (Versilian Community Sample Library)"

# The voice order is `Drum::ALL`: Kick, Snare, Rim, Clap, ClosedHat, OpenHat, LowTom, MidTom,
# HighTom, Crash, Ride, Cowbell, LowConga, HighConga, Clave, Shaker. A seventeenth voice is a
# third mask, not a row here.
VOICES = [
    "KICK", "SNR", "RIM", "CLAP", "CHH", "OHH", "LOT", "MID",
    "HTOM", "CRS", "RDE", "CWB", "LCO", "HCO", "CLV", "SHK",
]

# (source, path or name, what the recording is) per voice, per kit.
KITS = {
    "acoustic": [
        ("vcsl", "Membranophones/Struck Membranophones/Bass Drum 1/BDrumNew_hit_v5_rr1_Sum.wav", "Bass Drum 1, hard hit"),
        ("vcsl", "Membranophones/Struck Membranophones/Snare Drum, Modern 1/Snare2_HitSN_v5_rr1_Mid.wav", "Snare Drum, Modern 1, stick hit"),
        ("vcsl", "Membranophones/Struck Membranophones/Tom 1/TomH_rimS_v2_rr1_Mid.wav", "Tom 1, rim shot"),
        ("vcsl", "Idiophones/Struck Idiophones/Claps/Clap_rr1.wav", "Claps, single"),
        ("vcsl", "Idiophones/Struck Idiophones/Hi-Hat Cymbal/HiHat_HitC_v2_rr1_Mid.wav", "Hi-hat, closed"),
        ("vcsl", "Idiophones/Struck Idiophones/Hi-Hat Cymbal/HiHat_HitO_rr1_Mid.wav", "Hi-hat, open"),
        ("vcsl", "Membranophones/Struck Membranophones/Tom 2/Stick/TomL_HitS_v3_rr1_Mid.wav", "Tom 2, low, stick hit"),
        ("vcsl", "Membranophones/Struck Membranophones/Legacy Toms/tenor_lower/tenor_mf_rr1.wav", "Legacy tenor tom, lower, medium"),
        ("vcsl", "Membranophones/Struck Membranophones/Tom 1/Stick/TomH_HitS_v3_rr1_Mid.wav", "Tom 1, high, stick hit"),
        ("vcsl", "Idiophones/Struck Idiophones/Suspended Cymbal 1/susCymb1_hit_fff1.wav", "Suspended cymbal, hardest hit"),
        ("vcsl", "Idiophones/Struck Idiophones/Suspended Cymbal 1/susCymb1_hit_bell_mf1.wav", "Suspended cymbal, on the bell"),
        ("vcsl", "Idiophones/Struck Idiophones/Cowbells/Cowbell1_Normal_v2_rr1_Mid.wav", "Cowbell 1, normal"),
        ("vcsl", "Membranophones/Struck Membranophones/Conga/Tumba_HitN_v2_rr1_Sum.wav", "Conga, tumba (low), open tone"),
        ("vcsl", "Membranophones/Struck Membranophones/Conga/Quinto_HitN_v2_rr1_Sum.wav", "Conga, quinto (high), open tone"),
        ("vcsl", "Idiophones/Struck Idiophones/Claves/Claves1_Hit_v2_rr1_Mid.wav", "Claves 1, hit"),
        ("vcsl", "Idiophones/Struck Idiophones/Shaker, Small/Mid_Shaker_Slap_rr1.wav", "Small shaker, slap"),
    ],
    "808": [
        # `bd_808` is a 40 Hz sine with almost nothing above it: on any speaker smaller than a subwoofer the kit's kick
        # was silent (5.4% of its RMS above 200 Hz). `bd_fat` keeps the pitch of a drum machine kick and has an attack.
        ("sp", "bd_fat", "fat bass drum"),
        ("sp", "sn_dolf", "snare drum"),
        ("sp", "elec_tick", "electro tick"),
        ("sp", "perc_snap2", "hand clap"),
        ("sp", "hat_cats", "hi-hat"),
        ("sp", "hat_zild", "hi-hat"),
        ("sp", "elec_bong", "electro bongo"),
        ("sp", "elec_wood", "electro wood block"),
        ("sp", "elec_fuzz_tom", "electro fuzz tom"),
        ("sp", "elec_cymbal", "electro cymbal"),
        ("vcsl", "Idiophones/Struck Idiophones/Suspended Cymbal 1/susCymb1_hit_stick_mp1.wav", "Suspended cymbal, stick, medium"),
        ("sp", "drum_cowbell", "cowbell"),
        ("vcsl", "Membranophones/Struck Membranophones/Conga/Tumba_HitFM_v2_rr1_Sum.wav", "Conga, tumba (low), muted tone"),
        ("vcsl", "Membranophones/Struck Membranophones/Conga/Quinto_HitFM1_v2_rr1_Sum.wav", "Conga, quinto (high), muted tone"),
        ("vcsl", "Idiophones/Struck Idiophones/Claves/Claves2_Hit_v2_rr1_Mid.wav", "Claves 2, hit"),
        ("vcsl", "Idiophones/Struck Idiophones/Shaker, Large/LShaker_Hit_rr1_Mid.wav", "Large shaker, hit"),
    ],
    "909": [
        ("sp", "bd_haus", "haus bass drum"),
        ("sp", "sn_generic", "snare drum"),
        ("sp", "elec_pop", "electro pop"),
        ("sp", "perc_snap", "hand clap"),
        ("sp", "hat_tap", "hi-hat"),
        ("sp", "hat_psych", "hi-hat"),
        ("vcsl", "Membranophones/Struck Membranophones/Tom 2/Mallet/TomL_HitM_v3_rr1_Mid.wav", "Tom 2, low, mallet"),
        ("vcsl", "Membranophones/Struck Membranophones/Tom 1/Mallet/TomH_HitM_v3_rr1_Mid.wav", "Tom 1, high, mallet"),
        ("vcsl", "Membranophones/Struck Membranophones/Legacy Toms/tenor_higher/tenorH_mf_rr1.wav", "Legacy tenor tom, higher, medium"),
        ("sp", "drum_cymbal_hard", "cymbal, hard"),
        ("sp", "drum_cymbal_soft", "cymbal, soft"),
        ("vcsl", "Idiophones/Struck Idiophones/Cowbells/Cowbell2_Normal_v2_rr1_Mid.wav", "Cowbell 2, normal"),
        ("vcsl", "Membranophones/Struck Membranophones/Conga/Tumba_HitN_v3_rr1_Sum.wav", "Conga, tumba (low), open tone"),
        ("vcsl", "Membranophones/Struck Membranophones/Conga/Conga_HitN_v2_rr1_Sum.wav", "Conga, open tone"),
        ("vcsl", "Idiophones/Struck Idiophones/Claves/Legacy/claves_ff.wav", "Claves, loud"),
        ("vcsl", "Idiophones/Struck Idiophones/Shaker, Large/LShaker_Shake1D_rr2_Mid.wav", "Large shaker, shake down"),
    ],
}

KIT_NOTES = {
    "acoustic": "A kit of real drums and percussion: every voice is a recording from the Versilian"
                " Community Sample Library.",
    "808": "A drum machine kit, long and dark. The kick, snare, hats and electro voices are CC0"
           " freesound one-shots collected by Sonic Pi; the ride, congas, claves and shaker are"
           " Versilian recordings.",
    "909": "A drum machine kit, brighter and tighter than `808`. The kick, snare, hats and cymbals"
           " are CC0 freesound one-shots collected by Sonic Pi; the toms, cowbell and percussion are"
           " Versilian recordings.",
}


def url(source, path):
    if source == "sp":
        return SONIC_PI.format(name=path)
    return VCSL.format(path=path.replace(" ", "%20"))


def licence(source):
    return SONIC_PI_LICENCE if source == "sp" else VCSL_LICENCE


def fetch(source, path, cache):
    local = os.path.join(cache, source + "-" + path.replace("/", "_"))
    if os.path.exists(local) and os.path.getsize(local) > 0:
        return local
    subprocess.run(["curl", "-sfL", "--max-time", "120", "-o", local, url(source, path)],
                   check=True)
    if os.path.getsize(local) == 0:
        raise SystemExit(f"empty download: {path}")
    return local


def samples_of(local):
    """Decode anything ffmpeg knows into mono float32 at `TARGET_RATE`."""
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", local, "-ac", "1", "-ar", str(TARGET_RATE),
         "-f", "f32le", "-"],
        capture_output=True, check=True).stdout
    return [value for (value,) in struct.iter_unpack("<f", raw[:len(raw) // 4 * 4])]


def shape(samples):
    # Trim the leading silence below -60 dBFS, leaving 1 ms of it, so a hit lands on time.
    floor = 10 ** (-60 / 20)
    start = 0
    while start < len(samples) and abs(samples[start]) < floor:
        start += 1
    start = max(0, start - TARGET_RATE // 1000)
    samples = samples[start:start + int(MAX_SECONDS * TARGET_RATE)]
    peak = max((abs(value) for value in samples), default=0.0)
    if peak <= 0.0:
        raise SystemExit("a silent sample")
    samples = [value * (PEAK / peak) for value in samples]
    fade = min(len(samples), int(FADE * TARGET_RATE))
    for index in range(fade):
        samples[-1 - index] *= index / fade
    return samples


def write_wav(path, samples):
    frames = bytearray()
    for value in samples:
        frames += struct.pack("<h", int(round(max(-1.0, min(1.0, value)) * 32767)))
    with wave.open(path, "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(TARGET_RATE)
        out.writeframes(bytes(frames))


def crossing_rate(samples):
    """Zero crossings a second: a gross check that a file is the sort of sound its name says."""
    crossings = sum(1 for a, b in zip(samples, samples[1:], strict=False) if (a < 0) != (b < 0))
    return crossings / 2.0 / max(len(samples) / TARGET_RATE, 1e-9)


def credits(manifest):
    lines = [
        "# The factory kits' samples",
        "",
        "Every WAV in this folder is **CC0 (public domain dedication)**. Nothing here is merely"
        " \"royalty-free\": a bundled sample is redistributed inside the app, which is a different"
        " right, so nothing goes in this folder that does not say CC0 in its own source's words.",
        "",
        "Two collections supply every file:",
        "",
        "- **Sonic Pi's sample set**, mirrored at <https://github.com/smpldsnds/sonic-pi-samples>."
        " Its README states that every sample is CC0 and links the freesound page each one came from;"
        " the `source` column below is that page.",
        "- **The Versilian Community Sample Library (VCSL)**, <https://github.com/sgossner/vcsl>."
        " Its README dedicates the whole library to CC0.",
        "",
        "The TicTokMen Retro Drums set was considered and **not used**: its README says its"
        " recordings are under \"free use licenses\" without naming a licence, and an unverifiable"
        " claim is exactly what this page exists to prevent.",
        "",
        "## What was done to them",
        "",
        "Each file was decoded, folded to one channel, resampled to 44.1 kHz, trimmed to at most"
        " 0.6 s with a 10 ms fade at the tail, and peak-normalised. Nothing else: no EQ, no"
        " compression, no pitch shift. What makes an `808` kick sound like an `808` rather than like"
        " the recording it came from is the kit's own per-voice defaults, not the file.",
        "",
        "`refresh.py` beside this page is the script that did it, and this page is generated from"
        " the same table it fetched with — so a credit and the bytes beside it cannot disagree.",
        "",
    ]
    for kit in ["acoustic", "808", "909"]:
        lines += [
            f"## `{kit}`",
            "",
            KIT_NOTES[kit],
            "",
            "| file | voice | recording | source | licence |",
            "| --- | --- | --- | --- | --- |",
        ]
        for voice in VOICES:
            entry = manifest[kit][voice]
            lines.append(f"| `{kit}/{voice}.wav` | {voice} | {entry['what']} "
                         f"| <{entry['url']}> | {entry['licence']} |")
        lines.append("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--measure-only", action="store_true",
                        help="decode and report, without writing any WAV")
    parser.add_argument("--cache", default=os.path.join(tempfile.gettempdir(), "takka-kits"),
                        help="where the downloads are kept between runs")
    args = parser.parse_args()
    os.makedirs(args.cache, exist_ok=True)

    manifest = {}
    for kit, voices in KITS.items():
        if len(voices) != len(VOICES):
            raise SystemExit(f"{kit} has {len(voices)} voices, not {len(VOICES)}")
        folder = os.path.join(OUT, kit)
        if not args.measure_only:
            os.makedirs(folder, exist_ok=True)
        for voice, (source, path, what) in zip(VOICES, voices, strict=True):
            samples = shape(samples_of(fetch(source, path, args.cache)))
            if not args.measure_only:
                write_wav(os.path.join(folder, voice + ".wav"), samples)
            print(f"{kit:9s} {voice:5s} {len(samples) / TARGET_RATE:5.3f}s "
                  f"{len(samples) * 2 // 1024:4d}K ~{crossing_rate(samples):6.0f}Hz  {what}")
            manifest.setdefault(kit, {})[voice] = {
                "voice": voice,
                "source": source,
                "path": path,
                "url": url(source, path),
                "licence": licence(source),
                "what": what,
            }

    if args.measure_only:
        return
    with open(os.path.join(OUT, "CREDITS.md"), "w") as handle:
        handle.write(credits(manifest) + "\n")
    total = sum(os.path.getsize(os.path.join(OUT, kit, voice + ".wav"))
                for kit in KITS for voice in VOICES)
    print(f"\ntotal {total / 1024 / 1024:.2f} MiB across {len(KITS) * len(VOICES)} voices")


if __name__ == "__main__":
    sys.exit(main())
