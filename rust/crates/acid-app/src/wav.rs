//! A hand-rolled RIFF/WAVE decoder: a kit's samples, read once when a kit loads.
//!
//! `std` only. The round's Tech Stack forbids a new dependency, and the whole of what a drum kit needs is a header walk and
//! four sample widths — no compression, no metadata, no streaming. Everything here runs on the UI thread while a kit is
//! being loaded, never on the audio callback: `decode` allocates here, on load, and the engine is handed the finished
//! `Arc<[f32]>`.
//!
//! The output is one channel at the engine's rate, because that is what a [`KitVoice`](acid_core::drums::KitVoice) holds: a
//! stereo file is folded down and any rate is resampled to the engine's, so the renderer never resamples per block and never
//! has to ask a sample what rate it came from.

use std::fmt;
use std::sync::Arc;

/// The longest sample one voice may hold, in samples.
///
/// The cap is in samples rather than bytes so that it does not move with the file's width, and it is checked against the
/// *output* length — before either the mono fold or the resampled `Vec` is allocated — so a file whose header claims hours
/// of audio is refused before this process reserves the memory for it. At 48 kHz it is 21.8 seconds, and the longest voice
/// the drum machine's spec allows is 0.6 s, so the only file it refuses is one that is not a drum sample at all.
const MAX_SAMPLES: usize = 1 << 20;

/// Why a file could not be decoded.
///
/// This is the interface task 6 reports a bad kit sample through, so every variant's [`Display`](std::fmt::Display) is the
/// words a player reads beside the voice and the file name. They are fixed strings: a notice is not the place to build
/// sentences out of numbers.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WavError {
    /// The file does not start with `RIFF`.
    NotRiff,
    /// A `RIFF` file that is something other than a `WAVE` — an AVI, an IFF, a renamed file.
    NotWave,
    /// A WAVE this decoder does not read, with `what` naming it in words a player recognises: a compression (`ADPCM`), a
    /// channel count, a sample width. A format is named rather than numbered, so the notice means something to the person
    /// holding the file.
    Unsupported { what: &'static str },
    /// The file ends before a chunk it declared does: cut while copying, or half-written.
    Truncated,
    /// The sample is longer than the machine's cap, and would not be a drum voice if it were decoded.
    TooLong,
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRiff => f.write_str("not a RIFF file"),
            Self::NotWave => f.write_str("not a WAVE file"),
            Self::Unsupported { what } => write!(f, "unsupported ({what})"),
            Self::Truncated => f.write_str("the file ends inside a chunk it declared"),
            Self::TooLong => f.write_str("longer than the sample limit"),
        }
    }
}

impl std::error::Error for WavError {}

/// Decode a WAV file's samples: mono, at `target_rate`.
///
/// Accepts PCM in 8, 16, 24 or 32 bits and 32-bit IEEE float, one or two channels, and resamples linearly to `target_rate`.
/// Chunks this decoder does not know are skipped, including the `LIST`/`INFO` block most encoders write before the samples.
pub fn decode(bytes: &[u8], target_rate: f32) -> Result<Arc<[f32]>, WavError> {
    // A rate this decoder cannot divide by is the caller's mistake, not the file's, and refusing it here keeps the notice
    // task 6 prints in one vocabulary. `is_finite` also catches a NaN rate, which `<= 0.0` alone would let through.
    if !target_rate.is_finite() || target_rate <= 0.0 {
        return Err(WavError::Unsupported {
            what: "a target rate of zero",
        });
    }
    if !bytes.starts_with(b"RIFF") {
        return Err(WavError::NotRiff);
    }
    if take(bytes, 8, 4)? != b"WAVE" {
        return Err(WavError::NotWave);
    }

    // Walk the chunks in order. A `LIST` or `INFO` before `data` is normal, so anything unrecognised is stepped over rather
    // than refused; an odd size is followed by a pad byte the size does not count. The RIFF header's own size is ignored:
    // files in the wild disagree with it, and `bytes` is the truth about how much file there is.
    let mut format = None;
    let mut data = None;
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = take(bytes, at, 4)?;
        let size = u32_at(bytes, at + 4)? as usize;
        let body = take(bytes, at + 8, size)?;
        match id {
            b"fmt " => format = Some(Format::of(body)?),
            b"data" => data = Some(body),
            _ => {}
        }
        at = at + 8 + size + (size & 1);
    }
    let format = format.ok_or(WavError::Unsupported {
        what: "no format chunk",
    })?;
    // A `data` chunk that is present and empty is a voice with no sample, which a kit may hold; one that is absent is a
    // file this decoder cannot read. Keeping those apart is what stops a half-written file loading as a silent voice.
    let data = data.ok_or(WavError::Unsupported {
        what: "no sample data",
    })?;

    let frames = data.len() / format.frame();
    let out_len = output_length(frames, format.rate, target_rate)?;
    let mono = format.fold(data, frames)?;
    Ok(resample(&mono, format.rate, target_rate, out_len).into())
}

