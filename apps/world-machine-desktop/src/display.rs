//! How the app is shown: in which language, how large its text is, and
//! with how much contrast. What the player chose in Settings wins; where
//! they chose nothing, the Mac's own settings decide.

use crate::app_settings::AppSettings;
use world_gpui::Language;

/// The language the app is shown in, from what the player chose or else
/// the Mac.
pub fn language(settings: Option<&AppSettings>) -> Language {
    settings
        .and_then(|settings| settings.language.as_deref())
        .and_then(Language::from_id)
        .unwrap_or_else(world_i18n::system_language)
}

/// Whether the Mac asks for more contrast.
pub fn system_increase_contrast() -> bool {
    crate::platform::current().increase_contrast()
}

/// Shows the app as the player chose.
pub fn apply(settings: Option<&AppSettings>) {
    world_gpui::set_language(language(settings));
    world_gpui::set_text_scale(
        settings
            .and_then(|settings| settings.text_scale)
            .unwrap_or(100),
    );
    world_gpui::ui::set_increase_contrast(
        settings
            .and_then(|settings| settings.increase_contrast)
            .unwrap_or_else(system_increase_contrast),
    );
    world_gpui::ui::set_developer(settings.is_some_and(|settings| settings.developer));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_the_player_chose_wins() {
        let mut settings = AppSettings::empty();
        settings.language = Some("zh-Hans".into());
        assert_eq!(language(Some(&settings)), Language::SimplifiedChinese);
        settings.language = Some("ja".into());
        assert_eq!(language(Some(&settings)), Language::Japanese);
        settings.language = Some("en".into());
        assert_eq!(language(Some(&settings)), Language::English);
        settings.text_scale = Some(175);
        settings.increase_contrast = Some(true);
        apply(Some(&settings));
        assert!((world_gpui::text_scale() - 1.75).abs() < 1e-6);
        assert!(world_gpui::ui::increase_contrast());
        apply(None);
        assert!((world_gpui::text_scale() - 1.0).abs() < 1e-6);
        world_gpui::ui::set_increase_contrast(false);
        world_gpui::set_language(Language::English);
    }
}
