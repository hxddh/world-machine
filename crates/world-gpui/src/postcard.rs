//! Postcards: a moment of a World as a card, with a line a resident said
//! at that moment as its caption, the World's name and the day.
//!
//! What the card says is chosen here, from the snapshot alone, so it can be
//! tested without a window; the window draws the paper around the scene and
//! saves the picture. A postcard is presentation: it reads the snapshot's
//! narration and changes nothing.

use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem};

/// The paper a postcard is printed on, and its ink: the same in light and
/// dark, because a postcard is a thing, not a window.
pub const PAPER: u32 = 0xf5eedc;
pub const INK: u32 = 0x3a3226;
pub const INK_SOFT: u32 = 0x7a6d58;

/// What a postcard says.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Postcard {
    /// The World's name.
    pub world: String,
    /// The day it shows: "Day 12", or "The beginning".
    pub day: String,
    /// The line on the card: something a resident said, or, when nobody
    /// spoke, what happened.
    pub caption: String,
    /// Who said the caption, when someone did.
    pub speaker: Option<String>,
    /// The moment the card is of, when the World has one.
    pub moment: Option<SelectionId>,
}

impl Postcard {
    /// The caption as printed: a resident's words in quotes, a happening
    /// as it is.
    pub fn printed_caption(&self) -> String {
        match &self.speaker {
            Some(_) => format!("“{}”", self.caption.trim()),
            None => self.caption.trim().to_string(),
        }
    }

    /// The line under the caption: who said it, then the World and the day.
    pub fn signature(&self) -> String {
        let place = format!("{} · {}", self.world.trim(), self.day);
        match &self.speaker {
            Some(speaker) => format!("— {speaker}, {place}"),
            None => place,
        }
    }

    /// The name the card is saved under, without a folder: the World, the
    /// day, and "postcard", with nothing a file system could trip on.
    pub fn file_stem(&self) -> String {
        let raw = format!("{} {} postcard", self.world.trim(), self.day);
        let cleaned = raw
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>();
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// The postcard of `moment` (a timeline item) in `snapshot`, or of the
/// scene now when there is no moment or it is not in the World's history.
///
/// The caption is the first thing a resident said at that moment, the
/// story before the everyday round; if nobody said anything, the moment's
/// own title. "Now" is the latest moment someone spoke at today, else the
/// latest thing that happened today, else the latest thing that happened.
pub fn postcard(snapshot: &ProjectionSnapshot, moment: Option<SelectionId>) -> Postcard {
    let items = &snapshot.timeline.items;
    let chosen = moment
        .and_then(|moment| items.iter().find(|item| item.id == moment))
        .or_else(|| moment_now(snapshot));
    let voice = chosen.and_then(|item| {
        snapshot
            .voices
            .iter()
            .find(|voice| voice.moment == item.id && !voice.line.trim().is_empty())
    });
    let world_time = chosen.map_or(snapshot.world_time, |item| item.world_time);
    let (caption, speaker) = match (voice, chosen) {
        (Some(voice), _) => (voice.line.clone(), Some(name_of(snapshot, voice.speaker))),
        (None, Some(item)) => (item.title.clone(), None),
        (None, None) => (snapshot.title.clone(), None),
    };
    Postcard {
        world: snapshot.title.clone(),
        day: snapshot.moment_label(world_time),
        caption,
        speaker: speaker.map(|name| name.unwrap_or_else(|| "someone here".into())),
        moment: chosen.map(|item| item.id),
    }
}

/// The moment the scene shows now.
fn moment_now(snapshot: &ProjectionSnapshot) -> Option<&TimelineItem> {
    let items = &snapshot.timeline.items;
    let today = |item: &&TimelineItem| item.world_time == snapshot.world_time;
    let spoken = |item: &&TimelineItem| {
        snapshot
            .voices
            .iter()
            .any(|voice| voice.moment == item.id && !voice.line.trim().is_empty())
    };
    // The story before the everyday, and the newest of each.
    let newest = |routine: bool, voiced: bool| {
        items
            .iter()
            .filter(today)
            .filter(|item| item.routine == routine && (!voiced || spoken(item)))
            .max_by_key(|item| item.world_time)
    };
    newest(false, true)
        .or_else(|| newest(true, true))
        .or_else(|| newest(false, false))
        .or_else(|| newest(true, false))
        .or_else(|| {
            items
                .iter()
                .filter(|item| !item.routine)
                .max_by_key(|item| item.world_time)
        })
        .or_else(|| items.iter().max_by_key(|item| item.world_time))
}

fn name_of(snapshot: &ProjectionSnapshot, id: SelectionId) -> Option<String> {
    snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.clone())
        .filter(|label| !label.trim().is_empty())
}

/// Where a postcard's paper goes over a picture `width` by `height`: an
/// even border, and a deeper band at the foot for the caption, sized to
/// the picture so a small window still gets a card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PostcardLayout {
    pub border: f32,
    pub band: f32,
    /// The scene's window in the card.
    pub scene_width: f32,
    pub scene_height: f32,
}

