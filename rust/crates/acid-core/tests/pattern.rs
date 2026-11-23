use acid_core::pattern::{Grid, Pattern, PatternId, STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;

#[test]
fn a_new_pattern_is_sixteen_rests() {
    let p = Pattern::empty();
    assert_eq!(p.steps.len(), STEPS_PER_PATTERN);
    assert!(p.steps.iter().all(|s| !s.on && !s.accent && !s.slide));
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
