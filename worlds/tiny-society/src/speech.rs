//! Talking to the harbour's people in the player's own words: what the
//! conversation System needs to know about the harbour, and the harbour's
//! own words for its places, its work and its needs.

use crate::model::OPERATING_STATUS;
use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use society_basic::JOB;
use world_core::{ActionRequest, EntityId, Value, World, WorldState};

fn text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        _ => None,
    }
}

pub(crate) fn kit(_: &WorldState) -> conversation::Kit {
    conversation::Kit {
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
        "Jonas" => &["乔纳斯"],
        "Mara" => &["玛拉"],
        "Leo" => &["利奥", "里奥"],
        "Emma" => &["艾玛"],
        "Mia" => &["米娅", "米亚"],
        "Noah" => &["诺亚"],
        "Evan" => &["埃文"],
        "Sofia" => &["索菲亚", "苏菲亚"],
        "Ivo" => &["伊沃"],
        "Ada" => &["艾达"],
        "Harbor" => &["harbour", "quay", "港口", "码头"],
        "Harbor Bakery" => &["bakery", "面包店"],
        "Island School" => &["学校"],
        "Anchor Pub" => &["酒馆", "酒吧"],
        _ => &[],
    };
    names.iter().map(|name| name.to_string()).collect()
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

/// How someone the player can talk to stands with them.
pub(crate) fn standing_of(world: &World, who: EntityId) -> Option<world_projection::Standing> {
    let state = world.state();
    let kit = kit(state);
    conversation::can_talk_to(state, &kit, who).then(|| {
        let standing = conversation::standing(state, &kit, who);
        world_projection::Standing {
            level: standing.level,
            words: standing.words,
        }
    })
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

/// Everyone asking the player something now.
pub(crate) fn askers(world: &World) -> std::collections::BTreeSet<EntityId> {
    crate::story::commands(world)
        .iter()
        .filter(|command| command.question.is_some())
        .filter_map(|command| match command.asker {
            Some(world_projection::SelectionId::Entity(who)) => Some(who),
            _ => None,
        })
        .collect()
}

/// How someone feels, for their face: thinking while they are asking the
/// player something, cross while hurt by what the player said, else how
/// their life is going.
pub(crate) fn mood_of(
    world: &World,
    who: EntityId,
    askers: &std::collections::BTreeSet<EntityId>,
) -> Option<world_projection::Mood> {
    use world_projection::Mood;
    let state = world.state();
    if !lives::enrolled(state, who) {
        // A child of the harbour, too young for its everyday life, is happy.
        return (!lives::parents(state, who).is_empty()).then_some(Mood::Happy);
    }
    if askers.contains(&who) {
        return Some(Mood::Thinking);
    }
    if standing_of(world, who).is_some_and(|standing| standing.level < 0) {
        return Some(Mood::Cross);
    }
    Some(match lives::mood(state, who) {
        "cross" => Mood::Cross,
        "sad" => Mood::Sad,
        "happy" => Mood::Happy,
        _ => Mood::Content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use conversation::corpus;

    /// Everyday things a player types are heard as what they mean, spoken
    /// to anyone in the harbour about anyone else, in English and Chinese.
    #[test]
    fn people_understand_everyday_phrases() {
        let society = crate::TinySociety::new().unwrap();
        let world = society.world();
        let kit = kit(world.state());
        for (who, other, other_zh) in [
            (crate::MARA, "Leo", "利奥"),
            (crate::LEO, "Noah", "诺亚"),
            (crate::EMMA, "Sofia", "索菲亚"),
        ] {
            let phrases = corpus::filled(&corpus::Blanks {
                person: other.into(),
                person_zh: Some(other_zh.into()),
                place: "bakery".into(),
                place_zh: Some("面包店".into()),
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
