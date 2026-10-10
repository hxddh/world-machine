//! Talking to the settlement's people in the player's own words: what the
//! conversation System needs to know about each seed's place, and its own
//! words for places, work and needs.

use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_E};
use conversation::faces::in_chinese;
use world_core::{ActionRequest, EntityId, World, WorldState};

/// Every name the three places' lines can say, in each language they
/// speak: their people and everyone who may come, their figures, places
/// and days, each in translation.
pub(crate) const NAMES: &str = include_str!("../lexicon/names.tsv");

/// Every name any of the three places knows, read once: [`NAMES`], every
/// name a Pocket Universe person can have, what each place speaks of
/// elsewhere and its days, and each by its other names.
fn lexicon() -> &'static [String] {
    static LEXICON: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    LEXICON.get_or_init(|| {
        let people = crate::people_names();
        let festivals = crate::almanac::MARS
            .iter()
            .chain(crate::almanac::TOWN)
            .chain(crate::almanac::ICE)
            .map(|festival| festival.name);
        conversation::lexicon_of(
            people
                .iter()
                .copied()
                .chain(
                    ["sol", "week", "day"]
                        .iter()
                        .flat_map(|unit| elsewhere(unit).iter().copied()),
                )
                .chain(festivals),
            aliases,
            NAMES,
        )
    })
}

/// What each place has that its time otherwise would not: the colony
/// knows its planets by name; Maple Street in the late eighties has its
/// modems, answering machines and the first e-mail at work.
fn era_has(unit: &str) -> &'static [&'static str] {
    match unit {
        // The planets and their places, as a settlement between worlds
        // knows them.
        "sol" => &[
            "earth",
            "mars",
            "venus",
            "jupiter",
            "saturn",
            "mercury",
            "the moon",
            "luna",
            "phobos",
            "deimos",
            "olympus mons",
            "valles marineris",
            "hellas",
            "hellas planitia",
            "tharsis",
            "elysium",
            "elysium planitia",
            "utopia planitia",
            "arcadia planitia",
            "gale crater",
            "jezero",
            "jezero crater",
            "syrtis major",
            "the asteroid belt",
            "ceres",
            "europa",
            "titan",
            "ganymede",
            "io",
            "callisto",
            "the sun",
            "alpha centauri",
            "milky way",
            "the milky way",
            "地球",
            "火星",
            "金星",
            "木星",
            "土星",
            "水星",
            "月球",
            "月亮",
            "火卫一",
            "火卫二",
            "奥林匹斯山",
            "水手号峡谷",
            "水手谷",
            "希腊平原",
            "塔尔西斯",
            "埃律西昂",
            "乌托邦平原",
            "盖尔陨石坑",
            "小行星带",
            "谷神星",
            "木卫二",
            "土卫六",
            "太阳",
            "银河",
            "半人马座",
            "フォボス",
            "ダイモス",
            "オリンポス山",
            "マリネリス峡谷",
            "ヘラス平原",
            "タルシス",
            "エリシウム",
            "ユートピア平原",
            "ゲール・クレーター",
            "小惑星帯",
            "ケレス",
            "エウロパ",
            "タイタン",
            "天の川",
        ],
        "week" => &[
            "fax",
            "fax machine",
            "answering machine",
            "pager",
            "beeper",
            "modem",
            "bulletin board",
            "floppy disk",
            "floppy",
            "car phone",
            "email",
            "e-mail",
            "传真",
            "传呼机",
            "寻呼机",
            "调制解调器",
            "软盘",
            "电子邮件",
            "ファックス",
            "留守番電話",
            "ポケベル",
            "モデム",
            "フロッピー",
            "パソコン通信",
            "メール",
            // Its own money.
            "dollar*",
            "buck*",
            "quarter*",
            "美元",
            "块钱",
            "ドル",
            "セント",
        ],
        _ => &[],
    }
}

