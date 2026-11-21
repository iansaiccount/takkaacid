//! The kit browser: the three kits that ship, then the player's own, drawn over the panel as the banks card's sibling.
//!
//! It is also the one seam a kit edit goes through — [`edit`]. A drum dial is published clone-on-write, several times a
//! second while a hand is turning it, and a factory kit is compiled into the binary and cannot be edited at all. So "editing
//! a factory kit copies it under a free name" has to happen *once*, where the edit enters, before anything is written: put
//! that rule in the publish path and a single drag leaves a dozen numbered copies behind.
//!
//! Nothing here touches the disk. The card paints and asks; `main` reads and writes, exactly as the banks card does.

use std::sync::Arc;

use egui::{Rect, Ui};

// The card's measurements and its caption are the banks card's, imported rather than repeated: the two are the same object
// to a player, and a second set of numbers beside the first is how two siblings stop looking alike.
use super::banks::{CARD_GAP, CARD_HEADING, CARD_PADDING, CARD_ROW, heading};
use super::{PanelState, text, theme, transport};
use crate::persist::free_name;
use crate::state::SharedState;
use acid_core::drums::Kit;

/// The card's own state, and the list `main` fills for it.
///
/// The list outlives the card: [`edit`] needs to know which names are taken even when nothing is on screen, or the copy a
/// first edit makes could land on top of a folder the player already has.
#[derive(Clone, Debug)]
pub struct Kits {
    /// Whether the browser is showing.
    pub open: bool,
    /// Every kit on offer, the factory three first. `main` reads it; the panel only lists it.
    pub names: Vec<String>,
    /// Set when the list needs reading again — a copy was just named, so the list has a kit it did not have.
    pub refresh: bool,
    /// The row whose load has been clicked once while the kit was edited and not saved. Loading another kit throws the edit
    /// away, so it takes two clicks, like replacing an unsaved song.
    pub load_armed: Option<String>,
    /// The card's own line: what an armed row is waiting for, and what `main::kit_notice` last said about the device's kit — a
    /// load that failed, a voice that would not decode, the "not running yet" refusal. It is written *and* sent to the status
    /// line while this card is up, because the card fills the panel and a notice painted underneath it is a notice nobody
    /// reads; it is cleared when the card closes, and by the next row that loads. (A copy's own name is the panel's notice, made
    /// by `ui::kits::edit`: a dial cannot be turned while this card is up, so that one is never made behind it.)
    pub message: Option<String>,
}

impl Default for Kits {
    /// A list nobody has read is not a list that is up to date, so `refresh` starts **true**: the copy a first edit of a
    /// factory kit makes is named against this list, and an empty one would let the copy land on a folder the player already
    /// has — overwriting it on the next save, silently. `main` fills it on the first frame.
    fn default() -> Self {
        Self {
            open: false,
            names: Vec::new(),
            refresh: true,
            load_armed: None,
            message: None,
        }
    }
}

/// What the card asked for. `main` does the work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Play this kit — a factory one out of the binary, a player's out of its folder.
    Load(String),
    /// Copy a kit folder the player picks into the library.
    Import,
    Close,
}

/// One rect per row of the card, in the order it paints them: two headings, the rows under each, then the two keys.
///
/// A test clicks the same place the card draws.
pub struct Rows {
    pub factory_heading: Rect,
    pub factory: Vec<Rect>,
    pub mine_heading: Rect,
    pub mine: Vec<Rect>,
    pub import: Rect,
    pub close: Rect,
}

/// The card's rect inside `panel`: the whole panel, less its margin, like the banks card.
pub fn card_rect(panel: Rect, s: f32) -> Rect {
    panel.shrink(theme::MARGIN * s)
}

