//! MIDI input: finding a port, and holding a connection that never blocks the audio thread.
//!
//! Messages arrive on midir's own thread; the handler writes atomics into `SharedState` and nothing else.
//! A missing interface is the common case and must be a logged line, not an error dialog.

use anyhow::{Result, anyhow};
use midir::{MidiInput, MidiInputConnection};

/// What `list_ports` reports when the machine has no MIDI input at all.
pub const NO_PORTS: &str = "no MIDI input ports found";

/// The message `list_ports` fails with when nothing is attached.
pub fn empty_ports_error() -> &'static str {
    NO_PORTS
}

/// The index of the port with exactly this name. Never a substring and never a neighbour: every name the app holds came
/// from the machine's own enumeration, so a name it no longer reports is a port that has gone, not a fuzzy match.
fn exact_index(names: &[String], name: &str) -> Option<usize> {
    names.iter().position(|listed| listed == name)
}
/// The ports a session opens when it has no saved preference: the first the machine reports, which is the one the app
/// opened on its own before it could hold more than one.
///
/// One and not all, because one interface can publish two ports — USB and DIN, or a DAW forwarding its input — and two
/// handlers with a note gate each cannot tell that one press arrived twice. The card offers the rest, one tick each.
pub fn opened_by_default(names: &[String]) -> Vec<String> {
    names.first().cloned().into_iter().collect()
}

/// Every MIDI input port the system reports, by name.
pub fn list_ports() -> Result<Vec<String>> {
    let input = MidiInput::new("takka-acid").map_err(|error| anyhow!("opening MIDI: {error}"))?;
    let names: Vec<String> = input
        .ports()
        .iter()
        .filter_map(|port| input.port_name(port).ok())
        .collect();
    if names.is_empty() {
        return Err(anyhow!(empty_ports_error()));
    }
    Ok(names)
}

/// An open input connection. Dropping it closes the port.
pub struct MidiIn {
    /// Held only for its `Drop`, which closes the port; never read.
    #[allow(dead_code)]
    connection: MidiInputConnection<()>,
}

impl MidiIn {
    /// Open the port with exactly this name, and hand every message to `handler` on midir's thread.
    ///
    /// A name the machine does not report is an error, never a substitute: quietly listening to a different interface is
    /// worse than not listening at all. The handler must not block — it runs on the MIDI thread, so the wiring writes
    /// atomics and returns.
    pub fn open_named(
        name: &str,
        mut handler: impl FnMut(u64, &[u8]) + Send + 'static,
    ) -> Result<Self> {
        let input =
            MidiInput::new("takka-acid").map_err(|error| anyhow!("opening MIDI: {error}"))?;
        let ports = input.ports();
        let names: Vec<String> = ports
            .iter()
            .filter_map(|port| input.port_name(port).ok())
            .collect();
        if names.is_empty() {
            return Err(anyhow!(empty_ports_error()));
        }
        let index =
            exact_index(&names, name).ok_or_else(|| anyhow!("MIDI port {name:?} not found"))?;
        let connection = input
            .connect(
                &ports[index],
                "takka-acid-in",
                move |stamp, message, _| handler(stamp, message),
                (),
            )
            .map_err(|error| anyhow!("connecting to {name}: {error}"))?;
        Ok(Self { connection })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_answered_by_that_port_or_by_nothing() {
        // The rule that makes holding a name safe: a port that has gone is silent, not answered with its neighbour.
        let names = vec!["Arturia KeyStep".to_string(), "Launchkey MIDI".to_string()];
        assert_eq!(exact_index(&names, "Launchkey MIDI"), Some(1));
        assert_eq!(
            exact_index(&names, "keystep"),
            None,
            "a shorter name is a different port, not a match"
        );
        assert_eq!(exact_index(&names, "Port B"), None);
        assert_eq!(exact_index(&[], "Launchkey MIDI"), None);
    }

    #[test]
    fn a_fresh_session_opens_one_port_and_not_every_one() {
        // A keyboard can publish two ports — USB and DIN, or a DAW forwarding its input — and two handlers each with its
        // own note gate cannot tell that the two presses are one press. So a session with no saved preference opens the
        // port the app used to open on its own, and the card offers the rest.
        let names = vec!["Launchkey MIDI".to_string(), "nanoKONTROL".to_string()];
        assert_eq!(
            opened_by_default(&names),
            vec!["Launchkey MIDI".to_string()]
        );
        assert_eq!(opened_by_default(&names).len(), 1, "one press, one step");
        assert!(
            opened_by_default(&[]).is_empty(),
            "nothing to open is nothing"
        );
    }

    #[test]
    fn no_ports_is_a_clear_error() {
        // list_ports() with an empty registry: the message must name the problem, not just fail.
        assert_eq!(empty_ports_error(), "no MIDI input ports found");
    }

    /// The device-independent promise: enumerating ports either lists some or fails with the clear message.
    /// It never panics, whatever is or is not plugged in.
    #[test]
    fn listing_ports_either_lists_or_explains() {
        match list_ports() {
            Ok(names) => assert!(!names.is_empty(), "a successful listing is never empty"),
            Err(error) => assert_eq!(format!("{error}"), NO_PORTS),
        }
    }
}