/// What each place lacks beyond what its time does: the penguins of
/// Icebridge have no engines, wires or money, only ice, fish and each
/// other.
fn era_lacks(unit: &str) -> &'static [&'static str] {
    match unit {
        "sol" | "week" => &[],
        _ => &[
            "car",
            "cars",
            "truck*",
            "lorry",
            "lorries",
            "petrol",
            "gasoline",
            "diesel",
            "train",
            "trains",
            "railway*",
            "aeroplane*",
            "airplane*",
            "motorbike*",
            "motorcycle*",
            "factory",
            "factories",
            "telephone*",
            "telegram*",
            "telegraph*",
            "electricity",
            "bank",
            "banks",
            "banknote*",
            "rifle*",
            "tractor*",
            "bicycle*",
            "汽车",
            "卡车",
            "汽油",
            "柴油",
            "火车",
            "铁路",
            "飞机",
            "摩托车",
            "工厂",
            "电话",
            "电报",
            "电力",
            "银行",
            "步枪",
            "拖拉机",
            "自行车",
            "自動車",
            "トラック",
            "ガソリン",
            "軽油",
            "電車",
            "汽車",
            "鉄道",
            "飛行機",
            "オートバイ",
            "工場",
            "電話",
            "電報",
            "電気",
            "銀行",
            "トラクター",
            "自転車",
        ],
    }
}

pub(crate) fn kit(state: &WorldState) -> conversation::Kit {
    kit_with(&crate::life::cast(state))
}

/// The kit, with the cast the World keeps as it stands.
pub(crate) fn kit_of(world: &World) -> conversation::Kit {
    kit_with(&crate::life::cast_kept(world).0)
}

