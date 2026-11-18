# The factory kits' samples

Every WAV in this folder is **CC0 (public domain dedication)**. Nothing here is merely "royalty-free": a bundled sample is redistributed inside the app, which is a different right, so nothing goes in this folder that does not say CC0 in its own source's words.

Two collections supply every file:

- **Sonic Pi's sample set**, mirrored at <https://github.com/smpldsnds/sonic-pi-samples>. Its README states that every sample is CC0 and links the freesound page each one came from; the `source` column below is that page.
- **The Versilian Community Sample Library (VCSL)**, <https://github.com/sgossner/vcsl>. Its README dedicates the whole library to CC0.

The TicTokMen Retro Drums set was considered and **not used**: its README says its recordings are under "free use licenses" without naming a licence, and an unverifiable claim is exactly what this page exists to prevent.

## What was done to them

Each file was decoded, folded to one channel, resampled to 44.1 kHz, trimmed to at most 0.6 s with a 10 ms fade at the tail, and peak-normalised. Nothing else: no EQ, no compression, no pitch shift. What makes an `808` kick sound like an `808` rather than like the recording it came from is the kit's own per-voice defaults, not the file.

`refresh.py` beside this page is the script that did it, and this page is generated from the same table it fetched with — so a credit and the bytes beside it cannot disagree.

## `acoustic`

A kit of real drums and percussion: every voice is a recording from the Versilian Community Sample Library.

| file | voice | recording | source | licence |
| --- | --- | --- | --- | --- |
| `acoustic/KICK.wav` | KICK | Bass Drum 1, hard hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Bass%20Drum%201/BDrumNew_hit_v5_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/SNR.wav` | SNR | Snare Drum, Modern 1, stick hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Snare%20Drum,%20Modern%201/Snare2_HitSN_v5_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/RIM.wav` | RIM | Tom 1, rim shot | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Tom%201/TomH_rimS_v2_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/CLAP.wav` | CLAP | Claps, single | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Claps/Clap_rr1.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/CHH.wav` | CHH | Hi-hat, closed | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Hi-Hat%20Cymbal/HiHat_HitC_v2_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/OHH.wav` | OHH | Hi-hat, open | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Hi-Hat%20Cymbal/HiHat_HitO_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/LOT.wav` | LOT | Tom 2, low, stick hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Tom%202/Stick/TomL_HitS_v3_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/MID.wav` | MID | Legacy tenor tom, lower, medium | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Legacy%20Toms/tenor_lower/tenor_mf_rr1.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/HTOM.wav` | HTOM | Tom 1, high, stick hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Tom%201/Stick/TomH_HitS_v3_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/CRS.wav` | CRS | Suspended cymbal, hardest hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Suspended%20Cymbal%201/susCymb1_hit_fff1.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/RDE.wav` | RDE | Suspended cymbal, on the bell | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Suspended%20Cymbal%201/susCymb1_hit_bell_mf1.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/CWB.wav` | CWB | Cowbell 1, normal | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Cowbells/Cowbell1_Normal_v2_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/LCO.wav` | LCO | Conga, tumba (low), open tone | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Conga/Tumba_HitN_v2_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/HCO.wav` | HCO | Conga, quinto (high), open tone | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Conga/Quinto_HitN_v2_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/CLV.wav` | CLV | Claves 1, hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Claves/Claves1_Hit_v2_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `acoustic/SHK.wav` | SHK | Small shaker, slap | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Shaker,%20Small/Mid_Shaker_Slap_rr1.wav> | CC0 (Versilian Community Sample Library) |

## `808`

A drum machine kit, long and dark. The kick, snare, hats and electro voices are CC0 freesound one-shots collected by Sonic Pi; the ride, congas, claves and shaker are Versilian recordings.

