//! F6–F10: the composition rewritten in one script, without the model.

use karukan_engine::kana::katakana_to_half_width;
use karukan_engine::width::{to_full_width, to_half_width};
use karukan_engine::{hiragana_to_katakana, katakana_to_hiragana};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Transliteration {
    Hiragana,
    FullKatakana,
    HalfKatakana,
    FullAlphanumeric,
    HalfAlphanumeric,
}

impl Transliteration {
    const ALL: [Self; 5] = [
        Self::Hiragana,
        Self::FullKatakana,
        Self::HalfKatakana,
        Self::FullAlphanumeric,
        Self::HalfAlphanumeric,
    ];

    pub(super) fn for_key(key: &KeyEvent) -> Option<Self> {
        if key.modifiers.any() {
            return None;
        }
        match key.keysym {
            Keysym::F6 => Some(Self::Hiragana),
            Keysym::F7 => Some(Self::FullKatakana),
            Keysym::F8 => Some(Self::HalfKatakana),
            Keysym::F9 => Some(Self::FullAlphanumeric),
            Keysym::F10 => Some(Self::HalfAlphanumeric),
            _ => None,
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Hiragana => "[全]ひらがな",
            Self::FullKatakana => "[全]カタカナ",
            Self::HalfKatakana => "[半]カタカナ",
            Self::FullAlphanumeric => "[全]英数",
            Self::HalfAlphanumeric => "[半]英数",
        }
    }

    fn source(self) -> CandidateSource {
        match self {
            Self::Hiragana | Self::FullKatakana | Self::HalfKatakana => CandidateSource::Fallback,
            Self::FullAlphanumeric | Self::HalfAlphanumeric => CandidateSource::Transliteration,
        }
    }

    /// The forms this key steps through: the alphanumerics cycle their
    /// case (as typed → lower → UPPER → Capitalized), the kana have one.
    fn variants(self, reading: &str, keystrokes: &str) -> Vec<String> {
        let mut variants = match self {
            Self::Hiragana => vec![katakana_to_hiragana(reading)],
            Self::FullKatakana => vec![hiragana_to_katakana(reading)],
            Self::HalfKatakana => vec![katakana_to_half_width(&hiragana_to_katakana(reading))],
            Self::FullAlphanumeric => case_variants(keystrokes)
                .iter()
                .map(|v| to_full_width(v).replace(' ', "\u{3000}"))
                .collect(),
            Self::HalfAlphanumeric => {
                case_variants(&to_half_width(keystrokes).replace('\u{3000}', " "))
            }
        };
        let mut seen = HashSet::new();
        variants.retain(|v| !v.is_empty() && seen.insert(v.clone()));
        variants
    }
}

fn case_variants(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut chars = lower.chars();
    let capitalized = chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    vec![
        text.to_string(),
        lower.clone(),
        text.to_uppercase(),
        capitalized,
    ]
}

impl InputMethodEngine {
    /// Open a conversion whose list is the transliterations of the
    /// composition, with `form` selected. Pressing the same key again while
    /// that list is shown steps to the key's next variant.
    pub(super) fn transliterate(&mut self, form: Transliteration) -> EngineResult {
        let reading = self.input_buf.settled_reading(&self.converters.romaji);
        if reading.is_empty() {
            return EngineResult::consumed();
        }
        let keystrokes = self.input_buf.keystrokes();
        let groups: Vec<(Transliteration, Vec<String>)> = Transliteration::ALL
            .into_iter()
            .map(|t| (t, t.variants(&reading, &keystrokes)))
            .collect();
        let mut candidates = self.settle_candidates(
            groups
                .iter()
                .flat_map(|(t, variants)| {
                    variants.iter().map(|text| Candidate {
                        text: text.clone(),
                        reading: Some(reading.clone()),
                        source: Some(t.source()),
                        description: Some(t.description().to_string()),
                    })
                })
                .collect(),
        );

        let variants = form.variants(&reading, &keystrokes);
        let shown = self.state.candidates().filter(|shown| {
            shown
                .candidates()
                .iter()
                .map(|c| &c.text)
                .eq(candidates.candidates().iter().map(|c| &c.text))
        });
        let step = shown
            .and_then(CandidateList::selected_text)
            .and_then(|text| variants.iter().position(|v| v == text))
            .map_or(0, |i| (i + 1) % variants.len());
        let Some(target) = variants.get(step) else {
            return EngineResult::consumed();
        };
        if let Some(index) = candidates
            .candidates()
            .iter()
            .position(|c| &c.text == target)
        {
            candidates.set_cursor(index);
        }
        self.enter_conversion_state(&reading, candidates)
    }
}
