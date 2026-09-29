//! The player's mark on each place: plots, the newcomers what they build
//! draws, designs and names.

use crate::places::Place;
use crate::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use std::collections::BTreeSet;
use world_core::Value;
use world_projection::{
    command_with, CanvasItem, Design, MarkShape, ProjectionSnapshot, SelectionId, StoryPage,
    StoryRequest, PATTERN_CELLS,
};

const SEEDS: [(Place, &str); 3] = [
    (Place::Ares, SEED_MARS_COLONY_COMMAND),
    (Place::Maple, SEED_1980S_TOWN_COMMAND),
    (Place::Ice, SEED_PENGUIN_CIVILIZATION_COMMAND),
];

fn seeded(seed: &str) -> PocketUniverse {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    universe
}

fn item(snapshot: &ProjectionSnapshot, id: SelectionId) -> &CanvasItem {
    snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == id)
        .expect("on the scene")
}

#[test]
fn thirty_works_in_every_place_each_drawn_its_own_way() {
    for (place, seed) in SEEDS {
        let universe = seeded(seed);
        let state = universe.world().state();
        let works = crate::plots::works(state);
        let ids = works.iter().map(|work| work.id).collect::<BTreeSet<_>>();
        let names = works.iter().map(|work| work.name).collect::<BTreeSet<_>>();
        assert!(works.len() >= 30, "{place:?}: {}", works.len());
        assert_eq!(ids.len(), works.len(), "{place:?}");
        assert_eq!(names.len(), works.len(), "{place:?}");
        for work in works {
            assert_ne!(
                crate::story::fixture_shape(universe.world(), work.shape),
                MarkShape::Parcel,
                "{place:?}: {} is drawn as a parcel",
                work.id
            );
        }
        let snapshot = universe.projection_snapshot();
        let canvas = &snapshot.canvas;
        assert!(
            canvas.plots.len() >= 12,
            "{place:?}: {}",
            canvas.plots.len()
        );
        let districts = canvas
            .plots
            .iter()
            .map(|plot| plot.district.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(districts.len(), 3, "{place:?}: plots on every stretch");
        let offered = canvas
            .plots
            .iter()
            .flat_map(|plot| plot.offers.iter().map(|offer| offer.label.as_str()))
            .collect::<BTreeSet<_>>();
        assert!(offered.len() >= 30, "{place:?}: {offered:?}");
        for plot in &canvas.plots {
            for other in canvas
                .items
                .iter()
                .filter(|item| item.kind != world_projection::CanvasItemKind::Actor)
            {
                if let Some(px) = other.px {
                    assert!(
                        (px - plot.px).abs() >= 0.01,
                        "{place:?}: {} at {px}",
                        other.label
                    );
                }
            }
        }
    }
}

#[test]
fn a_work_on_a_plot_is_finished_and_draws_someone_in_every_place() {
    for (place, seed, work, trade) in [
        (
            Place::Ares,
            SEED_MARS_COLONY_COMMAND,
            "hydroponics_bay",
            "botanist",
        ),
        (Place::Maple, SEED_1980S_TOWN_COMMAND, "diner", "diner cook"),
        (
            Place::Ice,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
            "song_circle",
            "singer",
        ),
    ] {
        let mut universe = seeded(seed);
        let snapshot = universe.projection_snapshot();
        let offer = snapshot
            .canvas
            .plots
            .iter()
            .flat_map(|plot| plot.offers.iter())
            .find(|offer| offer.command.contains(&format!(".plot.{work}.")))
            .unwrap_or_else(|| panic!("{place:?}: a plot offers the {work}"))
            .command
            .clone();
        universe.invoke_projection_command(&offer).unwrap();
        let mut drawn = None;
        for _ in 0..40 {
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
            drawn = universe
                .world()
                .events()
                .iter()
                .find(|event| event.kind == "drawn_here")
                .cloned();
            if drawn.is_some() {
                break;
            }
        }
        // A full place draws nobody until someone leaves; this one has room.
        let drawn = drawn.unwrap_or_else(|| panic!("{place:?}: the {work} drew nobody"));
        let world = universe.world();
        let told = crate::story::told(world, &drawn).unwrap();
        assert!(told.contains(trade), "{place:?}: {told}");
        let newcomer = drawn.targets[0];
        let Some(StoryPage::Legend(legend)) =
            universe.story(StoryRequest::Legend(SelectionId::Entity(newcomer)))
        else {
            panic!("a legend");
        };
        assert!(
            legend
                .lines
                .iter()
                .any(|line| line.text.contains("you built")),
            "{place:?}: {legend:?}"
        );
        let replayed = world.replay().unwrap();
        assert_eq!(replayed.state(), world.state(), "{place:?}");
    }
}

/// A design in two colours.
fn a_design() -> Design {
    let cells = (0..PATTERN_CELLS)
        .map(|at| {
            if (at / 16 + at % 16) % 5 < 2 {
                '1'
            } else {
                '0'
            }
        })
        .collect::<String>();
    Design::new(&cells, &[9, 3]).unwrap()
}

#[test]
fn a_design_and_a_name_are_kept_through_a_replay_and_a_world_code() {
    for (place, seed, flag) in [
        (Place::Ares, SEED_MARS_COLONY_COMMAND, "colony_flag"),
        (Place::Maple, SEED_1980S_TOWN_COMMAND, "town_flag"),
        (
            Place::Ice,
            SEED_PENGUIN_CIVILIZATION_COMMAND,
            "rookery_flag",
        ),
    ] {
        let mut universe = seeded(seed);
        let offer = universe
            .projection_snapshot()
            .canvas
            .plots
            .iter()
            .flat_map(|plot| plot.offers.iter())
            .find(|offer| offer.command.contains(&format!(".plot.{flag}.")))
            .unwrap_or_else(|| panic!("{place:?}: a plot offers the {flag}"))
            .command
            .clone();
        universe.invoke_projection_command(&offer).unwrap();
        let snapshot = universe.projection_snapshot();
        let pole = snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.design.is_some() && item.variant.is_some())
            .expect("the flag can wear a design")
            .clone();
        let painted = a_design();
        universe
            .invoke_projection_command(&command_with(
                &pole.design.as_ref().unwrap().command,
                &painted.text(),
            ))
            .unwrap();
        universe
            .invoke_projection_command(&command_with(
                &pole.naming.as_ref().expect("a work can be named").command,
                "Our Colours",
            ))
            .unwrap();
        for _ in 0..3 {
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
        let snapshot = universe.projection_snapshot();
        let pole_now = item(&snapshot, pole.id);
        assert_eq!(pole_now.pattern, Some(painted.pattern()), "{place:?}");
        assert_eq!(pole_now.label, "Our Colours", "{place:?}");
        let replayed = universe.world().replay().unwrap();
        assert_eq!(replayed.state(), universe.world().state(), "{place:?}");
        // Through a World code, opened as a visit.
        let code =
            world_library::world_code_for_archive(universe.archive().unwrap(), None).unwrap();
        let mut registry = world_host::WorldRegistry::new();
        registry
            .register(crate::pocket_universe_registration())
            .unwrap();
        let visit = world_library::WorldVisit::open_code(&code, &registry).unwrap();
        let seen = visit.snapshot();
        let pole_seen = item(&seen, pole.id);
        assert_eq!(pole_seen.pattern, Some(painted.pattern()), "{place:?}");
        assert_eq!(pole_seen.label, "Our Colours", "{place:?}");
        // Refused: a design on what cannot wear one, a bad name.
        let SelectionId::Entity(pole_id) = pole.id else {
            unreachable!()
        };
        for bad in [
            format!(
                "pocket-universe.mark.design.{}={}",
                crate::SLOT_B.0,
                painted.text()
            ),
            format!("pocket-universe.mark.design.{}=01:1", pole_id.0),
            format!("pocket-universe.mark.name.{}=\u{7}", pole_id.0),
            format!("pocket-universe.mark.name.{}=", pole_id.0),
        ] {
            assert!(universe.invoke_projection_command(&bad).is_err(), "{bad}");
        }
    }
}

/// The warm builder in a place: the first question each period, and every
/// third period the first thing a plot offers, or else the first thing to
/// make by hand.
fn warm_builder(seed: &str, periods: usize) -> PocketUniverse {
    let mut universe = seeded(seed);
    for period in 1..=periods {
        let snapshot = universe.projection_snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none())
        {
            let _ = universe.invoke_projection_command(&answer.id.clone());
        }
        if period % 3 == 0 {
            let snapshot = universe.projection_snapshot();
            let offer = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone())
                .or_else(|| {
                    snapshot
                        .commands
                        .iter()
                        .find(|c| {
                            c.unavailable.is_none()
                                && c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo")
                        })
                        .map(|c| c.id.clone())
                });
            if let Some(offer) = offer {
                let _ = universe.invoke_projection_command(&offer);
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    universe
}

/// What three years of the warm builder came to in a place: works
/// finished on plots, each newcomer drawn as told, and the World code's
/// length.
fn three_years_of_building(seed: &str) -> (usize, Vec<String>, usize) {
    let universe = warm_builder(seed, 1_080);
    let world = universe.world();
    let drawn = world
        .events_of_kind(&["drawn_here"])
        .into_iter()
        .inspect(|event| {
            assert!(matches!(
                event.payload.get("drawn_by"),
                Some(Value::Entity(_))
            ))
        })
        .map(|event| crate::story::told(world, event).unwrap_or_default())
        .collect();
    let code = world_library::world_code_for_archive(universe.archive().unwrap(), None).unwrap();
    (
        world.events_of_kind(&["plot_finished"]).len(),
        drawn,
        code.len(),
    )
}

#[test]
#[ignore]
fn a_warm_builder_draws_newcomers_in_every_place_in_three_years() {
    let places = std::thread::scope(|scope| {
        SEEDS
            .map(|(place, seed)| (place, scope.spawn(move || three_years_of_building(seed))))
            .map(|(place, running)| (place, running.join().unwrap()))
    });
    for (place, (finished, drawn, code)) in places {
        eprintln!(
            "{place:?}: {finished} works finished on plots, {} newcomers drawn, a {code}-character World code",
            drawn.len()
        );
        for told in &drawn {
            eprintln!("  {told}");
        }
        assert!(drawn.len() >= 2, "{place:?}: {}", drawn.len());
    }
}

#[test]
#[ignore]
fn a_newborn_can_be_named_in_every_place() {
    for (place, seed) in SEEDS {
        let mut universe = seeded(seed);
        let mut child = None;
        for _ in 0..1_080 {
            let snapshot = universe.projection_snapshot();
            if let Some(answer) = snapshot.commands.iter().find(|c| {
                c.question.is_some() && c.unavailable.is_none() && !c.id.contains(".mark.")
            }) {
                let _ = universe.invoke_projection_command(&answer.id.clone());
            }
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
            if let Some(Value::Entity(born)) = universe
                .world()
                .events_of_kind(&["born"])
                .last()
                .and_then(|born| born.payload.get("who"))
            {
                child = Some(*born);
                break;
            }
        }
        let Some(child) = child else {
            eprintln!("{place:?}: nobody was born in three years");
            continue;
        };
        let snapshot = universe.projection_snapshot();
        let card = snapshot
            .commands
            .iter()
            .filter(|c| {
                c.question
                    .as_ref()
                    .is_some_and(|q| q.id == format!("name-{}", child.0))
            })
            .collect::<Vec<_>>();
        assert_eq!(card.len(), 3, "{place:?}");
        let pick = card[1].title.clone();
        universe
            .invoke_projection_command(&card[1].id.clone())
            .unwrap();
        let world = universe.world();
        assert_eq!(lives::name(world.state(), child), pick, "{place:?}");
        let born = world
            .events_of_kind(&["born"])
            .last()
            .copied()
            .unwrap()
            .clone();
        let told = crate::story::told(world, &born).unwrap();
        assert!(told.contains(&pick), "{place:?}: {told}");
    }
}

/// Whether a line, translated, still has a word of English in it that is
/// not a name.
fn english(names: &BTreeSet<String>, text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word.len() > 1 && !names.contains(word))
}

/// Everything the player's mark shows in each place a warm builder has
/// lived in for a year, designs and names among it, is shown in Chinese.
#[test]
#[ignore]
fn the_mark_is_shown_in_chinese_in_every_place() {
    let mut catalog = world_i18n::Catalog::parse(include_str!(
        "../../../crates/world-builtins/locales/systems.zh-Hans.tsv"
    ));
    catalog.extend(include_str!("../locales/zh-Hans.tsv"));
    let mut left = Vec::new();
    for (place, seed) in SEEDS {
        let mut universe = warm_builder(seed, 360);
        let mut shown = BTreeSet::new();
        let snapshot = universe.projection_snapshot();
        for item in &snapshot.canvas.items {
            if let Some(design) = &item.design {
                universe
                    .invoke_projection_command(&command_with(&design.command, &a_design().text()))
                    .unwrap();
            }
            if let Some(naming) = &item.naming {
                universe
                    .invoke_projection_command(&command_with(&naming.command, "Stormy Petrel"))
                    .unwrap();
            }
        }
        let snapshot = universe.projection_snapshot();
        for plot in &snapshot.canvas.plots {
            for offer in &plot.offers {
                shown.insert(offer.label.clone());
                shown.extend(offer.unavailable.clone());
            }
        }
        for item in snapshot
            .canvas
            .items
            .iter()
            .filter(|item| item.variant.is_some())
        {
            shown.insert(item.label.clone());
            shown.insert(item.detail.clone());
        }
        let world = universe.world();
        for event in world.events_of_kind(&[
            "built_by_hand",
            "plot_finished",
            "drawn_here",
            "named",
            "designed",
        ]) {
            shown.extend(crate::story::told(world, event));
            shown.extend(lives::said(event).map(|(_, said)| said));
            shown.extend(hands::said(event).map(|(_, said)| said));
        }
        let names = world
            .state()
            .entities()
            .filter(|entity| entity.kind != "fixture")
            .flat_map(|entity| [entity.component("name"), entity.component(hands::WAS)])
            .filter_map(|name| match name {
                Some(Value::Text(name)) => Some(name.clone()),
                _ => None,
            })
            .chain(["Stormy Petrel".to_string()])
            .flat_map(|name| {
                name.split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeSet<_>>();
        eprintln!("{place:?}: {} lines of the mark shown", shown.len());
        left.extend(
            shown
                .iter()
                .filter(|line| !line.trim().is_empty())
                .filter_map(|line| {
                    let translated = catalog.translate(line).unwrap_or_else(|| line.clone());
                    english(&names, &translated)
                        .then(|| format!("{place:?}: {line} => {translated}"))
                }),
        );
    }
    assert!(left.is_empty(), "left in English:\n{}", left.join("\n"));
}

/// What a card says under a thing's name is the player's words: never an
/// engine word, what the World keeps it as, or a raw value.
#[test]
fn every_card_speaks_the_players_words_in_every_place() {
    const KEPT_AS: [&str; 8] = [
        "asset", "fixture", "entity", "resident", "order", "place", "person", "penguin",
    ];
    for (place, seed) in SEEDS {
        let universe = warm_builder(seed, 30);
        let snapshot = universe.projection_snapshot();
        for item in &snapshot.canvas.items {
            let detail = item.detail.to_lowercase();
            let words = detail
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .collect::<Vec<_>>();
            assert!(
                world_projection::engine_words_in(&item.detail).is_empty()
                    && !words
                        .iter()
                        .any(|word| KEPT_AS.contains(word) || word.contains('_')),
                "{place:?}: {}: {:?}",
                item.label,
                item.detail
            );
        }
    }
}
