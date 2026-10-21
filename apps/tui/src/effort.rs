//! Reading pace, dressed as an agent's reasoning effort.
//!
//! A coding agent lets you pick how hard the model should think, and the honest
//! consequence of asking for more is that output arrives more slowly. readio
//! borrows the control and keeps the consequence: `minimal` skims, `max` savours.
//! Six levels, the same six a coding agent offers — and, because a reader's own
//! pace rarely lands on one of them, a seventh thing that is not a level at all:
//! `/effort 1.25` is whatever number they typed.
//!
//! Every level is a **multiplier**, and one multiplier drives both worlds:
//! the reveal speed is `reading.speed × multiplier`, and read-aloud plays at
//! `multiplier` — so "slower" means the same thing whether the text is being
//! typed out or spoken. The numbers live in `~/.readio/config.yaml`, because the
//! pace someone reads at is not something a program should have opinions about.

use serde::{Deserialize, Serialize};

use crate::i18n::t;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    /// A multiplier the reader typed instead of choosing a rung: `/effort 1.25`.
    ///
    /// Deliberately not in [`LEVELS`]. The six rungs are a ladder you can walk
    /// with `^r`; this is one pace of the reader's own, and putting it in the
    /// ladder would mean a key that means "one step slower" sometimes landing
    /// on a number nobody chose. It keeps its own slot in the config so the
    /// number is still there the next time they reach for it.
    Custom,
}

/// Every level, fastest first, which is the order `/effort` lists them in.
///
/// Six, and only six: `custom` is a pace rather than a rung, so it is offered
/// beside the ladder only while it is the one in force.
pub const LEVELS: [Effort; 6] = [
    Effort::Minimal,
    Effort::Low,
    Effort::Medium,
    Effort::High,
    Effort::Xhigh,
    Effort::Max,
];

impl Default for Effort {
    /// `high`, whose multiplier is 1.0: the pace readio has always read at.
    fn default() -> Self {
        Effort::High
    }
}

impl Effort {
    /// The spelling used in the config file, on the command line and in the menu.
    pub fn code(self) -> &'static str {
        match self {
            Effort::Minimal => "minimal",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Xhigh => "xhigh",
            Effort::Max => "max",
            Effort::Custom => "custom",
        }
    }

    /// `Minimal`, the way a menu of levels writes it.
    pub fn label(self) -> &'static str {
        match self {
            Effort::Minimal => "Minimal",
            Effort::Low => "Low",
            Effort::Medium => "Medium",
            Effort::High => "High",
            Effort::Xhigh => "Xhigh",
            Effort::Max => "Max",
            Effort::Custom => "Custom",
        }
    }

    /// What choosing this level means for a reader, in their own language.
    pub fn about(self) -> &'static str {
        match self {
            Effort::Minimal => t("effort.minimal"),
            Effort::Low => t("effort.low"),
            Effort::Medium => t("effort.medium"),
            Effort::High => t("effort.high"),
            Effort::Xhigh => t("effort.xhigh"),
            Effort::Max => t("effort.max"),
            Effort::Custom => t("effort.custom"),
        }
    }

    /// Generous about spelling: a level should never be refused over a case or a
    /// hyphen, and `x-high`, `xhigh` and `extra` are the same idea.
    pub fn parse(arg: &str) -> Option<Self> {
        let key: String = arg
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        match key.as_str() {
            "minimal" | "min" | "skim" | "0" => Some(Effort::Minimal),
            "low" | "1" => Some(Effort::Low),
            "medium" | "med" | "mid" | "2" => Some(Effort::Medium),
            "high" | "normal" | "3" => Some(Effort::High),
            "xhigh" | "extrahigh" | "extra" | "veryhigh" | "4" => Some(Effort::Xhigh),
            "max" | "maximum" | "ultra" | "5" => Some(Effort::Max),
            // The number the reader typed last, spelled out. `/effort 1.25`
            // chooses it; this is how they come back to it afterwards.
            "custom" | "own" | "mine" => Some(Effort::Custom),
            _ => None,
        }
    }

    /// The next level `^r` should move to, wrapping at the top.
    ///
    /// From a custom pace there is no "next rung" to look up, so the ladder
    /// answers instead: the nearest named rung at or below the number typed.
    /// A custom pace slower than every rung wraps to the fastest one, which is
    /// what `^r` does at the bottom of the ladder as well.
    pub fn next(self, ladder: &Ladder) -> Self {
        if self == Effort::Custom {
            return ladder
                .rung_at_or_below(ladder.get(Effort::Custom))
                .unwrap_or(LEVELS[0]);
        }
        let at = LEVELS.iter().position(|l| *l == self).unwrap_or(3);
        LEVELS[(at + 1) % LEVELS.len()]
    }
}

