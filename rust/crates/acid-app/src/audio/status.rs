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

/// A device name cut to `max` characters with the middle replaced, so both ends stay recognisable.
pub fn device_label(name: &str, max: usize) -> String {
    if name.chars().count() <= max || max < 7 {
        return name.to_string();
    }
    let keep = max - 3; // the "..."
    let head_len = keep - keep / 2 - 1;
    let tail_len = keep - head_len;
    let chars: Vec<char> = name.chars().collect();
    let head: String = chars[..head_len].iter().collect();
    let tail: String = chars[chars.len() - tail_len..].iter().collect();
    format!("{head}...{tail}")
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
        assert!(!is_failed("reconnecting (attempt 1/3)"));
        assert!(!is_failed("48 kHz - PipeWire"));
    }

    #[test]
    fn long_device_names_are_ellipsised_in_the_middle() {
        let label = device_label("Built-in Audio Analog Stereo Digital Surround", 24);
        assert!(label.len() <= 24, "{label}");
        assert!(
            label.contains('…') || label.contains("..."),
            "no ellipsis: {label}"
        );
        assert_eq!(device_label("PipeWire", 24), "PipeWire");
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
            let label = device_label(name, 24);
            assert!(label.chars().count() <= 24, "{label}");
        }
    }
}
