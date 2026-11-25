//! What a rack file does to a session, decided where a test can call it.
//!
//! `main` is a binary no test can construct, so a recall's decisions — which machines, which selection, what the panel
//! forgets, which kits and ports are owed — live in `rack_recall` and this file is where they are pinned. What `main` keeps
//! is the decoding and the MIDI, and it performs them from `Recalled`.

use acid_app::device::{DeviceKind, Rack, Scheme};
use acid_app::midi::route::{Channel, Ports, Role};
use acid_app::persist::{self, Knobs, RackDocument, SAVE_VERSION, SavedDevice, SavedInputs};
use acid_app::rack_recall;
use acid_app::ui::PanelState;
use acid_core::bank::PatternBank;

/// One machine as the file stores it, with every field set to something a default would not produce.
fn a_saved_device(
    number: usize,
    name: &str,
    kind: &str,
    kit: Option<&str>,
    port: &str,
) -> SavedDevice {
    SavedDevice {
        number,
        name: name.to_string(),
        kind: kind.to_string(),
        scheme: if number == 2 {
            "VINYL NOIR".to_string()
        } else {
            "BLACK".to_string()
        },
        inputs: SavedInputs {
            ports: Ports::Named(vec![port.to_string()]),
            channel: Channel::One(1),
            role: Role::PlayAndControl,
        },
        bank: if number == 1 {
            PatternBank::factory()
        } else {
            PatternBank::new_empty()
        },
        knobs: Knobs {
            cutoff: 0.1 * number as f32,
            ..Knobs::default()
        },
        swing: 0.0,
        cc_map: persist::default_cc_map(),
        kit: kit.map(str::to_string),
    }
}

#[test]
fn a_recall_replaces_the_machines_and_answers_what_is_left_to_do() {
    // One acid line at 140 with a bank file open and a card showing a machine that is about to be gone.
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    rack.set_tempo_bpm(140.0);
    let mut panel = PanelState::default();
    panel.songs.bank_file = Some("Live".to_string());
    panel.open_device_card(0);
    panel.dirty = false;

    let document = RackDocument {
        version: SAVE_VERSION,
        devices: vec![
            a_saved_device(1, "KIT", "DRUMS", Some("909"), "Port A"),
            a_saved_device(2, "BASS", "ACID", None, "Port B"),
        ],
    };

    let recalled = rack_recall::recall(&mut rack, &mut panel, &document, &["Port A".to_string()]);

    let names: Vec<&str> = rack.devices().iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["KIT", "BASS"], "the document's machines, in order");
    assert_eq!(rack.devices()[0].kind(), DeviceKind::Drums);
    assert_eq!(rack.devices()[1].scheme, Scheme::VinylNoir);
    assert_eq!(
        rack.devices()[0].bank,
        document.devices[0].bank,
        "a machine's own bank came with it"
    );
    assert_eq!(rack.selected(), 0, "the panel shows the file's first");
    assert_eq!(rack.tempo_bpm(), 140.0, "a rack file brings no clock");

    // What a restart forgets, a recall forgets: nothing is muted and nothing is transposed.
    for device in rack.devices() {
        assert!(!device.state.is_muted(), "a recall leaves every machine on");
        assert_eq!(device.state.transpose(), 0, "and in tune");
    }

    assert!(panel.dirty, "the session changed under the player");
    assert_eq!(
        panel.songs.bank_file, None,
        "the bank file belonged to machines that are gone"
    );
    assert!(
        panel.device_card.open.is_none(),
        "the card pointed at a machine that is no longer there"
    );

    assert_eq!(
        recalled.kits,
        vec![(0, "909".to_string())],
        "the machine whose kit must be decoded"
    );
    assert_eq!(
        recalled.unopened_ports,
        vec!["Port B".to_string()],
        "the port this machine has not got, named once"
    );
}