/// The multiplier each level stands for, straight from the config file.
///
/// Higher effort is slower, which is why the defaults descend. Nothing enforces
/// that order: a reader who wants `max` to mean "fastest" is allowed to be wrong
/// in their own config.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ladder {
    pub minimal: f32,
    pub low: f32,
    pub medium: f32,
    pub high: f32,
    pub xhigh: f32,
    pub max: f32,
    /// The pace last typed as a number, `/effort 1.25`. Not a rung: it has a
    /// slot of its own so the number is still there tomorrow, and so choosing
    /// a named level never overwrites it.
    pub custom: f32,
}

impl Default for Ladder {
    fn default() -> Self {
        Self {
            minimal: 2.5,
            low: 2.0,
            medium: 1.5,
            high: 1.0,
            xhigh: 0.85,
            max: 0.7,
            // Halfway between `high` and `medium`, where a reader who finds one
            // too slow and the other too brisk tends to land.
            custom: 1.25,
        }
    }
}

impl Ladder {
    pub fn get(&self, level: Effort) -> f32 {
        let raw = match level {
            Effort::Minimal => self.minimal,
            Effort::Low => self.low,
            Effort::Medium => self.medium,
            Effort::High => self.high,
            Effort::Xhigh => self.xhigh,
            Effort::Max => self.max,
            Effort::Custom => self.custom,
        };
        clamp(raw)
    }

    pub fn set(&mut self, level: Effort, multiplier: f32) {
        let value = clamp(multiplier);
        match level {
            Effort::Minimal => self.minimal = value,
            Effort::Low => self.low = value,
            Effort::Medium => self.medium = value,
            Effort::High => self.high = value,
            Effort::Xhigh => self.xhigh = value,
            Effort::Max => self.max = value,
            Effort::Custom => self.custom = value,
        }
    }

    /// The named rung closest to `pace` without reading faster than it — the one
    /// `^r` steps onto from a custom number, since a smaller multiplier reads
    /// more slowly.
    ///
    /// `None` when the custom pace is slower than every rung, which is the
    /// caller's cue to wrap.
    fn rung_at_or_below(&self, pace: f32) -> Option<Effort> {
        LEVELS
            .iter()
            .copied()
            .filter(|level| self.get(*level) <= pace)
            .max_by(|a, b| self.get(*a).total_cmp(&self.get(*b)))
    }

    /// Pull hand-edited numbers into a range that still produces sound.
    pub fn clamp_all(&mut self) {
        for level in LEVELS {
            let value = self.get(level);
            self.set(level, value);
        }
        // `custom` is not a rung, so the loop above walks past it.
        self.set(Effort::Custom, self.custom);
    }
}

/// The range a multiplier may take: what the player, the pacer and `/rate`
/// can all still make sense of.
pub const MIN: f32 = 0.5;
pub const MAX: f32 = 3.0;

/// A number typed where a level goes: `/effort 1.25`, `1.25x`, `1.25×`.
///
/// Named spellings are checked first by the caller, so the bare integers `1`–`5`
/// keep meaning the rungs they always meant; a decimal is a pace of its own.
/// Out-of-range numbers are refused rather than clamped — quietly turning `9`
/// into `3` would be a lie about what the reader asked for.
pub fn parse_multiplier(arg: &str) -> Option<f32> {
    let number = arg.trim().trim_end_matches(['x', 'X', '×']).trim();
    let value: f32 = number.parse().ok()?;
    (value.is_finite() && (MIN..=MAX).contains(&value)).then_some(value)
}

/// The embedded player and the text pacer share [`MIN`]..=[`MAX`], and a reveal
/// speed of zero is a hang, so every multiplier lands inside the same range
/// `/rate` accepts.
fn clamp(multiplier: f32) -> f32 {
    if multiplier.is_finite() {
        multiplier.clamp(MIN, MAX)
    } else {
        1.0
    }
}

