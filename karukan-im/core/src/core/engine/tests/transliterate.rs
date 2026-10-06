//! F6–F10 transliteration: the composition in one script, selected in a
//! conversion that lists every transliteration.

use super::*;

fn type_str(engine: &mut InputMethodEngine, keys: &str) {
    for ch in keys.chars() {
        engine.process_key(&press(ch));
    }
}

fn preedit(engine: &InputMethodEngine) -> String {
    engine.preedit().unwrap().text().to_string()
}

fn committed(result: &EngineResult) -> Option<&str> {
    result.actions.iter().find_map(|a| match a {
        EngineAction::Commit(text) => Some(text.as_str()),
        _ => None,
    })
}

#[test]
fn each_key_selects_its_form() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "kyouha");

    for (key, expected) in [
        (Keysym::F7, "キョウハ"),
        (Keysym::F8, "ｷｮｳﾊ"),
        (Keysym::F9, "ｋｙｏｕｈａ"),
        (Keysym::F10, "kyouha"),
        (Keysym::F6, "きょうは"),
    ] {
        assert!(engine.process_key(&press_key(key)).consumed);
        assert!(matches!(engine.state(), InputState::Conversion { .. }));
        assert_eq!(preedit(&engine), expected, "{key:?}");
    }
}

#[test]
fn alphanumerics_are_the_keystrokes_as_typed() {
    let mut engine = InputMethodEngine::new();
    // し as `si` and ん as `nn`: a kana→romaji table would answer `shinbun`
    type_str(&mut engine, "sinnbunn1k");
    assert_eq!(preedit(&engine), "しんぶん1k");

    engine.process_key(&press_key(Keysym::F10));
    assert_eq!(preedit(&engine), "sinnbunn1k");
}

#[test]
fn repeated_alphanumeric_key_cycles_case() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "kai");

    let mut seen = Vec::new();
    for _ in 0..4 {
        engine.process_key(&press_key(Keysym::F10));
        seen.push(preedit(&engine));
    }
    assert_eq!(seen, ["kai", "KAI", "Kai", "kai"]);

    engine.process_key(&press_key(Keysym::F9));
    assert_eq!(
        preedit(&engine),
        "ｋａｉ",
        "another key starts at its own first form"
    );
}

#[test]
fn escape_returns_to_the_composition() {
    let mut engine = InputMethodEngine::new();
    type_str(&mut engine, "kak");
    engine.process_key(&press_key(Keysym::F7));
    assert_eq!(preedit(&engine), "カk");

    engine.process_key(&press_key(Keysym::ESCAPE));
    assert!(matches!(engine.state(), InputState::Composing { .. }));
    assert_eq!(preedit(&engine), "かk");
}

#[test]
fn kana_forms_are_learned_alphanumerics_are_not() {
    let mut engine = engine_with_learned("ん", "ん");

    type_str(&mut engine, "kai");
    engine.process_key(&press_key(Keysym::F10));
    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert_eq!(committed(&result), Some("kai"));
    assert!(engine.learning.as_ref().unwrap().lookup("かい").is_empty());

    type_str(&mut engine, "kai");
    engine.process_key(&press_key(Keysym::F7));
    let result = engine.process_key(&press_key(Keysym::RETURN));
    assert_eq!(committed(&result), Some("カイ"));
    let learned: Vec<String> = engine
        .learning
        .as_ref()
        .unwrap()
        .lookup("かい")
        .into_iter()
        .map(|(surface, _)| surface)
        .collect();
    assert_eq!(learned, ["カイ"]);
}

#[test]
fn empty_state_passes_function_keys_through() {
    let mut engine = InputMethodEngine::new();
    assert!(!engine.process_key(&press_key(Keysym::F7)).consumed);
}
