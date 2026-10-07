//! Talking to the harbour's people in the player's own words: what the
//! conversation System needs to know about the harbour, and the harbour's
//! own words for its places, its work and its needs.

use crate::model::OPERATING_STATUS;
use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use conversation::faces::in_chinese;
use society_basic::JOB;
use world_core::{ActionRequest, EntityId, Value, World, WorldState};

fn text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        _ => None,
    }
}

/// What the harbour's people speak of that is not on the scene, as the
/// harbour's own lines name it in English and in Chinese: the mainland
/// over the water, the boats, everyone who lives or will live here, and
/// the people they remember.
const ELSEWHERE: &[&str] = &[
    "the mainland",
    "大陆",
    "Sea Finch",
    "海雀号",
    "Kittiwake",
    "三趾鸥号",
    "Brave Molly",
    "勇敢的莫莉号",
    "Jonas",
    "Mara",
    "Leo",
    "Emma",
    "Mia",
    "Noah",
    "Evan",
    "Sofia",
    "Ivo",
    "Ada",
    "Old Tam",
    "老Tam",
    "Clark",
    "Bess",
    "Pike",
];

pub(crate) fn kit(_: &WorldState) -> conversation::Kit {
    conversation::Kit {
        era: conversation::Era::Radio,
        elsewhere: ELSEWHERE,
        period: crate::persistence::WORLD_DAY_TICKS,
        unit: "day",
        settlement: "the harbour",
        people: crate::story::people_in,
        places: |state| {
            [HARBOR, BAKERY, PUB, SCHOOL]
                .into_iter()
                .filter(|place| state.entity(*place).is_some())
                .collect()
        },
        place_line,
        need_line,
        work_line,
        place_mood: crate::talk::harbour_mood,
        coming_up: |world| {
            let almanac = crate::almanac::almanac(world.state());
            calendar::coming_up(world.state(), &almanac, 7)
        },
        aliases,
        weather,
        occasions: |state| {
            crate::almanac::almanac(state)
                .festivals
                .iter()
                .map(|festival| festival.name.to_string())
                .collect()
        },
        recalled: |_, event, who| crate::story::recalled(event, who),
    }
}

/// The other names the harbour's people and places go by.
fn aliases(name: &str) -> Vec<String> {
    let names: &[&str] = match name {
        "Jonas" => &["乔纳斯", "ジョナス"],
        "Mara" => &["玛拉", "マーラ", "マラ"],
        "Leo" => &["利奥", "里奥", "レオ"],
        "Emma" => &["艾玛", "エマ"],
        "Mia" => &["米娅", "米亚", "ミア"],
        "Noah" => &["诺亚", "ノア"],
        "Evan" => &["埃文", "エヴァン", "エバン"],
        "Sofia" => &["索菲亚", "苏菲亚", "ソフィア"],
        "Ivo" => &["伊沃", "イーヴォ", "イボ"],
        "Ada" => &["艾达", "エイダ"],
        "the harbour" => &["harbour", "quay", "港口", "码头", "港", "波止場"],
        "Harbour Bakery" => &["bakery", "面包店", "パン屋"],
        "Island School" => &["学校"],
        "Anchor Pub" => &["酒馆", "酒吧", "酒場", "パブ"],
        _ => &[],
    };
    names
        .iter()
        .copied()
        .chain(in_chinese(crate::ZH_HANS, name))
        .chain(in_chinese(crate::JA, name))
        .map(str::to_string)
        .collect()
}

/// What the harbour's sky is doing, in anybody's words.
fn weather(world: &World) -> String {
    use world_projection::Weather;
    match crate::story::weather(world) {
        Weather::Clear => "Clear skies. Lovely out on the water.",
        Weather::Cloudy => "Grey, but dry. It'll hold, I think.",
        Weather::Rain => "Wet. It's been coming down all day.",
        Weather::Storm => "A proper storm. The boats are staying in.",
        Weather::Snow => "Snow on the quay! Everything's gone quiet.",
        Weather::Fog => "Thick fog. You can't see the end of the quay.",
        Weather::Dust => "Dusty and dry.",
    }
    .into()
}

/// Who is at a place now, by name.
fn there_now(world: &World, place: EntityId, who: EntityId) -> Vec<String> {
    crate::story::people(world)
        .into_iter()
        .filter(|person| *person != who && lives::at(world.state(), *person) == Some(place))
        .map(|person| lives::name(world.state(), person))
        .collect()
}

fn place_line(world: &World, who: EntityId, place: EntityId) -> String {
    let bakery_open = text(world, BAKERY, OPERATING_STATUS).as_deref() != Some("closed");
    let mut line = match place {
        BAKERY if !bakery_open => "The bakery's shut. Everyone feels it.".to_string(),
        BAKERY if who == crate::MARA => "My bakery? Busy, warm, never enough hours.".into(),
        BAKERY => "Mara's bread is the best thing about this place.".into(),
        PUB if who == crate::LEO => {
            "The pub keeps me on my feet. And in everyone's secrets.".into()
        }
        PUB => "The pub's where everyone ends up of an evening.".into(),
        SCHOOL if who == crate::EMMA => "The children keep me young. And tired.".into(),
        SCHOOL => "Emma works wonders with those children.".into(),
        HARBOR => "The harbour's the heart of it all. Boats in, boats out.".into(),
        _ => format!("{}? It's alright.", lives::name(world.state(), place)),
    };
    let there = there_now(world, place, who);
    match there.as_slice() {
        [] => {}
        [one] => line.push_str(&format!(" {one}'s there now.")),
        [first, .., last] => line.push_str(&format!(" {first} and {last} are there now.")),
    }
    line
}