/// One sample's width and how to read it.
///
/// The four integer widths are scaled into `-1.0..=1.0`. A float file's numbers are passed through as recorded, so a file that
/// ran hot stays hot: clamping or refusing it is a decision for the renderer that plays it, not for a decoder.
#[derive(Copy, Clone)]
enum Sample {
    /// Unsigned, with 128 as zero: the one width that is not two's complement.
    U8,
    I16,
    I24,
    I32,
    /// 32-bit IEEE float — the same width as `I32` and not the same meaning, so the `fmt ` tag decides which this is.
    F32,
}

impl Sample {
    /// Which sample a `fmt ` chunk's format tag and width mean.
    ///
    /// The tag is read before the width, so `ADPCM` at four bits is refused by its compression's name rather than as a odd
    /// sample width. Every refusal is a `&'static str`, so the notice cannot be built out of a number a player cannot use.
    fn of(tag: u16, bits: u16) -> Result<Self, WavError> {
        match tag {
            1 => match bits {
                8 => Ok(Self::U8),
                16 => Ok(Self::I16),
                24 => Ok(Self::I24),
                32 => Ok(Self::I32),
                _ => Err(WavError::Unsupported {
                    what: "a sample width that is not 8, 16, 24 or 32 bits",
                }),
            },
            3 => match bits {
                32 => Ok(Self::F32),
                _ => Err(WavError::Unsupported {
                    what: "a floating-point width that is not 32 bits",
                }),
            },
            // The two ADPCM tags are the same compression family, and the names are the ones a player will have seen on the
            // file they were given. `WAVE_FORMAT_EXTENSIBLE` usually wraps plain PCM in a sub-format field this decoder does
            // not read, so it is refused by name rather than guessed at.
            2 | 0x11 => Err(WavError::Unsupported { what: "ADPCM" }),
            6 => Err(WavError::Unsupported { what: "A-law" }),
            7 => Err(WavError::Unsupported { what: "mu-law" }),
            0xFFFE => Err(WavError::Unsupported {
                what: "WAVE_FORMAT_EXTENSIBLE",
            }),
            _ => Err(WavError::Unsupported {
                what: "a format tag this decoder does not know",
            }),
        }
    }

