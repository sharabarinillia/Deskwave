//! Languages Deskwave ships in, and the few strings Rust shows itself (the tray menu).
//! Everything else is translated in the pages (ui/js/strings.js) and in each mod.

/// English first: it is the fallback.
pub const LANGUAGES: [&str; 7] = ["en", "uk", "fr", "nl", "de", "pl", "es"];

/// The user's choice if supported, else the first supported Windows display language
/// (`uk-UA` → `uk`), else English.
pub fn resolve(chosen: Option<&str>, system: &[String]) -> &'static str {
    let supported = |code: &str| LANGUAGES.iter().copied().find(|l| l.eq_ignore_ascii_case(code));
    chosen
        .and_then(supported)
        .or_else(|| system.iter().find_map(|tag| supported(tag.split(['-', '_']).next().unwrap_or(""))))
        .unwrap_or("en")
}

pub struct TrayText {
    pub edit: &'static str,
    pub pause: &'static str,
    pub open: &'static str,
    pub quit: &'static str,
}

pub fn tray(lang: &str) -> TrayText {
    let (edit, pause, open, quit) = match lang {
        "uk" => ("Редагувати віджети", "Призупинити анімацію", "Відкрити Deskwave", "Вийти з Deskwave"),
        "fr" => ("Modifier les widgets", "Mettre les animations en pause", "Ouvrir Deskwave", "Quitter Deskwave"),
        "nl" => ("Widgets bewerken", "Animaties pauzeren", "Deskwave openen", "Deskwave afsluiten"),
        "de" => ("Widgets bearbeiten", "Animationen anhalten", "Deskwave öffnen", "Deskwave beenden"),
        "pl" => ("Edytuj widżety", "Wstrzymaj animacje", "Otwórz Deskwave", "Zamknij Deskwave"),
        "es" => ("Editar widgets", "Pausar animaciones", "Abrir Deskwave", "Salir de Deskwave"),
        _ => ("Edit widgets", "Pause animations", "Open Deskwave", "Quit Deskwave"),
    };
    TrayText { edit, pause, open, quit }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_choice_then_system_then_english() {
        let system = vec!["nl-BE".to_string(), "en-US".to_string()];
        assert_eq!(resolve(Some("pl"), &system), "pl");
        assert_eq!(resolve(Some("xx"), &system), "nl");
        assert_eq!(resolve(None, &["uk-UA".to_string()]), "uk");
        assert_eq!(resolve(None, &["ja-JP".to_string()]), "en");
        assert_eq!(resolve(None, &[]), "en");
    }

    #[test]
    fn every_language_has_tray_text() {
        for lang in LANGUAGES {
            let text = tray(lang);
            assert!(!text.edit.is_empty() && !text.quit.is_empty());
            assert!(lang == "en" || text.edit != tray("en").edit, "{lang} falls back to English");
        }
    }
}
