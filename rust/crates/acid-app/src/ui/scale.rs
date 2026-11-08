//! The scales the randomiser may draw from, all rooted on the lattice's own C.
//!
//! One root, six scales: a root selector would be a second control and a second saved field, and this instrument sits in
//! one key at a time. The catalogue is the only decoder of a scale, so a seventh entry cannot leave a stale `match`
//! behind — the same rule the wave catalogue follows.

use acid_core::pitch::Pitch;

/// A set of notes a randomise may use, as semitone offsets from C.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Scale {
    #[default]
    Chromatic,
    Major,
    Minor,
    Dorian,
    MinorPentatonic,
    MajorPentatonic,
}

impl Scale {
    /// Every scale, in the order the knob shows them.
    pub const ALL: [Scale; 6] = [
        Scale::Chromatic,
        Scale::Major,
        Scale::Minor,
        Scale::Dorian,
        Scale::MinorPentatonic,
        Scale::MajorPentatonic,
    ];

    /// Where this scale sits in [`Self::ALL`]; the save file stores this number.
    pub fn index(self) -> usize {
        match self {
            Scale::Chromatic => 0,
            Scale::Major => 1,
            Scale::Minor => 2,
            Scale::Dorian => 3,
            Scale::MinorPentatonic => 4,
            Scale::MajorPentatonic => 5,
        }
    }

    /// The scale at `index`, clamped: a file from a newer build names a scale this one lacks, and the last entry is
    /// kinder than refusing to load.
    pub fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    /// The knob caption, and the name the save file would show: ASCII, short enough for a keycap.
    pub fn name(self) -> &'static str {
        match self {
            Scale::Chromatic => "CHROM",
            Scale::Major => "MAJOR",
            Scale::Minor => "MINOR",
            Scale::Dorian => "DORIAN",
            Scale::MinorPentatonic => "MIN PEN",
            Scale::MajorPentatonic => "MAJ PEN",
        }
    }

    /// The semitones above C this scale uses, ascending, starting on 0.
    pub fn members(self) -> &'static [u8] {
        match self {
            Scale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::MinorPentatonic => &[0, 3, 5, 7, 10],
            Scale::MajorPentatonic => &[0, 2, 4, 7, 9],
        }
    }

    /// The pitches this scale uses, in the panel's own spelling.
    pub fn pitches(self) -> impl Iterator<Item = Pitch> {
        self.members()
            .iter()
            .map(|offset| Pitch::ALL[*offset as usize])
    }
}

/// A scale's position on the knob: `0.0` is the first catalogue entry, `1.0` the last.
pub fn scale_position(scale: Scale) -> f32 {
    scale.index() as f32 / (Scale::ALL.len() - 1) as f32
}

/// The scale nearest a knob position. Rounded, so the detents divide the travel evenly and the knob can never rest
/// between two scales.
pub fn scale_from_position(position: f32) -> Scale {
    let last = (Scale::ALL.len() - 1) as f32;
    Scale::from_index((position.clamp(0.0, 1.0) * last).round() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scale_starts_on_its_root_and_stays_in_the_octave() {
        for scale in Scale::ALL {
            let members = scale.members();
            assert_eq!(members[0], 0, "{scale:?} does not start on C");
            assert!(
                members.iter().all(|offset| *offset < 12),
                "{scale:?} reaches past the octave"
            );
            assert!(
                members.windows(2).all(|pair| pair[0] < pair[1]),
                "{scale:?} is not ascending"
            );
            assert_eq!(
                scale.pitches().count(),
                members.len(),
                "{scale:?} loses a pitch"
            );
        }
    }

    #[test]
    fn the_names_are_short_ascii_and_distinct() {
        let mut names: Vec<&str> = Scale::ALL.iter().map(|scale| scale.name()).collect();
        for name in &names {
            assert!(name.is_ascii() && !name.is_empty());
            assert!(name.len() <= 7, "{name} is too wide for a caption");
        }
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "two scales share a name");
    }

    #[test]
    fn the_knob_snaps_to_one_scale_per_detent() {
        for (index, scale) in Scale::ALL.iter().enumerate() {
            let position = scale_position(*scale);
            assert!((position - index as f32 / 5.0).abs() < 1e-6);
            assert_eq!(scale_from_position(position), *scale);
            assert_eq!(scale_from_position(position - 0.02), *scale);
            assert_eq!(scale_from_position(position + 0.02), *scale);
        }
        // Ties round up, and the ends clamp: a controller or a wheel that overshoots must not wrap.
        assert_eq!(scale_from_position(0.0), Scale::Chromatic);
        assert_eq!(scale_from_position(0.5), Scale::Dorian);
        assert_eq!(scale_from_position(1.0), Scale::MajorPentatonic);
        assert_eq!(scale_from_position(-2.0), Scale::Chromatic);
        assert_eq!(scale_from_position(3.0), Scale::MajorPentatonic);
    }

    #[test]
    fn a_file_from_a_newer_build_clamps_to_the_last_scale() {
        assert_eq!(Scale::from_index(99), Scale::ALL[Scale::ALL.len() - 1]);
        for (index, scale) in Scale::ALL.iter().enumerate() {
            assert_eq!(Scale::from_index(index), *scale);
            assert_eq!(scale.index(), index);
        }
    }
}