fn need_line(world: &World, who: EntityId) -> (String, Option<String>) {
    let commands = crate::projection::available_commands(world);
    crate::talk::request(world, who, &commands)
        .unwrap_or_else(|| ("Nothing, really. Thanks for asking.".into(), None))
}

fn work_line(world: &World, who: EntityId) -> Option<String> {
    Some(
        match text(world, who, JOB)?.as_str() {
            "baker" => "Up before dawn for the bread, every day.",
            "pub_owner" => "Pulling pints and hearing everyone's troubles.",
            "teacher" => "Lessons, marking, more lessons.",
            "fisher" => "Out on the water whenever the weather lets me.",
            "unemployed" => "I haven't any. It's eating at me.",
            _ => return None,
        }
        .into(),
    )
}

/// What a language model should be asked to hear the player's words to
/// someone with.
pub(crate) fn prompt(world: &World, who: EntityId, words: &str) -> Option<String> {
    conversation::prompt_for(world, &kit(world.state()), who, words)
}

/// The same, as data, for an app that builds the prompt itself.
pub(crate) fn voice_hearing(
    world: &World,
    who: EntityId,
    words: &str,
) -> Option<world_projection::VoiceHearing> {
    conversation::hearing_for(world, &kit(world.state()), who, words)
        .map(|hearing| hearing.to_voice())
}

/// How someone the player can talk to stands with them.
pub(crate) fn standing_of(world: &World, who: EntityId) -> Option<world_projection::Standing> {
    conversation::faces::standing_of(world, &kit(world.state()), who)
}

/// Hears what the player says to someone and answers, ready to record: with
/// the listener's ears if it has something usable to say, the System's own
/// otherwise.
pub(crate) fn say(
    world: &World,
    who: EntityId,
    words: &str,
    listener: &mut dyn conversation::Listener,
) -> Result<ActionRequest, String> {
    conversation::say_with(world, &kit(world.state()), who, words, listener)
}

/// Records a favour done, if what the player just said (`spoken`) did it.
pub(crate) fn favour_done(
    world: &mut World,
    actions: &world_core::ActionRegistry,
    spoken: world_core::EventId,
) -> Result<Option<world_core::EventId>, world_core::WorldError> {
    let kit = kit(world.state());
    conversation::favour::follow_up(world, actions, &kit, spoken)
}

/// At the end of a day the player was there for, someone may ask a favour.
pub(crate) fn favour_asked(
    world: &mut World,
    actions: &world_core::ActionRegistry,
    away: bool,
) -> Result<Vec<world_core::EventId>, world_core::WorldError> {
    let kit = kit(world.state());
    conversation::favour::tick(world, actions, &kit, away)
}

/// Everyone asking the player something now.
pub(crate) fn askers(world: &World) -> std::collections::BTreeSet<EntityId> {
    conversation::faces::askers(&crate::story::commands(world))
}

/// How someone feels, for their face.
pub(crate) fn mood_of(
    world: &World,
    who: EntityId,
    askers: &std::collections::BTreeSet<EntityId>,
) -> Option<world_projection::Mood> {
    conversation::faces::mood_of(world, &kit(world.state()), who, askers, || {
        // A child of the harbour, too young for its everyday life, is happy.
        (!lives::parents(world.state(), who).is_empty()).then_some(world_projection::Mood::Happy)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use conversation::corpus;

    /// Everyday things a player types are heard as what they mean, spoken
    /// to anyone in the harbour about anyone else, in English, Chinese and
    /// Japanese.
    #[test]
    fn people_understand_everyday_phrases() {
        let society = crate::TinySociety::new().unwrap();
        let world = society.world();
        let kit = kit(world.state());
        for (who, other, other_zh, other_ja) in [
            (crate::MARA, "Leo", "利奥", "レオ"),
            (crate::LEO, "Noah", "诺亚", "ノア"),
            (crate::EMMA, "Sofia", "索菲亚", "ソフィア"),
        ] {
            let phrases = corpus::filled(&corpus::Blanks {
                person: other.into(),
                person_zh: Some(other_zh.into()),
                place: "bakery".into(),
                place_zh: Some("面包店".into()),
                person_ja: Some(other_ja.into()),
                place_ja: Some("パン屋".into()),
                occasion: Some("Lantern Night".into()),
            });
            let score = corpus::score(&phrases, |words| {
                conversation::hear(world.state(), &kit, who, words).intent
            });
            assert!(score.misheard.is_empty(), "{:#?}", score.misheard);
            assert!(
                score.unclear_percent() <= 10.0,
                "{:.1}%: {:#?}",
                score.unclear_percent(),
                score.unclear
            );
        }
    }
}