/// `1.5×`, as the interface writes a multiplier.
pub fn times(multiplier: f32) -> String {
    let trimmed = format!("{multiplier:.2}");
    let trimmed = trimmed.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed}×")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_level_changes_nothing() {
        assert_eq!(Effort::default(), Effort::High);
        assert_eq!(Ladder::default().get(Effort::High), 1.0);
    }

    #[test]
    fn more_effort_means_a_slower_read() {
        let ladder = Ladder::default();
        let speeds: Vec<f32> = LEVELS.iter().map(|level| ladder.get(*level)).collect();
        for pair in speeds.windows(2) {
            assert!(pair[0] > pair[1], "{speeds:?} should descend");
        }
    }

    #[test]
    fn every_spelling_a_reader_might_try_lands() {
        assert_eq!(Effort::parse("Minimal"), Some(Effort::Minimal));
        assert_eq!(Effort::parse("  MIN "), Some(Effort::Minimal));
        assert_eq!(Effort::parse("x-high"), Some(Effort::Xhigh));
        assert_eq!(Effort::parse("extra high"), Some(Effort::Xhigh));
        assert_eq!(Effort::parse("ultra"), Some(Effort::Max));
        assert_eq!(Effort::parse("3"), Some(Effort::High));
        assert_eq!(Effort::parse("custom"), Some(Effort::Custom));
        assert_eq!(Effort::parse("sideways"), None);
    }

    #[test]
    fn ctrl_r_visits_every_level_and_comes_back() {
        let ladder = Ladder::default();
        let mut level = Effort::Minimal;
        let mut seen = vec![level];
        for _ in 0..LEVELS.len() {
            level = level.next(&ladder);
            seen.push(level);
        }
        assert_eq!(seen.first(), seen.last(), "the ladder should wrap");
        assert_eq!(seen.len(), LEVELS.len() + 1);
        assert!(
            !seen.contains(&Effort::Custom),
            "a number nobody chose is not a rung to step through"
        );
    }

    #[test]
    fn a_typed_number_is_a_pace_of_its_own() {
        let mut ladder = Ladder::default();
        ladder.set(Effort::Custom, 1.25);
        assert_eq!(ladder.get(Effort::Custom), 1.25);
        // Choosing a named rung leaves the reader's own number alone.
        ladder.set(Effort::High, 0.9);
        assert_eq!(ladder.get(Effort::Custom), 1.25);
    }

    #[test]
    fn ctrl_r_from_a_custom_pace_lands_on_the_rung_at_or_below_it() {
        let mut ladder = Ladder::default();
        ladder.set(Effort::Custom, 1.25);
        assert_eq!(
            Effort::Custom.next(&ladder),
            Effort::High,
            "1.0 is the next rung down"
        );

        // Exactly on a rung: that rung is the one it means, so the step is a
        // change of label rather than a jump past the reader's own number.
        ladder.set(Effort::Custom, 1.5);
        assert_eq!(Effort::Custom.next(&ladder), Effort::Medium);

        // Slower than every rung: wrap, the same way `max` wraps to `minimal`.
        ladder.set(Effort::Custom, 0.55);
        assert_eq!(Effort::Custom.next(&ladder), Effort::Minimal);
    }

    #[test]
    fn a_typed_number_must_be_a_number_that_can_be_read_at() {
        assert_eq!(parse_multiplier("1.25"), Some(1.25));
        assert_eq!(parse_multiplier(" 1.25× "), Some(1.25));
        assert_eq!(parse_multiplier("0.8x"), Some(0.8));
        assert_eq!(parse_multiplier("0.5"), Some(0.5));
        assert_eq!(parse_multiplier("3"), Some(3.0));
        // Refused, not clamped: the reader asked for something else.
        assert_eq!(parse_multiplier("0.4"), None);
        assert_eq!(parse_multiplier("9"), None);
        assert_eq!(parse_multiplier("NaN"), None);
        assert_eq!(parse_multiplier("xhigh"), None);
    }

    #[test]
    fn a_hand_edited_impossible_multiplier_is_pulled_into_range() {
        let mut ladder = Ladder {
            minimal: 99.0,
            max: -1.0,
            custom: 0.1,
            ..Ladder::default()
        };
        ladder.clamp_all();
        assert_eq!(ladder.get(Effort::Minimal), MAX);
        assert_eq!(ladder.get(Effort::Max), MIN);
        assert_eq!(ladder.get(Effort::Custom), MIN);
    }

    #[test]
    fn multipliers_are_written_the_way_a_reader_says_them() {
        assert_eq!(times(1.0), "1×");
        assert_eq!(times(1.25), "1.25×");
        assert_eq!(times(0.7), "0.7×");
    }
}
