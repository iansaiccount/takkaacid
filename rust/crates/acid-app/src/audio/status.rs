//! How the panel describes the audio stream: one honest line, and device names that cannot overflow it.

use crate::state::AudioStatus;

/// The status line for the strip. Always ASCII, never empty, never a lie: it says what the stream is doing.
pub fn status_text(status: &AudioStatus) -> String {
    match status {
        AudioStatus::Starting => "starting".to_string(),
        AudioStatus::Running {
            device,
            sample_rate,
        } => format!("{} kHz - {}", sample_rate / 1_000, device),
        AudioStatus::Failed { reason } => format!("failed: {reason}"),
    }
}

/// Whether a status line says the stream has given up. The supervisor writes `failed: …` when it has, and
/// `reconnecting (attempt n/m)` while it is still trying, so the words are the only signal either surface needs.
pub fn is_failed(text: &str) -> bool {
    text.starts_with("failed")
}

/// The typed status a device's state mirrors, from the rig's two facts: the open stream (its device name and rate) and the
/// line the player reads.
///
/// `device` and `sample_rate` come from the stream and never from the line, which is prose — a `Running` whose `device` is
/// `"48 kHz - PipeWire"` ticks no row on the settings card and prints a sentence where a device belongs. A `reason` likewise
/// carries the failure without the `failed: ` the line prefixes it with, because `status_text` puts that back.
pub fn mirror(running: Option<(&str, u32)>, line: &str) -> AudioStatus {
    match running {
        Some((device, sample_rate)) => AudioStatus::Running {
            device: device.to_string(),
            sample_rate,
        },
        None if is_failed(line) => AudioStatus::Failed {
            reason: line.trim_start_matches("failed:").trim_start().to_string(),
        },
        // Starting, reconnecting, or a stream that was never opened: nothing is running, and `Starting` is the honest word for
        // all three — the line itself says which, and it is what the settings card shows.
        None => AudioStatus::Starting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_names_every_state() {
        assert_eq!(
            status_text(&AudioStatus::Running {
                device: "PipeWire".into(),
                sample_rate: 48_000
            }),
            "48 kHz - PipeWire"
        );
        assert_eq!(status_text(&AudioStatus::Starting), "starting");
        let failed = status_text(&AudioStatus::Failed {
            reason: "device busy".into(),
        });
        assert!(failed.starts_with("failed:") && failed.contains("device busy"));
    }

    #[test]
    fn a_failure_is_recognisable_from_the_words_alone() {
        // The card colours the line by this, so it must agree with what `status_text` writes for every state.
        assert!(is_failed(&status_text(&AudioStatus::Failed {
            reason: "device busy".into()
        })));
        assert!(!is_failed(&status_text(&AudioStatus::Starting)));
        assert!(!is_failed("48 kHz - PipeWire"));
        // ...and the mirror agrees with it: only a line that says `failed:` is a failure, and a reconnect is `Starting`.
        assert!(matches!(
            mirror(None, "failed: device busy"),
            AudioStatus::Failed { .. }
        ));
        assert_eq!(
            mirror(None, "reconnecting (attempt 1/3)"),
            AudioStatus::Starting
        );
    }

    #[test]
    fn the_mirror_takes_the_device_from_the_stream_and_the_reason_from_the_line() {
        // The bug this replaced: `device` was the *display line* (`"48 kHz - PipeWire"`), so the settings card printed it as a
        // device name and ticked no row, and `reason` was `"failed: device busy"`, which `status_text` prefixed again.
        assert_eq!(
            mirror(Some(("PipeWire", 48_000)), "48 kHz - PipeWire"),
            AudioStatus::Running {
                device: "PipeWire".to_string(),
                sample_rate: 48_000,
            }
        );
        assert_eq!(
            mirror(None, "failed: device busy"),
            AudioStatus::Failed {
                reason: "device busy".to_string(),
            }
        );
        assert_eq!(mirror(None, "starting"), AudioStatus::Starting);
        // The round trip the strip and the card both depend on: the line this status prints is the line the rig had.
        assert_eq!(
            status_text(&mirror(Some(("PipeWire", 48_000)), "48 kHz - PipeWire")),
            "48 kHz - PipeWire"
        );
    }

    #[test]
    fn the_status_line_is_ascii_for_every_state() {
        for status in [
            AudioStatus::Starting,
            AudioStatus::Running {
                device: "Built-in Audio Analog Stereo".into(),
                sample_rate: 44_100,
            },
            AudioStatus::Failed {
                reason: "no device".into(),
            },
        ] {
            let text = status_text(&status);
            assert!(!text.is_empty(), "an empty status says nothing");
            assert!(text.is_ascii(), "non-ascii status: {text}");
        }
    }

    #[test]
    fn listing_output_devices_never_panics() {
        // A machine with no devices at all must give an empty list, not a panic or an unwrap.
        let devices = crate::audio::list_output_devices();
        for name in &devices {
            assert!(
                !name.is_empty(),
                "a device with no name is no use on a panel"
            );
        }
    }
}
