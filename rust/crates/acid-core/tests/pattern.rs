use acid_core::pattern::{Grid, PATTERN_SLOTS, Pattern, PatternId, Step};
use acid_core::pitch::Pitch;

#[test]
fn a_new_pattern_is_sixteen_rests() {
    let p = Pattern::empty();
    assert_eq!(
        p.len(),
        Grid::Sixteenth.steps_per_bar(),
        "a fresh pattern is one sixteenth bar"
    );
    // Every slot, the ones above the bar included: the invariant is that they are rests.
    assert!(
        p.steps.iter().all(|s| !s.on && !s.accent && !s.slide),
        "a fresh pattern is all rests, played or not"
    );
}

#[test]
fn a_note_step_reports_its_frequency() {
    let s = Step::note(Pitch::CHigh, 0);
    assert!(s.on);
    assert!((s.hz() - 32.7).abs() < 0.05, "C0 is 32.7 Hz");
    let up = Step::note(Pitch::CHigh, 1);
    assert!((up.hz() - 65.41).abs() < 0.1, "C1 is 65.4 Hz");
}

#[test]
fn patterns_round_trip_through_json() {
    let mut p = Pattern::empty();
    p.steps[0] = Step {
        accent: true,
        ..Step::note(Pitch::E, 0)
    };
    p.steps[3] = Step {
        slide: true,
        ..Step::note(Pitch::DSharp, 0)
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: Pattern = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn pattern_ids_read_like_the_hardware() {
    assert_eq!(PatternId { bank: 0, number: 0 }.to_string(), "A1");
    assert_eq!(PatternId { bank: 2, number: 4 }.to_string(), "C5");
    assert_eq!(PatternId { bank: 7, number: 7 }.to_string(), "H8");
}

#[test]
fn the_grid_catalogue_names_one_bar_each() {
    assert_eq!(
        Grid::ALL,
        [Grid::Sixteenth, Grid::ThirtySecond, Grid::EighthTriplet]
    );
    for grid in Grid::ALL {
        assert_eq!(
            Grid::from_name(grid.name()),
            Some(grid),
            "a caption round-trips"
        );
    }
    assert_eq!(Grid::Sixteenth.name(), "1/16");
    assert_eq!(Grid::ThirtySecond.name(), "1/32");
    assert_eq!(Grid::EighthTriplet.name(), "1/8T");
    assert_eq!(
        Grid::from_name("1/64"),
        None,
        "a caption this build has never had"
    );
    assert_eq!(Grid::Sixteenth.steps_per_bar(), 16);
    assert_eq!(Grid::ThirtySecond.steps_per_bar(), 32);
    assert_eq!(Grid::EighthTriplet.steps_per_bar(), 12);
}

#[test]
fn a_beat_is_every_fourth_eighth_or_third_step() {
    for (grid, beats) in [
        (Grid::Sixteenth, [0, 4, 8, 12]),
        (Grid::ThirtySecond, [0, 8, 16, 24]),
        (Grid::EighthTriplet, [0, 3, 6, 9]),
    ] {
        let found: Vec<usize> = (0..grid.steps_per_bar())
            .filter(|i| grid.is_beat(*i))
            .collect();
        assert_eq!(found, beats, "{} frames its beats", grid.name());
    }
}

#[test]
fn an_unknown_grid_caption_reads_as_sixteenths() {
    assert_eq!(
        serde_json::from_str::<Grid>("\"1/32\"").unwrap(),
        Grid::ThirtySecond
    );
    // A word this build does not know must not cost the player their song: it reads as the grid every version-12
    // file is on.
    assert_eq!(
        serde_json::from_str::<Grid>("\"1/64\"").unwrap(),
        Grid::Sixteenth
    );
    assert_eq!(
        serde_json::to_string(&Grid::EighthTriplet).unwrap(),
        "\"1/8T\""
    );
}

#[test]
fn a_pattern_is_as_long_as_its_grid() {
    assert_eq!(Pattern::empty().len(), 16, "a fresh pattern is sixteenths");
    assert_eq!(Pattern::empty_in(Grid::EighthTriplet).len(), 12);
    assert_eq!(Pattern::empty_in(Grid::ThirtySecond).len(), 32);
    assert_eq!(Pattern::empty().iter().count(), 16);
}

#[test]
fn a_narrower_grid_holds_the_steps_it_stops_playing() {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[0] = Step::note(Pitch::A, 1);
    steps[20] = Step::note(Pitch::C, 2); // beyond a 12-step bar
    let wide = Pattern::from_steps_in(Grid::ThirtySecond, steps);
    assert_eq!(wide.len(), 32);
    assert!(wide.steps[20].on, "the long bar holds it");

    let narrow = wide.with_grid(Grid::EighthTriplet);
    assert_eq!(narrow.len(), 12);
    assert!(narrow.steps[0].on, "the steps inside the bar are untouched");
    assert!(
        narrow.steps[20].on,
        "the step outside the bar is held, not destroyed"
    );

    // Back again: the held note plays, unchanged. This is the whole point of the round.
    let back = narrow.with_grid(Grid::ThirtySecond);
    assert_eq!(back.len(), 32);
    assert!(back.steps[20].on, "the note came back");
    assert_eq!(back.steps[20].pitch, Pitch::C);
    assert_eq!(back, wide, "narrowing and widening is the identity");
}

#[test]
fn every_slot_above_len_is_not_played() {
    let steps = [Step::note(Pitch::G, 0); PATTERN_SLOTS];
    for grid in Grid::ALL {
        for pattern in [Pattern::empty_in(grid), Pattern::from_steps_in(grid, steps)] {
            for (index, step) in pattern.steps.iter().enumerate().skip(pattern.len()) {
                assert_eq!(
                    *step,
                    Step::rest(),
                    "{} slot {index} holds sound from a constructor",
                    grid.name()
                );
            }
        }
        // A grid *change* is the one path that may hold a note above the bar, and it must not play it.
        let held = Pattern::from_steps_in(Grid::ThirtySecond, steps).with_grid(grid);
        assert!(
            held.iter().all(|step| step.on),
            "{} plays only its own bar",
            grid.name()
        );
        assert_eq!(held.iter().count(), held.len());
    }
}

#[test]
fn the_file_writes_the_steps_that_play() {
    let sixteenth = Pattern::from_steps([Step::note(Pitch::A, 1); 16]);
    let json: serde_json::Value = serde_json::to_value(sixteenth).unwrap();
    assert_eq!(json["grid"], "1/16");
    assert_eq!(json["steps"].as_array().unwrap().len(), 16);

    let wide = Pattern::empty_in(Grid::ThirtySecond);
    let json: serde_json::Value = serde_json::to_value(wide).unwrap();
    assert_eq!(json["grid"], "1/32");
    assert_eq!(json["steps"].as_array().unwrap().len(), 32);
}

#[test]
fn a_version_twelve_pattern_reads_as_sixteenths() {
    // What a version-12 build wrote: sixteen steps and no grid at all.
    let mut steps = vec![
        serde_json::json!({"on": false, "pitch": "C", "octave_offset": 0,
        "accent": false, "slide": false, "velocity": 127, "hits": 0, "accents": 0});
        16
    ];
    steps[0] = serde_json::json!({"on": true, "pitch": "A", "octave_offset": 1,
        "accent": false, "slide": false, "velocity": 127, "hits": 0, "accents": 0});
    let text = serde_json::json!({ "steps": steps }).to_string();
    let pattern: Pattern = serde_json::from_str(&text).unwrap();
    assert_eq!(pattern.grid, Grid::Sixteenth);
    assert_eq!(pattern.len(), 16);
    assert!(pattern.steps[0].on, "the sixteen steps came through");
}

#[test]
fn a_short_list_pads_and_a_long_one_truncates() {
    let short = serde_json::json!({ "grid": "1/32", "steps": [] }).to_string();
    let pattern: Pattern = serde_json::from_str(&short).unwrap();
    assert_eq!(pattern.len(), 32);
    assert!(
        pattern.steps.iter().all(|step| *step == Step::rest()),
        "padded with rests"
    );

    let steps = vec![
        serde_json::json!({"on": true, "pitch": "A", "octave_offset": 1,
        "accent": false, "slide": false, "velocity": 127, "hits": 0, "accents": 0});
        64
    ];
    let long = serde_json::json!({ "grid": "1/16", "steps": steps }).to_string();
    let pattern: Pattern = serde_json::from_str(&long).unwrap();
    assert_eq!(pattern.len(), 16, "a 1/16 bar reads sixteen of them");
    for (index, step) in pattern.steps.iter().enumerate() {
        assert_eq!(step.on, index < 16, "the rest are rested, not played");
    }
}

/// The `GRID` key cycles the catalogue in `ALL`'s order and wraps: sixteenths, thirty-seconds, eighth-note triplets, and
/// round again. A full `match`, so a fourth grid added to the catalogue fails to compile here until it is placed.
#[test]
fn the_grid_key_cycles_the_catalogue() {
    assert_eq!(Grid::Sixteenth.next(), Grid::ThirtySecond);
    assert_eq!(Grid::ThirtySecond.next(), Grid::EighthTriplet);
    assert_eq!(Grid::EighthTriplet.next(), Grid::Sixteenth);
}

/// A press to a narrower grid rests whatever falls outside the new bar, and the status line counts what that cost in
/// steps that *sounded*, not slots that stopped being drawn — a rest that falls outside has cost the player nothing.
#[test]
fn a_press_counts_the_sound_it_would_drop() {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[0] = Step::note(Pitch::A, 1);
    steps[20] = Step::note(Pitch::C, 2);
    steps[21] = Step::note(Pitch::D, 2);
    steps[22] = Step::rest(); // a rest is not sound, however far outside the bar it sits
    let wide = Pattern::from_steps_in(Grid::ThirtySecond, steps);

    assert_eq!(wide.sounding_steps_beyond(Grid::EighthTriplet), 2);
    assert_eq!(
        wide.sounding_steps_beyond(Grid::ThirtySecond),
        0,
        "nothing sounds past a bar that holds it all"
    );
    assert_eq!(
        wide.with_grid(Grid::Sixteenth)
            .sounding_steps_beyond(Grid::Sixteenth),
        2,
        "narrowed to sixteenths the two notes are held, and counted as what stops playing"
    );
}

/// A note a grid change stopped playing travels through the file and plays again when a wider grid arrives; a pattern
/// with nothing held writes **no `held` key**, so its bytes are what they were before the field existed.
#[test]
fn a_held_note_travels_through_the_file_and_plays_again() {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[20] = Step::note(Pitch::C, 2);
    let wide = Pattern::from_steps_in(Grid::ThirtySecond, steps);
    let narrow = wide.with_grid(Grid::EighthTriplet);

    let json = serde_json::to_string(&narrow).unwrap();
    assert!(
        json.contains("\"held\""),
        "the held note is written: {json}"
    );
    let back: Pattern = serde_json::from_str(&json).unwrap();
    assert_eq!(back.len(), 12);
    assert!(back.steps[20].on, "the held note is restored above the bar");
    assert_eq!(
        back.with_grid(Grid::ThirtySecond),
        wide,
        "and widening plays it again"
    );

    let plain = serde_json::to_string(&Pattern::empty()).unwrap();
    assert!(
        !plain.contains("held"),
        "an untouched pattern keeps its bytes: {plain}"
    );
}