pub fn postcard_layout(width: f32, height: f32) -> PostcardLayout {
    let short = width.min(height).max(0.0);
    let border = (short * 0.045).clamp(10.0, 36.0);
    let band = (height * 0.2).clamp(72.0, 150.0);
    PostcardLayout {
        border,
        band,
        scene_width: (width - 2.0 * border).max(0.0),
        scene_height: (height - border - band).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::{Calendar, CanvasItem, CanvasItemKind, TimelineProjection, Voice};

    fn id(n: u64) -> SelectionId {
        SelectionId::from_stable_key(&format!("entity-{n}")).unwrap()
    }

    fn event(n: u64) -> SelectionId {
        SelectionId::from_stable_key(&format!("event-{n}")).unwrap()
    }

    fn moment(n: u64, world_time: u64, title: &str, routine: bool) -> TimelineItem {
        TimelineItem {
            id: event(n),
            world_time,
            title: title.into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
            routine,
        }
    }

    fn person(n: u64, label: &str) -> CanvasItem {
        CanvasItem {
            id: id(n),
            kind: CanvasItemKind::Actor,
            label: label.into(),
            detail: String::new(),
            x: 0.5,
            y: 0.5,
            changes: Vec::new(),
            shape: None,
            at: None,
            look: None,
            drawing: None,
            stance: None,
            standing: None,
            mood: None,
            spot: None,
        }
    }

    fn town() -> ProjectionSnapshot {
        ProjectionSnapshot {
            title: "Harbor Town".into(),
            world_time: 3,
            calendar: Some(Calendar {
                unit: "Day".into(),
                length: 1,
                season: None,
                coming: None,
                festival_today: false,
            }),
            timeline: TimelineProjection {
                items: vec![
                    moment(1, 1, "The bakery opened", false),
                    moment(2, 2, "Mara fixed the pier", false),
                    moment(3, 3, "Leo worked a shift", true),
                    moment(4, 3, "The boats came home", false),
                ],
            },
            voices: vec![
                Voice {
                    moment: event(2),
                    speaker: id(7),
                    line: "The pier holds! Come and see.".into(),
                },
                Voice {
                    moment: event(3),
                    speaker: id(8),
                    line: "Another long shift.".into(),
                },
                Voice {
                    moment: event(4),
                    speaker: id(7),
                    line: "Everyone's home safe tonight.".into(),
                },
            ],
            canvas: world_projection::CanvasProjection {
                items: vec![person(7, "Mara Quill"), person(8, "Leo Park")],
                ..Default::default()
            },
            ..ProjectionSnapshot::default()
        }
    }

    #[test]
    fn a_moment_is_captioned_with_what_a_resident_said_then() {
        let card = postcard(&town(), Some(event(2)));
        assert_eq!(card.caption, "The pier holds! Come and see.");
        assert_eq!(card.speaker.as_deref(), Some("Mara Quill"));
        assert_eq!(card.world, "Harbor Town");
        assert_eq!(card.day, "Day 2");
        assert_eq!(card.moment, Some(event(2)));
        assert_eq!(card.printed_caption(), "“The pier holds! Come and see.”");
        assert_eq!(card.signature(), "— Mara Quill, Harbor Town · Day 2");
    }

    #[test]
    fn a_moment_nobody_spoke_at_is_captioned_with_what_happened() {
        let card = postcard(&town(), Some(event(1)));
        assert_eq!(card.caption, "The bakery opened");
        assert_eq!(card.speaker, None);
        assert_eq!(card.day, "Day 1");
        assert_eq!(card.printed_caption(), "The bakery opened");
        assert_eq!(card.signature(), "Harbor Town · Day 1");
    }

    #[test]
    fn the_scene_now_prefers_todays_story_over_the_everyday() {
        let card = postcard(&town(), None);
        assert_eq!(card.moment, Some(event(4)));
        assert_eq!(card.caption, "Everyone's home safe tonight.");
        assert_eq!(card.speaker.as_deref(), Some("Mara Quill"));
        assert_eq!(card.day, "Day 3");
        // A moment that is not in the history is the scene now too.
        assert_eq!(postcard(&town(), Some(id(99))), card);
    }

    #[test]
    fn with_nobody_speaking_today_the_latest_happening_is_the_caption() {
        let mut quiet = town();
        quiet.voices.clear();
        let card = postcard(&quiet, None);
        assert_eq!(card.caption, "The boats came home");
        assert_eq!(card.speaker, None);

        // No history at all: the World's own name, at the beginning.
        let empty = ProjectionSnapshot {
            title: "A new World".into(),
            ..ProjectionSnapshot::default()
        };
        let card = postcard(&empty, None);
        assert_eq!(card.caption, "A new World");
        assert_eq!(card.day, "The beginning");
        assert_eq!(card.moment, None);
    }

    #[test]
    fn a_speaker_who_is_not_on_the_scene_is_still_quoted() {
        let mut town = town();
        town.canvas.items.clear();
        let card = postcard(&town, Some(event(2)));
        assert_eq!(card.speaker.as_deref(), Some("someone here"));
        assert!(card.printed_caption().starts_with('“'));
    }

    #[test]
    fn a_postcard_is_saved_under_a_plain_name() {
        let mut card = postcard(&town(), Some(event(2)));
        assert_eq!(card.file_stem(), "Harbor Town Day 2 postcard");
        card.world = "Ares/Pocket: Colony".into();
        assert_eq!(card.file_stem(), "Ares-Pocket- Colony Day 2 postcard");
    }

    #[test]
    fn the_paper_leaves_the_scene_most_of_the_card() {
        for (width, height) in [(1100.0, 848.0), (640.0, 400.0), (2560.0, 1400.0)] {
            let layout = postcard_layout(width, height);
            assert!(layout.border >= 10.0 && layout.border <= 36.0);
            assert!(layout.band > layout.border, "the caption band is deeper");
            let scene = layout.scene_width * layout.scene_height;
            assert!(
                scene / (width * height) > 0.6,
                "{width}x{height}: the scene is most of the card"
            );
        }
        let tiny = postcard_layout(10.0, 10.0);
        assert_eq!((tiny.scene_width, tiny.scene_height), (0.0, 0.0));
    }
}
