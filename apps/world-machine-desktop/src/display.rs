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

/// Whether the system asks for less motion.
pub fn system_reduce_motion() -> bool {
    crate::platform::current().reduce_motion()
}

/// Whether the scene holds still: the player's choice, or the system's.
/// The window applies it with GPUI's `set_reduce_motion`, which needs the
/// app, so it is not part of [`apply`].
pub fn reduce_motion(settings: Option<&AppSettings>) -> bool {
    settings
        .and_then(|settings| settings.reduce_motion)
        .unwrap_or_else(system_reduce_motion)
}

/// The most frames a second a World window draws (`None`: the display's
/// rate); 60 when nothing is chosen.
pub fn frame_cap(settings: Option<&AppSettings>) -> Option<u32> {
    settings.map_or(Some(60), AppSettings::frame_cap)
}

/// Shows the app as the player chose.
pub fn apply(settings: Option<&AppSettings>) {
    world_gpui::frames::set_frame_cap(frame_cap(settings));
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
    // The demo plays freely: its second morning comes at once.
    world_gpui::ui::set_play_freely(crate::demo::plays_freely(
        settings.is_some_and(|settings| settings.play_freely),
    ));
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

    #[test]
    fn motion_and_frames_follow_the_player_then_the_system() {
        let mut settings = AppSettings::empty();
        assert_eq!(frame_cap(None), Some(60));
        assert_eq!(frame_cap(Some(&settings)), Some(60));
        settings.frame_rate = Some(crate::app_settings::FrameRate::Display);
        assert_eq!(frame_cap(Some(&settings)), None);
        settings.battery_saver = true;
        assert_eq!(frame_cap(Some(&settings)), Some(30));
        assert_eq!(reduce_motion(None), system_reduce_motion());
        settings.reduce_motion = Some(true);
        assert!(reduce_motion(Some(&settings)));
        settings.reduce_motion = Some(false);
        assert!(!reduce_motion(Some(&settings)));
    }
}