pub fn rows(panel: Rect, s: f32, names: &[String]) -> Rows {
    let card = card_rect(panel, s);
    let left = card.left() + CARD_PADDING * s;
    let rows_wide = card.width() - CARD_PADDING * 2.0;
    let mut y = card.top() + (CARD_PADDING + 18.0) * s;
    let row = |y: &mut f32, wide: f32| {
        let rect = Rect::from_min_size(egui::Pos2::new(left, *y), egui::vec2(wide, CARD_ROW * s));
        *y += (CARD_ROW + CARD_GAP) * s;
        rect
    };
    let caption = |y: &mut f32| {
        let rect = Rect::from_min_size(
            egui::Pos2::new(left, *y),
            egui::vec2(rows_wide, CARD_HEADING * s),
        );
        *y += CARD_HEADING * s;
        rect
    };

    let factory_heading = caption(&mut y);
    let mut factory = Vec::new();
    let mut mine = Vec::new();
    // The caption for the player's list is placed where that list starts: above the first of their kits, or — when they
    // have none, which is what a first run has — after the factory rows. It used to be measured straight after the
    // `FACTORY` caption and moved only when a player's kit appeared, so a first run drew `MY KITS` on top of `808`.
    let mut mine_heading = Rect::NOTHING;
    // The room the rows may take: everything above the footer keys, less one row so the last row is not painted over them.
    // A player who has made more kits than fit sees the first of them — the alternative is rows painted over IMPORT and CLOSE
    // and off the bottom of the card. The songs card caps its list the same way, from the same numbers.
    let limit = card.bottom() - CARD_PADDING * s - CARD_ROW * s - CARD_GAP * s;
    for name in names {
        if (y + CARD_ROW * s) > limit {
            break;
        }
        if crate::kits::is_factory(name) {
            factory.push(row(&mut y, rows_wide));
        } else {
            if mine.is_empty() {
                mine_heading = caption(&mut y);
            }
            mine.push(row(&mut y, rows_wide));
        }
    }
    if mine.is_empty() {
        mine_heading = caption(&mut y);
    }

    let bottom = card.bottom() - CARD_PADDING * s - CARD_ROW * s;
    let import = Rect::from_min_size(
        egui::Pos2::new(left, bottom),
        egui::vec2(96.0 * s, CARD_ROW * s),
    );
    let close = Rect::from_min_size(
        egui::Pos2::new(card.right() - CARD_PADDING * s - 90.0 * s, bottom),
        egui::vec2(90.0 * s, CARD_ROW * s),
    );
    Rows {
        factory_heading,
        factory,
        mine_heading,
        mine,
        import,
        close,
    }
}