fn kit_with(cast: &lives::Cast) -> conversation::Kit {
    conversation::Kit {
        era: era(cast.unit),
        elsewhere: elsewhere(cast.unit),
        lexicon,
        era_has: era_has(cast.unit),
        era_lacks: era_lacks(cast.unit),
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
/// How far along each seed's things are, told by what it counts time in.
fn era(unit: &str) -> conversation::Era {
    match unit {
        "sol" => conversation::Era::Spacefaring,
        "week" => conversation::Era::Television,
        _ => conversation::Era::Radio,
    }
}

/// What each seed's people speak of that is not on the scene: the places
/// over the horizon and the things of their day, as the seed's own lines
/// name them.
pub(crate) fn elsewhere(unit: &str) -> &'static [&'static str] {
    match unit {
        "sol" => &[
            "Earth",
            "地球",
            "Mars",
            "火星",
            "Tharsis",
            "塔尔西斯",
            "Phobos",
            "火卫一",
            "Hellas",
            "希腊平原",
            "Elysium",
            "埃律西昂",
            "Earthrise",
            "Ares Habitat",
            "Hydroponics Bay",
            "Kestrel Rover",
            "红隼号",
        ],
        "week" => &[
            "Maple Street",
            "枫树街",
            "Maple Arcade",
            "Elm Street",
            "榆树街",
            "K-88 Radio",
            "Night Bus 6",
            "the Rialto",
            "里亚托",
            "the Hendersons",
            "亨德森",
            "Greyhound",
            "灰狗巴士",
            "Pac-Man",
            "吃豆人",
            "Centipede",
            "蜈蚣",
            "Galaga",
            "大蜜蜂",
            "Tetris",
            "俄罗斯方块",
            "Donkey Kong",
            "Walkman",
            "随身听",
            "Nintendo",
            "Rubik",
            "MTV",
            "Springsteen",
            "斯普林斯汀",
            "Bee Gees",
            "Chevy",
            "Ford",
        ],
        _ => &[
            "Icebridge",
            "冰桥",
            "Fish Vault",
            "鱼库",
            "Aurora Council",
            "极光议会",
            "the Huddle",
        ],
    }
}

pub(crate) fn aliases(name: &str) -> Vec<String> {
    let names: &[&str] = match name {
        "Nia Chen" => &["妮娅", "ニア"],
        "Tomas Vale" => &["托马斯", "トマス"],
        "Ines Duarte" => &["伊内丝", "伊内斯", "イネス"],
        "Lena Ortiz" => &["莉娜", "蕾娜", "レナ"],
        "Max Park" => &["马克斯", "マックス"],
        "Ray Kowalski" => &["雷", "レイ"],
        "Piko" => &["皮可", "ピコ"],
        "Miri" => &["米丽", "ミリ"],
        "Tuk" => &["图克", "トゥク"],
        "Ares Habitat" => &["the dome", "基地", "栖息地", "居住区", "ドーム"],
        "Hydroponics Bay" => &["hydroponics", "greenhouse", "温室", "水培", "水耕"],
        "Maple Arcade" => &["游戏厅", "街机厅", "ゲームセンター", "ゲーセン"],
        "K-88 Radio" => &[
            "radio station",
            "the station",
            "电台",
            "广播站",
            "ラジオ局",
            "放送局",
        ],
        "Icebridge" => &["the bridge", "冰桥", "氷の橋"],
        "Fish Vault" => &["鱼库", "鱼仓", "魚の貯蔵庫", "魚倉"],
        _ => &[],
    };
    names
        .iter()
        .copied()
        .chain(in_chinese(include_str!("../locales/zh-Hans.tsv"), name))
        .chain(in_chinese(include_str!("../locales/ja.tsv"), name))
        .map(str::to_string)
        .chain(conversation::forms_in(NAMES, name))
        .collect()
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

fn place_mood(world: &World) -> String {
    let anchor = lives::name(world.state(), SLOT_A);
    format!("{anchor}'s holding together. We're managing.")
}

fn place_line(world: &World, who: EntityId, place: EntityId) -> String {
    let state = world.state();
    let name = lives::name(state, place);
    let mut line = if crate::life::work(state, who) == Some(place) {
        format!("{name}? It's where I spend my days. I know every corner of it.")
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
    // Worked out once for where the World stands, not once for everyone
    // asked: the openers ask it of every resident in every snapshot.
    struct OnOffer(Vec<world_projection::ProjectionCommand>);
    let commands = world.as_it_stands(|| OnOffer(crate::projection::commands_on_offer(world)));
    crate::talk::request(world, who, &commands.0)
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

/// The same, as data, for an app that builds the prompt itself.
pub(crate) fn voice_hearing(
    world: &World,
    who: EntityId,
    words: &str,
) -> Option<world_projection::VoiceHearing> {
    conversation::hearing_for(world, &kit(world.state()), who, words)
        .map(|hearing| conversation::to_voice(&hearing))
}

/// How someone the player can talk to stands with them.
pub(crate) fn standing_of(world: &World, who: EntityId) -> Option<world_projection::Standing> {
    conversation::faces::standing_of(world, &kit_of(world), who)
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

/// The open favour's quick reply, chosen with a click: done as offered,
/// never heard (`conversation::say_offered`).
pub(crate) fn say_offered(
    world: &World,
    who: EntityId,
    words: &str,
) -> Result<ActionRequest, String> {
    conversation::say_offered(world, &kit(world.state()), who, words)
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

/// At the end of a period the player was there for, someone may ask a
/// favour.
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
    conversation::faces::mood_of(world, &kit_of(world), who, askers, || None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use conversation::corpus;

    /// In every seed, everyday things a player types are heard as what
    /// they mean, in English, Chinese and Japanese.
    #[test]
    fn people_understand_everyday_phrases_in_every_seed() {
        for (seed, other_zh, other_ja, place) in [
            (
                crate::SEED_MARS_COLONY_COMMAND,
                "托马斯",
                "トマス",
                "hydroponics",
            ),
            (
                crate::SEED_1980S_TOWN_COMMAND,
                "马克斯",
                "マックス",
                "radio",
            ),
            (
                crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
                "米丽",
                "ミリ",
                "vault",
            ),
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
            let place_ja = aliases(&lives::name(state, SLOT_C))
                .into_iter()
                .find(|alias| {
                    alias
                        .chars()
                        .any(|c| ('\u{3040}'..='\u{30ff}').contains(&c))
                });
            let occasion = crate::almanac::almanac(state)
                .festivals
                .first()
                .map(|festival| festival.name.to_string());
            let phrases = corpus::filled(&corpus::Blanks {
                person: other,
                person_zh: Some(other_zh.into()),
                place: place.into(),
                place_zh,
                person_ja: Some(other_ja.into()),
                place_ja,
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
