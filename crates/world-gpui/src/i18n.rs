//! A World shown in the player's language: its snapshot's words run
//! through the catalogs once as it arrives. Presentation only; the World
//! and everything it records keep their own words.

use world_i18n::{language, tr_owned, Language};
use world_projection::ProjectionSnapshot;

/// The app's own words in Simplified Chinese.
pub const APP_ZH_HANS: &str = include_str!("../locales/zh-Hans.tsv");

fn put(text: &mut String) {
    if !text.is_empty() {
        *text = tr_owned(text);
    }
}

fn put_option(text: &mut Option<String>) {
    if let Some(text) = text {
        put(text);
    }
}

/// `snapshot` with every word the player reads in the app's language.
pub fn localize(mut snapshot: ProjectionSnapshot) -> ProjectionSnapshot {
    if language() == Language::English {
        return snapshot;
    }
    put(&mut snapshot.title);
    if let Some(briefing) = &mut snapshot.briefing {
        for item in &mut briefing.items {
            put(&mut item.title);
            put(&mut item.detail);
        }
    }
    for command in &mut snapshot.commands {
        put(&mut command.title);
        put(&mut command.detail);
        put_option(&mut command.unavailable);
        if let Some(question) = &mut command.question {
            put(&mut question.prompt);
        }
        if let Some(hand) = &mut command.hand {
            put(&mut hand.thing);
            put_option(&mut hand.cost);
        }
    }
    for item in &mut snapshot.timeline.items {
        put(&mut item.title);
        put(&mut item.subtitle);
    }
    for item in &mut snapshot.canvas.items {
        put(&mut item.label);
        put(&mut item.detail);
        if let Some(standing) = &mut item.standing {
            put(&mut standing.words);
        }
    }
    if let Some(calendar) = &mut snapshot.calendar {
        put(&mut calendar.unit);
        put_option(&mut calendar.season);
        put_option(&mut calendar.coming);
    }
    for gauge in &mut snapshot.gauges {
        put(&mut gauge.label);
        put(&mut gauge.reading);
    }
    for talk in &mut snapshot.talks {
        put(&mut talk.question);
        put(&mut talk.answer);
        put_option(&mut talk.asks_for);
    }
    for exchange in &mut snapshot.exchanges {
        put(&mut exchange.answer);
    }
    for voice in &mut snapshot.voices {
        put(&mut voice.line);
    }
    for chapter in &mut snapshot.chapters {
        put(&mut chapter.title);
        put(&mut chapter.summary);
    }
    for goal in &mut snapshot.goals {
        put(&mut goal.label);
    }
    for keepsake in &mut snapshot.keepsakes {
        put(&mut keepsake.what);
        put(&mut keepsake.note);
    }
    for entry in &mut snapshot.book {
        put(&mut entry.shelf);
        put(&mut entry.name);
        put(&mut entry.hint);
    }
    snapshot
}