    /// The bytes one sample takes.
    fn width(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::I16 => 2,
            Self::I24 => 3,
            Self::I32 | Self::F32 => 4,
        }
    }

    /// One sample at `at`: an integer width scaled into `-1.0..=1.0`, a float one as recorded.
    fn at(self, bytes: &[u8], at: usize) -> Result<f32, WavError> {
        Ok(match self {
            Self::U8 => (take(bytes, at, 1)?[0] as f32 - 128.0) / 128.0,
            Self::I16 => f32::from(i16::from_le_bytes(exact(take(bytes, at, 2)?)?)) / 32_768.0,
            // Sign extension: left in the top three bytes of an `i32` by `from_le_bytes`, shifted into bit 31 and back, so
            // the arithmetic shift repeats the sign. Without it `-0.5`'s `0xC00000` reads as the positive `1.5`.
            Self::I24 => {
                let b = exact::<3>(take(bytes, at, 3)?)?;
                (i32::from_le_bytes([b[0], b[1], b[2], 0]) << 8 >> 8) as f32 / 8_388_608.0
            }
            Self::I32 => i32::from_le_bytes(exact(take(bytes, at, 4)?)?) as f32 / 2_147_483_648.0,
            Self::F32 => f32::from_le_bytes(exact(take(bytes, at, 4)?)?),
        })
    }
}

/// How many channels a frame carries. One or two, decided once so the fold is a two-armed match and not a `_` arm.
#[derive(Copy, Clone)]
enum Channels {
    Mono,
    Stereo,
}

impl Channels {
    fn of(count: u16) -> Result<Self, WavError> {
        match count {
            1 => Ok(Self::Mono),
            2 => Ok(Self::Stereo),
            _ => Err(WavError::Unsupported {
                what: "not mono or stereo",
            }),
        }
    }