/// Paint the card, and say what was clicked, if anything. The first request of the frame wins.
pub fn paint(
    ui: &mut Ui,
    state: &mut PanelState,
    shared: &SharedState,
    panel: Rect,
    s: f32,
) -> Option<Request> {
    let current = shared.kit();
    let read_only = crate::kits::is_factory(&current.name);
    let card = card_rect(panel, s);
    {
        let painter = ui.painter();
        painter.rect_filled(card, 6.0 * s, theme::BLACK.panel_raised);
        painter.rect_stroke(
            card,
            6.0 * s,
            theme::BLACK.silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
    }
    header(ui, card, s, &current.name, read_only);

    let names = state.kits.names.clone();
    let layout = rows(panel, s, &names);
    let mut request = None;

    heading(ui, s, "FACTORY", layout.factory_heading.left_top());
    for (index, name) in names
        .iter()
        .filter(|name| crate::kits::is_factory(name))
        .enumerate()
    {
        let Some(rect) = layout.factory.get(index).copied() else {
            break;
        };
        if row_key(ui, state, s, rect, name, &current.name) {
            request = Some(Request::Load(name.clone()));
        }
    }
    heading(ui, s, "MY KITS", layout.mine_heading.left_top());
    for (index, name) in names
        .iter()
        .filter(|name| !crate::kits::is_factory(name))
        .enumerate()
    {
        let Some(rect) = layout.mine.get(index).copied() else {
            break;
        };
        if row_key(ui, state, s, rect, name, &current.name) {
            request = Some(Request::Load(name.clone()));
        }
    }

    let a_key = transport::card_button(ui, "kit-import", layout.import, "IMPORT", false).clicked();
    let closed = transport::card_button(ui, "kit-close", layout.close, "CLOSE", false).clicked();
    if a_key || closed {
        state.kits.load_armed = None;
        if request.is_none() {
            request = Some(if a_key {
                Request::Import
            } else {
                Request::Close
            });
        }
    }
    if let Some(message) = &state.kits.message {
        ui.painter().text(
            egui::Pos2::new(
                layout.import.right() + CARD_PADDING * s,
                layout.import.center().y,
            ),
            egui::Align2::LEFT_CENTER,
            message,
            theme::legend(10.0 * s),
            theme::BLACK.silkscreen_dim,
        );
    }
    request
}

/// One kit's row. Answers whether the click on it carries a load out; a first click on a row whose kit has unsaved edits
/// only asks.
///
/// What the row is — a kit that ships or one of the player's — is read from the name, and whether an edit is owed from the
/// panel, so neither is a fourth caller-side truth that could disagree with the card's own caption.
fn row_key(
    ui: &mut Ui,
    state: &mut PanelState,
    s: f32,
    rect: Rect,
    name: &str,
    current: &str,
) -> bool {
    let key = format!("kit:{name}");
    let read_only = crate::kits::is_factory(name);
    // Whether the kit that a load would replace has edits that are not on disk. The load lands on the selected device, so
    // the question is that device's, by number — another machine's owed write is not this row's to guard.
    let selected = state
        .strip
        .cells
        .get(state.strip.selected)
        .map(|cell| cell.number);
    let dirty = state.kit_dirty.is_some() && state.kit_dirty == selected;
    let armed = state.kits.load_armed.as_deref() == Some(key.as_str());
    // The card says what a row means before it is clicked: a factory kit is read-only, and a row that will replace unsaved
    // edits says so on its second click.
    let mut label = if read_only {
        format!("{name} - READ ONLY")
    } else {
        name.to_string()
    };
    if armed && dirty {
        label = format!("{name} - click again to replace");
    }
    let font = transport::key_font(rect);
    let label = text::fit(ui.painter(), &label, font, rect.width() - 8.0 * s);
    let id = format!("kit-{}-{name}", if read_only { "factory" } else { "mine" });
    let clicked = transport::card_button(ui, &id, rect, &label, name == current).clicked();
    if !clicked {
        return false;
    }
    if !dirty || armed {
        state.kits.load_armed = None;
        state.kits.message = None;
        return true;
    }
    // Two clicks to throw an unsaved edit away, the same rule as loading over an unsaved song.
    state.kits.load_armed = Some(key);
    state.kits.message = Some("this kit has unsaved edits: click again to replace it".to_string());
    false
}

/// The card's caption, and what the device on the panel is playing: a kit that ships says so, because the next edit of it
/// makes a copy.
fn header(ui: &Ui, card: Rect, s: f32, current: &str, read_only: bool) {
    let painter = ui.painter();
    painter.text(
        egui::Pos2::new(
            card.left() + CARD_PADDING * s,
            card.top() + CARD_PADDING * s,
        ),
        egui::Align2::LEFT_TOP,
        "KITS",
        theme::legend(13.0 * s),
        theme::BLACK.silkscreen,
    );
    let now = if read_only {
        format!("NOW: {current} - READ ONLY")
    } else {
        format!("NOW: {current}")
    };
    let font = theme::legend(10.0 * s);
    let now = text::fit(
        painter,
        &now,
        font.clone(),
        (card.width() - 2.0 * CARD_PADDING * s) * 0.6,
    );
    painter.text(
        egui::Pos2::new(
            card.right() - CARD_PADDING * s,
            card.top() + CARD_PADDING * s + 2.0 * s,
        ),
        egui::Align2::RIGHT_TOP,
        now,
        font,
        theme::BLACK.silkscreen_dim,
    );
}

/// Edit the kit the device on the panel plays, through the one rule that a factory kit cannot be edited in place.
///
/// This is the seam every kit edit goes through — the drum face's dials, its filter key, and anything later that shapes a
/// voice. The first edit of a kit that ships renames the copy to a free name, once, in memory, before anything is written,
/// so a dial drag publishes one copy however many frames it lasts. The copy is written when the session is saved and named
/// by `SavedState::kit`; nothing is ever written into the factory kits, which are in the binary and have no folder at all.
pub fn edit(state: &mut PanelState, shared: &SharedState, edit: impl FnOnce(&mut Kit)) {
    let mut kit = (*shared.kit()).clone();
    let mut message = None;
    if crate::kits::is_factory(&kit.name) {
        let original = kit.name.clone();
        // Both walls at once: a factory name, and anything the library already holds. `free_name` is the rule a song's copy
        // uses, so a kit's `808 (2)` cannot land differently from a song's.
        let copy = free_name(&original, |candidate| {
            crate::kits::is_factory(candidate)
                || state.kits.names.iter().any(|listed| listed == candidate)
        });
        kit.name = copy.clone();
        state.kits.refresh = true;
        message = Some(format!(
            "{original} is read-only: this is {copy} now, saved with the session"
        ));
    }
    edit(&mut kit);
    shared.set_kit(Arc::new(kit));
    // A kit is part of the document twice over — its name is in the save file and its samples are a folder — so an edit is
    // unsaved work whichever of the two it changed.
    state.dirty = true;
    // ...and the folder is owed a write for the device that was edited, recorded by its **number** — the identity that
    // survives a removal, as `PanelState::focused` and `edit_slots` already use — because the device selected when the save
    // happens is not always the one that was edited, and a rack position shifts when a device before it goes.
    state.kit_dirty = state
        .strip
        .cells
        .get(state.strip.selected)
        .map(|cell| cell.number);
    if let Some(message) = message {
        state.notify(message);
    }
}

/// Show the browser, and put every other card away: there is room for one, and this one is about the device's sound.
pub fn open(state: &mut PanelState) {
    // One card at a time, spelled in one place: `put_cards_away` closes every other card *including this one*, so the two
    // fields below are set after it.
    state.put_cards_away();
    state.kits.open = true;
    state.kits.refresh = true;
    state.kits.load_armed = None;
    state.kits.message = None;
}

/// Put the browser away, and its arming with it: the question belonged to the card.
pub fn close(state: &mut PanelState) {
    state.kits.open = false;
    state.kits.load_armed = None;
    state.kits.message = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Rect {
        Rect::from_min_size(egui::Pos2::ZERO, theme::DESIGN_SIZE)
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn each_caption_sits_above_its_own_rows_and_never_on_one() {
        // A first run: the three that ship and nothing of the player's. `MY KITS` still gets a caption, and it belongs
        // under the last factory row — the browser first shipped with that caption measured straight after `FACTORY`,
        // which drew it across `808` and made the card read as if the two lists overlapped.
        let layout = rows(panel(), 1.0, &names(&["808", "909", "Acoustic"]));
        assert_eq!(layout.factory.len(), 3);
        assert!(layout.mine.is_empty());
        assert!(layout.factory_heading.bottom() <= layout.factory[0].top());
        for pair in layout.factory.windows(2) {
            assert!(pair[0].bottom() <= pair[1].top(), "factory rows overlap");
        }
        assert!(
            layout.mine_heading.top() >= layout.factory[2].bottom(),
            "the MY KITS caption is drawn over the factory rows: {:?} vs {:?}",
            layout.mine_heading,
            layout.factory[2]
        );

        // With a kit of the player's, the caption moves to just above their first row, below the factory ones.
        let layout = rows(panel(), 1.0, &names(&["808", "909", "Acoustic", "My kit"]));
        assert_eq!(layout.mine.len(), 1);
        assert!(layout.mine_heading.bottom() <= layout.mine[0].top());
        assert!(layout.mine_heading.top() >= layout.factory[2].bottom());
    }

    #[test]
    fn the_footer_keys_stay_below_every_row() {
        // Forty kits: the rows stop above `IMPORT` and `CLOSE` rather than being painted over them.
        let many: Vec<String> = (0..40).map(|index| format!("Kit {index}")).collect();
        let layout = rows(panel(), 1.0, &many);
        let footer = layout.import.top().min(layout.close.top());
        for rect in layout.mine.iter().chain(&layout.factory) {
            assert!(rect.bottom() <= footer, "a row runs under the keys");
        }
    }
}
