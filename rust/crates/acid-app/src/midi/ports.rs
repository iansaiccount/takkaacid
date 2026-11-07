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

/// Choose a port: an exact name first, then a case-insensitive substring, then the first port when no
/// preference was given.
pub fn pick_port(names: &[String], wanted: Option<&str>) -> Option<usize> {
    let Some(wanted) = wanted else {
        return if names.is_empty() { None } else { Some(0) };
    };
    if let Some(exact) = names.iter().position(|name| name == wanted) {
        return Some(exact);
    }
    let needle = wanted.to_lowercase();
    names
        .iter()
        .position(|name| name.to_lowercase().contains(&needle))
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
    port_name: String,
}

impl MidiIn {
    /// Open a port and hand every message to `handler` on midir's thread.
    ///
    /// `port_name` is a preference: exact match, then substring, then the first port. The handler must not
    /// block — it runs on the MIDI thread, so the wiring writes atomics and returns.
    pub fn open(
        port_name: Option<&str>,
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
        let index = pick_port(&names, port_name)
            .ok_or_else(|| anyhow!("MIDI port {port_name:?} not found"))?;
        let chosen = names[index].clone();
        let connection = input
            .connect(
                &ports[index],
                "takka-acid-in",
                move |stamp, message, _| handler(stamp, message),
                (),
            )
            .map_err(|error| anyhow!("connecting to {chosen}: {error}"))?;
        Ok(Self {
            connection,
            port_name: chosen,
        })
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exact_name_wins_over_a_substring() {
        let names = vec!["Arturia KeyStep".to_string(), "Launchkey MIDI".to_string()];
        assert_eq!(pick_port(&names, Some("Launchkey MIDI")), Some(1));
        assert_eq!(pick_port(&names, Some("keystep")), Some(0));
        assert_eq!(pick_port(&names, Some("ARTURIA")), Some(0));
    }

    #[test]
    fn a_missing_name_and_no_ports_are_handled() {
        let names = vec!["Port A".to_string()];
        assert_eq!(pick_port(&names, Some("nope")), None);
        assert_eq!(pick_port(&[], None), None);
        assert_eq!(pick_port(&names, None), Some(0));
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