| file | voice | recording | source | licence |
| --- | --- | --- | --- | --- |
| `808/KICK.wav` | KICK | 808 bass drum | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/bd_808.ogg> | CC0 (Sonic Pi's sample set) |
| `808/SNR.wav` | SNR | snare drum | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/sn_dolf.ogg> | CC0 (Sonic Pi's sample set) |
| `808/RIM.wav` | RIM | electro tick | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/elec_tick.ogg> | CC0 (Sonic Pi's sample set) |
| `808/CLAP.wav` | CLAP | hand clap | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/perc_snap2.ogg> | CC0 (Sonic Pi's sample set) |
| `808/CHH.wav` | CHH | hi-hat | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/hat_cats.ogg> | CC0 (Sonic Pi's sample set) |
| `808/OHH.wav` | OHH | hi-hat | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/hat_zild.ogg> | CC0 (Sonic Pi's sample set) |
| `808/LOT.wav` | LOT | electro bongo | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/elec_bong.ogg> | CC0 (Sonic Pi's sample set) |
| `808/MID.wav` | MID | electro wood block | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/elec_wood.ogg> | CC0 (Sonic Pi's sample set) |
| `808/HTOM.wav` | HTOM | electro fuzz tom | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/elec_fuzz_tom.ogg> | CC0 (Sonic Pi's sample set) |
| `808/CRS.wav` | CRS | electro cymbal | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/elec_cymbal.ogg> | CC0 (Sonic Pi's sample set) |
| `808/RDE.wav` | RDE | Suspended cymbal, stick, medium | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Suspended%20Cymbal%201/susCymb1_hit_stick_mp1.wav> | CC0 (Versilian Community Sample Library) |
| `808/CWB.wav` | CWB | cowbell | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/drum_cowbell.ogg> | CC0 (Sonic Pi's sample set) |
| `808/LCO.wav` | LCO | Conga, tumba (low), muted tone | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Conga/Tumba_HitFM_v2_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `808/HCO.wav` | HCO | Conga, quinto (high), muted tone | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Conga/Quinto_HitFM1_v2_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `808/CLV.wav` | CLV | Claves 2, hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Claves/Claves2_Hit_v2_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `808/SHK.wav` | SHK | Large shaker, hit | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Shaker,%20Large/LShaker_Hit_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |

## `909`

A drum machine kit, brighter and tighter than `808`. The kick, snare, hats and cymbals are CC0 freesound one-shots collected by Sonic Pi; the toms, cowbell and percussion are Versilian recordings.

| file | voice | recording | source | licence |
| --- | --- | --- | --- | --- |
| `909/KICK.wav` | KICK | haus bass drum | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/bd_haus.ogg> | CC0 (Sonic Pi's sample set) |
| `909/SNR.wav` | SNR | snare drum | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/sn_generic.ogg> | CC0 (Sonic Pi's sample set) |
| `909/RIM.wav` | RIM | electro pop | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/elec_pop.ogg> | CC0 (Sonic Pi's sample set) |
| `909/CLAP.wav` | CLAP | hand clap | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/perc_snap.ogg> | CC0 (Sonic Pi's sample set) |
| `909/CHH.wav` | CHH | hi-hat | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/hat_tap.ogg> | CC0 (Sonic Pi's sample set) |
| `909/OHH.wav` | OHH | hi-hat | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/hat_psych.ogg> | CC0 (Sonic Pi's sample set) |
| `909/LOT.wav` | LOT | Tom 2, low, mallet | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Tom%202/Mallet/TomL_HitM_v3_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `909/MID.wav` | MID | Tom 1, high, mallet | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Tom%201/Mallet/TomH_HitM_v3_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `909/HTOM.wav` | HTOM | Legacy tenor tom, higher, medium | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Legacy%20Toms/tenor_higher/tenorH_mf_rr1.wav> | CC0 (Versilian Community Sample Library) |
| `909/CRS.wav` | CRS | cymbal, hard | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/drum_cymbal_hard.ogg> | CC0 (Sonic Pi's sample set) |
| `909/RDE.wav` | RDE | cymbal, soft | <https://raw.githubusercontent.com/smpldsnds/sonic-pi-samples/main/samples/drum_cymbal_soft.ogg> | CC0 (Sonic Pi's sample set) |
| `909/CWB.wav` | CWB | Cowbell 2, normal | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Cowbells/Cowbell2_Normal_v2_rr1_Mid.wav> | CC0 (Versilian Community Sample Library) |
| `909/LCO.wav` | LCO | Conga, tumba (low), open tone | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Conga/Tumba_HitN_v3_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `909/HCO.wav` | HCO | Conga, open tone | <https://raw.githubusercontent.com/sgossner/vcsl/master/Membranophones/Struck%20Membranophones/Conga/Conga_HitN_v2_rr1_Sum.wav> | CC0 (Versilian Community Sample Library) |
| `909/CLV.wav` | CLV | Claves, loud | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Claves/Legacy/claves_ff.wav> | CC0 (Versilian Community Sample Library) |
| `909/SHK.wav` | SHK | Large shaker, shake down | <https://raw.githubusercontent.com/sgossner/vcsl/master/Idiophones/Struck%20Idiophones/Shaker,%20Large/LShaker_Shake1D_rr2_Mid.wav> | CC0 (Versilian Community Sample Library) |