    fn count(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

/// What a `fmt ` chunk says: how each sample is read, how many there are in a frame, and the rate they were recorded at.
#[derive(Copy, Clone)]
struct Format {
    sample: Sample,
    channels: Channels,
    rate: f32,
}

impl Format {
    /// Read a `fmt ` chunk's body.
    ///
    /// The `cbSize` extension bytes some encoders append are ignored, not misread: the first sixteen bytes are all this
    /// decoder needs, and a longer chunk is normal.
    fn of(body: &[u8]) -> Result<Self, WavError> {
        if body.len() < 16 {
            return Err(WavError::Unsupported {
                what: "a short `fmt ` chunk",
            });
        }
        let format = Self {
            sample: Sample::of(u16_at(body, 0)?, u16_at(body, 14)?)?,
            channels: Channels::of(u16_at(body, 2)?)?,
            rate: u32_at(body, 4)? as f32,
        };
        if format.rate == 0.0 {
            return Err(WavError::Unsupported {
                what: "a sample rate of zero",
            });
        }
        // A `fmt ` chunk that describes a frame wider than its samples would put every frame after the first one out of
        // step, which sounds like noise rather than like a file that failed to load. Zero means "not filled in", which some
        // encoders write and which the channels and width already answer.
        let align = u16_at(body, 12)?;
        if align != 0 && usize::from(align) != format.frame() {
            return Err(WavError::Unsupported {
                what: "a frame that is not packed",
            });
        }
        Ok(format)
    }

    /// The bytes one frame takes across all its channels.
    fn frame(self) -> usize {
        self.channels.count() * self.sample.width()
    }

    /// The samples as one channel: a mono file as it stands, a stereo file averaged.
    ///
    /// Averaged, not summed. Summing a correlated stereo pair doubles it, and a kit that clipped its own samples could not be
    /// fixed by the mixer's ceiling later.
    fn fold(self, data: &[u8], frames: usize) -> Result<Vec<f32>, WavError> {
        let frame = self.frame();
        let width = self.sample.width();
        let mut mono = Vec::with_capacity(frames);
        for index in 0..frames {
            let at = index * frame;
            let left = self.sample.at(data, at)?;
            mono.push(match self.channels {
                Channels::Mono => left,
                Channels::Stereo => (left + self.sample.at(data, at + width)?) * 0.5,
            });
        }
        Ok(mono)
    }
}

/// `bytes[at..at + n]`, or [`WavError::Truncated`] when the file ends first.
///
/// Every slice in this module is taken through here: a file is bytes that arrived from somewhere else, so reading past the
/// end is a refusal to report, never a panic to recover from.
fn take(bytes: &[u8], at: usize, n: usize) -> Result<&[u8], WavError> {
    let end = at.checked_add(n).ok_or(WavError::Truncated)?;
    bytes.get(at..end).ok_or(WavError::Truncated)
}

/// The same, for a fixed width: `from_le_bytes` needs an array, and a checked slice is the only way to make one that cannot
/// panic on a short file.
fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N], WavError> {
    <[u8; N]>::try_from(bytes).map_err(|_| WavError::Truncated)
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, WavError> {
    Ok(u16::from_le_bytes(exact(take(bytes, at, 2)?)?))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, WavError> {
    Ok(u32::from_le_bytes(exact(take(bytes, at, 4)?)?))
}

/// How many samples `frames` at `rate` become at `target`, rounded rather than grown.
///
/// The length is computed, not discovered: it is what tells a file that is not a drum sample from one that is, before the
/// output `Vec` exists. `f64` because the product of a long file and a rate ratio overflows an `f32` mantissa's worth of
/// significant digits long before it overflows `usize`.
fn output_length(frames: usize, rate: f32, target: f32) -> Result<usize, WavError> {
    let scaled = frames as f64 * f64::from(target) / f64::from(rate);
    if !scaled.is_finite() || scaled > MAX_SAMPLES as f64 {
        return Err(WavError::TooLong);
    }
    Ok(scaled.round() as usize)
}

/// Resample linearly to exactly `out_len` samples, reading between the source's samples and clamping at its end.
///
/// `out_len` comes from [`output_length`], so it is zero when there are no frames and the loop below — which would otherwise
/// be the one place a sample index could reach an empty slice — does not run.
fn resample(mono: &[f32], rate: f32, target: f32, out_len: usize) -> Vec<f32> {
    debug_assert!(
        out_len == 0 || !mono.is_empty(),
        "out_len comes from output_length, so it is zero when there are no frames"
    );
    let step = f64::from(rate) / f64::from(target);
    let last = mono.len().saturating_sub(1);
    let mut out = Vec::with_capacity(out_len);
    for index in 0..out_len {
        let position = index as f64 * step;
        let whole = position.floor();
        let frac = (position - whole) as f32;
        let here = (whole as usize).min(last);
        let next = (whole as usize + 1).min(last);
        out.push(mono[here] + (mono[next] - mono[here]) * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rate the engine runs at in these tests, so a file already at it is not resampled.
    const RATE: f32 = 48_000.0;

    /// One RIFF chunk: a four-byte id, a little-endian size, the body, and then the pad byte an odd size forces — a pad
    /// the size does not count, which is why a walker that forgets it lands mid-chunk on the next one.
    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + body.len() + 1);
        out.extend_from_slice(id);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// A whole file: `RIFF`, its size, `WAVE`, then the chunks in order.
    fn riff(parts: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = parts.concat();
        let mut out = Vec::with_capacity(12 + body.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((4 + body.len()) as u32).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(&body);
        out
    }

    /// A `fmt ` chunk whose block alignment is what its channels and width imply.
    fn fmt_chunk(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
        fmt_chunk_with_align(tag, channels, rate, bits, channels * bits / 8)
    }

    /// The sixteen bytes of a `fmt ` chunk's body.
    fn fmt_body(tag: u16, channels: u16, rate: u32, bits: u16, align: u16) -> Vec<u8> {
        let mut body = Vec::with_capacity(16);
        body.extend_from_slice(&tag.to_le_bytes());
        body.extend_from_slice(&channels.to_le_bytes());
        body.extend_from_slice(&rate.to_le_bytes());
        body.extend_from_slice(&(rate * u32::from(align)).to_le_bytes());
        body.extend_from_slice(&align.to_le_bytes());
        body.extend_from_slice(&bits.to_le_bytes());
        body
    }

    /// A `fmt ` chunk that says whatever a test wants, including a block alignment that lies about the frame.
    fn fmt_chunk_with_align(tag: u16, channels: u16, rate: u32, bits: u16, align: u16) -> Vec<u8> {
        chunk(b"fmt ", &fmt_body(tag, channels, rate, bits, align))
    }

    /// A `fmt ` chunk with the `cbSize` extension a DAW writes: eighteen bytes, of which the last two are not read.
    fn fmt_chunk_with_extension(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
        let mut body = fmt_body(tag, channels, rate, bits, channels * bits / 8);
        body.extend_from_slice(&0u16.to_le_bytes());
        chunk(b"fmt ", &body)
    }

    fn data_chunk(bytes: &[u8]) -> Vec<u8> {
        chunk(b"data", bytes)
    }

    /// `fmt ` then `data`: the shape every normal file arrives in.
    fn pcm(tag: u16, channels: u16, rate: u32, bits: u16, bytes: &[u8]) -> Vec<u8> {
        riff(&[fmt_chunk(tag, channels, rate, bits), data_chunk(bytes)])
    }

    fn pcm16(samples: &[i16], channels: u16) -> Vec<u8> {
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        pcm(1, channels, RATE as u32, 16, &bytes)
    }

    fn pcm8(samples: &[u8]) -> Vec<u8> {
        pcm(1, 1, RATE as u32, 8, samples)
    }

    fn pcm24(samples: &[i32]) -> Vec<u8> {
        let bytes: Vec<u8> = samples
            .iter()
            .flat_map(|s| s.to_le_bytes()[..3].to_vec())
            .collect();
        pcm(1, 1, RATE as u32, 24, &bytes)
    }

    fn pcm32(samples: &[i32]) -> Vec<u8> {
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        pcm(1, 1, RATE as u32, 32, &bytes)
    }

    /// 32-bit float: tag 3, so the width and the meaning are told apart by the format tag and not by the size.
    fn float32(samples: &[f32]) -> Vec<u8> {
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        pcm(3, 1, RATE as u32, 32, &bytes)
    }

    #[test]
    fn a_fmt_chunk_with_an_extension_decodes() {
        // The shape a WAV that has been through a DAW arrives in: `fmt ` is 18 bytes, with a `cbSize` word this decoder does
        // not read. A `fmt ` chunk that insists on being exactly 16 refuses most real sample packs.
        let bytes = riff(&[
            fmt_chunk_with_extension(1, 1, RATE as u32, 16),
            data_chunk(&[0x00, 0x40, 0x00, 0xC0]),
        ]);
        let decoded = decode(&bytes, RATE).expect("an extended fmt chunk is still a fmt chunk");
        let expected: [f32; 2] = [0.5, -0.5];
        assert_eq!(decoded.as_ref(), &expected[..]);
    }

    #[test]
    fn a_riff_size_that_lies_about_the_length_is_ignored() {
        // Encoders have written wrong RIFF sizes for thirty years, so the file that arrived is the truth about how much
        // file there is. This one claims four bytes follow the form type.
        let mut bytes = pcm16(&[16_384, -16_384], 1);
        bytes[4..8].copy_from_slice(&4u32.to_le_bytes());
        let decoded =
            decode(&bytes, RATE).expect("a wrong RIFF size does not stop a file decoding");
        let expected: [f32; 2] = [0.5, -0.5];
        assert_eq!(decoded.as_ref(), &expected[..]);
    }

    #[test]
    fn a_sixteen_bit_mono_file_decodes_to_the_same_samples() {
        let decoded = decode(&pcm16(&[0, 16_384, -16_384, 32_767, -32_768], 1), RATE)
            .expect("a plain 16-bit file decodes");
        let expected: [f32; 5] = [0.0, 0.5, -0.5, 32_767.0 / 32_768.0, -1.0];
        assert_eq!(
            decoded.as_ref(),
            &expected[..],
            "a 16-bit sample is its number over 2^15, sign kept"
        );
    }

    #[test]
    fn a_stereo_file_is_folded_to_mono() {
        // Two frames of `(l, r)`. A correlated pair is the tell: summing puts the first frame at 1.0, averaging at 0.5.
        let decoded = decode(
            &pcm16(&[16_384, 16_384, 16_384, -16_384, 8_192, 0], 2),
            RATE,
        )
        .expect("a stereo file decodes");
        let expected: [f32; 3] = [0.5, 0.0, 0.125];
        assert_eq!(
            decoded.as_ref(),
            &expected[..],
            "the channels are averaged, not summed"
        );
        assert_eq!(
            decoded.len(),
            3,
            "two channels are one frame, not two samples"
        );
    }

    #[test]
    fn a_forty_four_one_file_is_resampled_to_forty_eight() {
        // A ramp, so a linear resampler can be told from a nearest-neighbour one: the output of a ramp rises at every step
        // when it is interpolated, and holds a value for two outputs and then jumps when it is decimated.
        let ramp: Vec<u8> = (0..100i32)
            .flat_map(|i| ((i * 32_767 / 99) as i16).to_le_bytes())
            .collect();
        let bytes = pcm(1, 1, 44_100, 16, &ramp);
        let decoded = decode(&bytes, RATE).expect("a 44.1 kHz file decodes at 48 kHz");
        assert_eq!(
            decoded.len(),
            109,
            "100 frames at 44.1 kHz are 48000/44100 as many samples at 48 kHz"
        );
        assert!(
            decoded.windows(2).all(|pair| pair[1] > pair[0]),
            "a ramp stays a ramp: every output sample is above the one before it"
        );
        assert!(
            (decoded[0] - 0.0).abs() < 1e-6,
            "the first sample is the file's first"
        );
        assert!(
            (decoded[108] - 32_767.0 / 32_768.0).abs() < 1e-4,
            "the peak survives the rate change: {}",
            decoded[108]
        );
    }

    #[test]
    fn a_ninety_six_file_is_resampled_down_to_forty_eight() {
        // The other direction: a 96 kHz sample pack into a 48 kHz engine. The length halves, and the ramp still reaches its
        // peak — so a rate ratio the wrong way up is caught rather than a length that happens to match.
        let ramp: Vec<u8> = (0..100i32)
            .flat_map(|i| ((i * 32_767 / 99) as i16).to_le_bytes())
            .collect();
        let bytes = pcm(1, 1, 96_000, 16, &ramp);
        let decoded = decode(&bytes, RATE).expect("a 96 kHz file decodes at 48 kHz");
        assert_eq!(
            decoded.len(),
            50,
            "100 frames at 96 kHz are half as many samples at 48 kHz"
        );
        assert!(
            decoded.windows(2).all(|pair| pair[1] > pair[0]),
            "every other sample of the ramp, still rising"
        );
        assert!(
            (decoded[49] - 32_767.0 / 32_768.0).abs() < 0.02,
            "the peak survives the halving: {}",
            decoded[49]
        );
    }

    #[test]
    fn a_float_file_decodes() {
        let decoded =
            decode(&float32(&[0.25, -0.5, 0.0, 0.999]), RATE).expect("a float file decodes");
        let expected: [f32; 4] = [0.25, -0.5, 0.0, 0.999];
        assert_eq!(decoded.as_ref(), &expected[..]);
    }

    #[test]
    fn eight_and_twenty_four_bit_decode() {
        // 8-bit PCM is unsigned with 128 as zero, which is the one width that is not two's complement.
        let eight = decode(&pcm8(&[0, 128, 255]), RATE).expect("an 8-bit file decodes");
        let expected: [f32; 3] = [-1.0, 0.0, 127.0 / 128.0];
        assert_eq!(
            eight.as_ref(),
            &expected[..],
            "an 8-bit sample is (byte - 128) / 128"
        );

        // 24-bit is signed: -0.5 is 0xC00000, and a reader that does not sign-extend reads it as 1.5.
        let twenty_four = decode(
            &pcm24(&[0, 4_194_304, -4_194_304, 8_388_607, -8_388_608]),
            RATE,
        )
        .expect("a 24-bit file decodes");
        let expected: [f32; 5] = [0.0, 0.5, -0.5, 8_388_607.0 / 8_388_608.0, -1.0];
        assert_eq!(
            twenty_four.as_ref(),
            &expected[..],
            "a 24-bit sample is sign-extended"
        );
    }

    #[test]
    fn a_thirty_two_bit_integer_file_decodes_as_samples() {
        let decoded = decode(&pcm32(&[0, 1 << 30, -(1 << 30), i32::MAX, i32::MIN]), RATE)
            .expect("a 32-bit integer file decodes");
        let expected: [f32; 5] = [0.0, 0.5, -0.5, 1.0, -1.0];
        assert_eq!(
            decoded.as_ref(),
            &expected[..],
            "32-bit integers are samples, and the fmt tag is what says so"
        );
    }

    #[test]
    fn adpcm_is_refused_by_name() {
        let bytes = pcm(2, 1, RATE as u32, 4, &[0; 8]);
        let error = decode(&bytes, RATE).expect_err("ADPCM is a compression, not a sample width");
        assert_eq!(error, WavError::Unsupported { what: "ADPCM" });
        assert!(
            error.to_string().contains("ADPCM"),
            "the notice names the format: {error}"
        );
    }

    #[test]
    fn every_format_but_pcm_and_float_is_refused_by_name() {
        for (tag, what) in [
            (6, "A-law"),
            (7, "mu-law"),
            (0xFFFE, "WAVE_FORMAT_EXTENSIBLE"),
        ] {
            let bytes = pcm(tag, 1, RATE as u32, 16, &[0; 4]);
            assert_eq!(
                decode(&bytes, RATE),
                Err(WavError::Unsupported { what }),
                "format tag {tag} is refused as {what}"
            );
        }
    }

    #[test]
    fn a_width_this_decoder_does_not_read_is_refused() {
        // 12-bit PCM and 64-bit float both exist in the wild. Neither is a width to guess at.
        for (tag, bits, what) in [
            (1, 12, "a sample width that is not 8, 16, 24 or 32 bits"),
            (3, 64, "a floating-point width that is not 32 bits"),
        ] {
            let bytes = pcm(tag, 1, RATE as u32, bits, &[0; 8]);
            assert_eq!(
                decode(&bytes, RATE),
                Err(WavError::Unsupported { what }),
                "format tag {tag} at {bits} bits a sample"
            );
        }
    }

    #[test]
    fn a_format_tag_this_decoder_does_not_know_is_refused() {
        let bytes = pcm(0x55, 1, RATE as u32, 16, &[0; 4]);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::Unsupported {
                what: "a format tag this decoder does not know"
            })
        );
    }

    #[test]
    fn a_file_that_is_not_riff_is_refused() {
        assert_eq!(decode(b"not a wav!!", RATE), Err(WavError::NotRiff));
    }

    #[test]
    fn a_riff_file_that_is_not_a_wave_is_refused() {
        let bytes = b"RIFF\x04\0\0\0AVI ".to_vec();
        assert_eq!(decode(&bytes, RATE), Err(WavError::NotWave));
    }

    #[test]
    fn an_unknown_chunk_before_the_data_is_skipped() {
        // A `LIST` of seven bytes: odd, so it carries a pad byte the size does not count. A walker that forgets the pad
        // reads `fmt ` as `at ` and finds no format at all.
        let bytes = riff(&[
            chunk(b"LIST", b"INFOabc"),
            fmt_chunk(1, 1, RATE as u32, 16),
            data_chunk(&[0x00, 0x40, 0x00, 0xC0]),
        ]);
        let decoded = decode(&bytes, RATE).expect("a LIST before the samples is normal");
        let expected: [f32; 2] = [0.5, -0.5];
        assert_eq!(decoded.as_ref(), &expected[..]);
    }

    #[test]
    fn a_data_chunk_that_runs_past_the_file_is_refused() {
        let mut bytes = pcm16(&[0, 16_384, -16_384], 1);
        bytes.truncate(bytes.len() - 2);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::Truncated),
            "a cut file is an error, not a short read padded with garbage"
        );
    }

    #[test]
    fn a_file_longer_than_the_cap_is_refused() {
        let samples = vec![0u8; (MAX_SAMPLES + 1) * 2];
        let bytes = pcm(1, 1, RATE as u32, 16, &samples);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::TooLong),
            "{} samples is past the cap",
            MAX_SAMPLES + 1
        );
    }

    #[test]
    fn a_file_that_is_not_mono_or_stereo_is_refused() {
        let bytes = pcm(1, 3, RATE as u32, 16, &[0; 6]);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::Unsupported {
                what: "not mono or stereo"
            })
        );
    }

    #[test]
    fn a_frame_that_is_not_packed_is_refused() {
        // Four bytes a frame for one 16-bit channel: every frame after the first would be read from the wrong place.
        let bytes = riff(&[
            fmt_chunk_with_align(1, 1, RATE as u32, 16, 4),
            data_chunk(&[0; 8]),
        ]);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::Unsupported {
                what: "a frame that is not packed"
            })
        );
    }

    #[test]
    fn a_wave_missing_its_chunks_is_refused() {
        let no_format = riff(&[data_chunk(&[0, 0])]);
        assert_eq!(
            decode(&no_format, RATE),
            Err(WavError::Unsupported {
                what: "no format chunk"
            })
        );
        let no_data = riff(&[fmt_chunk(1, 1, RATE as u32, 16)]);
        assert_eq!(
            decode(&no_data, RATE),
            Err(WavError::Unsupported {
                what: "no sample data"
            })
        );
    }

    #[test]
    fn a_short_fmt_chunk_is_refused() {
        let bytes = riff(&[chunk(b"fmt ", &[1, 0, 1, 0]), data_chunk(&[0, 0])]);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::Unsupported {
                what: "a short `fmt ` chunk"
            })
        );
    }

    #[test]
    fn a_file_with_no_samples_is_silence_not_a_refusal() {
        // A zero-length `data` chunk is a voice with no sample, which a kit may hold: empty is silent, absent is an error.
        let decoded = decode(&pcm16(&[], 1), RATE).expect("an empty data chunk decodes");
        assert!(decoded.is_empty(), "no frames decode to no samples");
    }

    #[test]
    fn a_file_that_declares_no_rate_is_refused() {
        let bytes = pcm(1, 1, 0, 16, &[0; 4]);
        assert_eq!(
            decode(&bytes, RATE),
            Err(WavError::Unsupported {
                what: "a sample rate of zero"
            })
        );
    }

    #[test]
    fn a_target_rate_of_zero_is_refused() {
        let bytes = pcm16(&[0, 1], 1);
        let refusal = Err(WavError::Unsupported {
            what: "a target rate of zero",
        });
        assert_eq!(
            decode(&bytes, 0.0),
            refusal,
            "a rate of zero would divide by zero"
        );
        assert_eq!(
            decode(&bytes, -48_000.0),
            refusal,
            "a negative rate is not a rate"
        );
        assert_eq!(
            decode(&bytes, f32::NAN),
            refusal,
            "a NaN rate is not finite, and the guard that catches it is not the one for zero"
        );
    }

    #[test]
    fn every_refusal_says_what_is_wrong() {
        assert_eq!(WavError::NotRiff.to_string(), "not a RIFF file");
        assert_eq!(WavError::NotWave.to_string(), "not a WAVE file");
        assert_eq!(
            WavError::Unsupported { what: "mu-law" }.to_string(),
            "unsupported (mu-law)"
        );
        assert_eq!(
            WavError::Truncated.to_string(),
            "the file ends inside a chunk it declared"
        );
        assert_eq!(
            WavError::TooLong.to_string(),
            "longer than the sample limit"
        );
    }
}
