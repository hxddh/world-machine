//! Talking to the settlement's people in the player's own words: what the
//! conversation System needs to know about each seed's place, and its own
//! words for places, work and needs.

use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_E};
use world_core::{ActionRequest, EntityId, World, WorldState};

pub(crate) fn kit(state: &WorldState) -> conversation::Kit {
    let cast = crate::life::cast(state);
    conversation::Kit {
        period: crate::BACKGROUND_PERIOD,
        unit: cast.unit,
        settlement: cast.settlement,
        people: crate::life::people_in,
        places: |state| {
            [SLOT_A, SLOT_C]
                .into_iter()
                .filter(|place| state.entity(*place).is_some())
                .collect()
        },
        place_line,
        need_line,
        work_line,
        place_mood,
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
        recalled: crate::story::recalled,
    }
}

/// The other names each seed's people and places go by.
fn aliases(name: &str) -> Vec<String> {
    let names: &[&str] = match name {
        "Nia Chen" => &["妮娅"],
        "Tomas Vale" => &["托马斯"],
        "Ines Duarte" => &["伊内丝", "伊内斯"],
        "Lena Ortiz" => &["莉娜", "蕾娜"],
        "Max Park" => &["马克斯"],
        "Ray Kowalski" => &["雷"],
        "Piko" => &["皮可"],
        "Miri" => &["米丽"],
        "Tuk" => &["图克"],
        "Ares Habitat" => &["the dome", "基地", "栖息地"],
        "Hydroponics Bay" => &["hydroponics", "greenhouse", "温室", "水培"],
        "Maple Arcade" => &["游戏厅", "街机厅"],
        "K-88 Radio" => &["radio station", "the station", "电台", "广播站"],
        "Icebridge" => &["the bridge", "冰桥"],
        "Fish Vault" => &["鱼库", "鱼仓"],
        _ => &[],
    };
    names.iter().map(|name| name.to_string()).collect()
}

/// What the sky is doing, in anybody's words.
fn weather(world: &World) -> String {
    use world_projection::Weather;
    match crate::story::weather(world) {
        Weather::Clear => "Clear. Not a cloud anywhere.",
        Weather::Cloudy => "Overcast. Flat light all day.",
        Weather::Rain => "Raining. Everyone's indoors.",
        Weather::Storm => "A storm. Nobody's going out in that.",
        Weather::Snow => "Snowing. Everything's gone soft and quiet.",
        Weather::Fog => "Fog. You can hardly see your own feet.",
        Weather::Dust => "Dust in the air. The filters are working hard.",
    }
    .into()
}

fn troubled(world: &World) -> bool {
    matches!(
        crate::pressure::pressure_id_from_state(world.state()).as_str(),
        "warning" | "crisis" | "lost"
    )
}

fn place_mood(world: &World) -> String {
    let anchor = lives::name(world.state(), SLOT_A);
    if troubled(world) {
        format!("Worried. {anchor} needs all of us.")
    } else {
        format!("{anchor}'s holding together. We're managing.")
    }
}

fn place_line(world: &World, who: EntityId, place: EntityId) -> String {
    let state = world.state();
    let name = lives::name(state, place);
    let mut line = if crate::life::work(state, who) == Some(place) {
        format!("{name}? It's where I spend my days. I know every corner of it.")
    } else if place == SLOT_A && troubled(world) {
        format!("{name} is in trouble. We all feel it.")
    } else if place == SLOT_A {
        format!("{name}'s home. It holds us all together.")
    } else {
        format!("{name}'s where you go to think.")
    };
    let there = crate::life::people(world)
        .into_iter()
        .filter(|person| *person != who && lives::at(state, *person) == Some(place))
        .map(|person| lives::name(state, person))
        .collect::<Vec<_>>();
    match there.as_slice() {
        [] => {}
        [one] => line.push_str(&format!(" {one}'s there now.")),
        [first, .., last] => line.push_str(&format!(" {first} and {last} are there now.")),
    }
    line
}

fn need_line(world: &World, who: EntityId) -> (String, Option<String>) {
    let commands = crate::projection::commands_on_offer(world);
    crate::talk::request(world, who, &commands)
        .unwrap_or_else(|| ("Nothing right now. Just time.".into(), None))
}

fn work_line(world: &World, who: EntityId) -> Option<String> {
    let state = world.state();
    let place = lives::name(state, crate::life::work(state, who)?);
    Some(match who {
        SLOT_B => format!("Keeping {place} running. Somebody has to."),
        SLOT_E => format!("Out past {place} as often as I can."),
        _ => format!("I help out at {place}."),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use conversation::corpus;

    /// In every seed, everyday things a player types are heard as what
    /// they mean, in English and Chinese.
    #[test]
    fn people_understand_everyday_phrases_in_every_seed() {
        for (seed, other_zh, place) in [
            (crate::SEED_MARS_COLONY_COMMAND, "托马斯", "hydroponics"),
            (crate::SEED_1980S_TOWN_COMMAND, "马克斯", "radio"),
            (crate::SEED_PENGUIN_CIVILIZATION_COMMAND, "米丽", "vault"),
        ] {
            let mut universe = crate::PocketUniverse::new().unwrap();
            universe.invoke_projection_command(seed).unwrap();
            let world = universe.world();
            let state = world.state();
            let kit = kit(state);
            let other = lives::first_name(state, SLOT_E);
            let place_zh = aliases(&lives::name(state, SLOT_C))
                .into_iter()
                .find(|alias| !alias.is_ascii());
            let occasion = crate::almanac::almanac(state)
                .festivals
                .first()
                .map(|festival| festival.name.to_string());
            let phrases = corpus::filled(&corpus::Blanks {
                person: other,
                person_zh: Some(other_zh.into()),
                place: place.into(),
                place_zh,
                occasion,
            });
            let score = corpus::score(&phrases, |words| {
                conversation::hear(state, &kit, SLOT_B, words).intent
            });
            assert!(score.misheard.is_empty(), "{seed}: {:#?}", score.misheard);
            assert!(
                score.unclear_percent() <= 10.0,
                "{seed}: {:.1}%: {:#?}",
                score.unclear_percent(),
                score.unclear
            );
        }
    }
}
