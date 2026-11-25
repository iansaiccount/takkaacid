//! What a rack file does to the session it is recalled into.
//!
//! A recall is four decisions and two side effects. The decisions — which machines, which selection, what the panel forgets,
//! which kits and ports are owed — live here, because `main` is a binary no test can construct and the last round's review
//! found its one P0 in code that no test could reach. The side effects need the audio device and the MIDI system, so `main`
//! performs them from [`Recalled`].

use crate::device::Rack;
use crate::persist::{self, RackDocument};
use crate::ui::{PanelState, pattern_swap};

/// What a recall changed, and what `main` still has to do about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recalled {
    /// The machines whose kit has to be decoded, at the rate the running device plays at.
    pub kits: Vec<(usize, String)>,
    /// The ports the new rack names that this machine has not got, in the order they are first named.
    pub unopened_ports: Vec<String>,
}

/// Replace the session's rack with a document's, and put the panel back on slot 0.
///
/// The machines come back as the file wrote them, and everything a restart forgets a recall forgets too — mute, `REVERSE`
/// and transpose — because every machine is built anew, exactly as a launch builds it. The rack's tempo cell is the
/// session's and does not move: a rack document carries no clock.
///
/// `open_ports` is what the MIDI system has open, which is `main`'s to know; the answer names the ports the document wants
/// and this machine has not got, so one line on the status line can say so.
pub fn recall(
    rack: &mut Rack,
    panel: &mut PanelState,
    document: &RackDocument,
    open_ports: &[String],
) -> Recalled {
    let state = document.into_saved();
    rack.replace_devices_from(&state);
    // The panel was pointing into a rack that is gone: the card it had open, the pattern it was drawing and the bank file
    // the selected device's bank came from all belong to machines that are no longer there.
    panel.close_device_card();
    pattern_swap::adopt(panel, rack.state(), rack.bank());
    panel.songs.bank_file = None;
    panel.dirty = true;
    Recalled {
        kits: persist::wanted_kits(&state),
        unopened_ports: persist::unopened_ports(&state, open_ports),
    }
}
