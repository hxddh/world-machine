//! The player's mark on the harbour: plots, the people what they build
//! draws, designs and names.

use crate::plots::{self, PLOT_ROW};
use crate::{story, TinySociety, TinySocietyBranch};
use std::collections::BTreeSet;
use world_core::{Value, World};
use world_projection::{
    command_with, CanvasItem, Design, MarkShape, ProjectionSnapshot, SelectionId, StoryPage,
    StoryRequest, PATTERN_CELLS,
};

fn opened() -> TinySocietyBranch {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    branch
}

fn pass(branch: &mut TinySocietyBranch) {
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
}

fn item(snapshot: &ProjectionSnapshot, id: SelectionId) -> &CanvasItem {
    snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == id)
        .expect("on the scene")
}

/// A design of stripes and a sun in three colours.
fn a_design() -> Design {
    let cells = (0..PATTERN_CELLS)
        .map(|at| {
            let (x, y) = ((at % 16) as i32, (at / 16) as i32);
            if (x - 8).pow(2) + (y - 8).pow(2) < 16 {
                '2'
            } else if y % 4 < 2 {
                '0'
            } else {
                '1'
            }
        })
        .collect::<String>();
    Design::new(&cells, &[8, 0, 4]).unwrap()
}

#[test]
fn thirty_odd_works_can_be_built_on_plots_each_drawn_its_own_way() {
    let works = plots::works();
    let ids = works.iter().map(|work| work.id).collect::<BTreeSet<_>>();
    let names = works.iter().map(|work| work.name).collect::<BTreeSet<_>>();
    assert!(works.len() >= 30, "{}", works.len());
    assert_eq!(ids.len(), works.len());
    assert_eq!(names.len(), works.len());
    for work in works {
        assert_ne!(
            story::fixture_shape(work.shape),
            MarkShape::Parcel,
            "{} is drawn as a parcel",
            work.id
        );
    }
    let branch = opened();
    let snapshot = branch.projection_snapshot();
    let canvas = &snapshot.canvas;
    assert!(canvas.plots.len() >= 15, "{}", canvas.plots.len());
    let districts = canvas
        .plots
        .iter()
        .map(|plot| plot.district.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(districts.len(), 3, "plots on every stretch");
    let offered = canvas
        .plots
        .iter()
        .flat_map(|plot| plot.offers.iter().map(|offer| offer.label.as_str()))
        .collect::<BTreeSet<_>>();
    assert!(offered.len() >= 30, "{offered:?}");
    for plot in &canvas.plots {
        assert_eq!(plot.row as usize, PLOT_ROW);
        assert!(!plot.offers.is_empty() && plot.offers.len() <= 12);
        assert!(canvas.district_at(plot.px).is_some());
        // No plot shares a spot with anything on the scene.
        for other in &canvas.items {
            if let Some(px) = other
                .px
                .filter(|_| other.kind != world_projection::CanvasItemKind::Actor)
            {
                assert!((px - plot.px).abs() >= 0.01, "{} at {px}", other.label);
            }
        }
    }
}

/// Builds a work on a plot by the command a screen sends.
fn build(branch: &mut TinySocietyBranch, work: &str) -> (String, f32) {
    let snapshot = branch.projection_snapshot();
    let (plot, offer) = snapshot
        .canvas
        .plots
        .iter()
        .find_map(|plot| {
            plot.offers
                .iter()
                .find(|offer| offer.command.contains(&format!(".plot.{work}.")))
                .map(|offer| (plot, offer))
        })
        .unwrap_or_else(|| panic!("a plot offers the {work}"));
    assert!(offer.unavailable.is_none(), "{:?}", offer.unavailable);
    branch.invoke_projection_command(&offer.command).unwrap();
    (plot.id.clone(), plot.px)
}

#[test]
fn a_bandstand_on_a_plot_is_finished_and_draws_a_musician() {
    let mut branch = opened();
    let (plot, px) = build(&mut branch, "bandstand");
    let snapshot = branch.projection_snapshot();
    let bandstand = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.label == "Bandstand")
        .expect("the bandstand stands on the scene");
    assert_eq!(bandstand.px, Some(px));
    assert_eq!(bandstand.detail, "Being built");
    assert!(bandstand.variant.is_some());
    assert!(
        !snapshot.canvas.plots.iter().any(|p| p.id == plot),
        "the plot is taken"
    );
    // A plot build is a deed: it counts toward the day's two.
    let SelectionId::Entity(id) = bandstand.id else {
        panic!("an entity");
    };
    let mut drawn = None;
    for _ in 0..20 {
        pass(&mut branch);
        drawn = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "drawn_here")
            .cloned();
        if drawn.is_some() {
            break;
        }
    }
    let world = branch.world();
    let finished = world
        .events()
        .iter()
        .find(|event| event.kind == "plot_finished")
        .expect("the harbour finished it");
    let drawn = drawn.expect("the bandstand drew someone");
    assert_eq!(drawn.payload.get("drawn_by"), Some(&Value::Entity(id)));
    assert_eq!(drawn.caused_by, vec![finished.id]);
    let told = story::told(world, &drawn).unwrap();
    assert!(
        told.contains("musician") && told.contains("bandstand"),
        "{told}"
    );
    let newcomer = drawn.targets[0];
    assert_eq!(lives::drawn_by(world.state(), newcomer), Some(id));
    // Their legend says so, and they work at the bandstand.
    let Some(StoryPage::Legend(legend)) =
        branch.story(StoryRequest::Legend(SelectionId::Entity(newcomer)))
    else {
        panic!("a legend");
    };
    assert!(
        legend
            .lines
            .iter()
            .any(|line| line.text.contains("bandstand")),
        "{legend:?}"
    );
    let snapshot = branch.projection_snapshot();
    let person = item(&snapshot, SelectionId::Entity(newcomer));
    assert!(person
        .day
        .iter()
        .any(|stop| stop.at == SelectionId::Entity(id)));
    let bandstand = item(&snapshot, SelectionId::Entity(id));
    assert!(bandstand.built.is_some() && bandstand.detail == "Built by you");
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
}

#[test]
fn a_plot_takes_what_it_offers_and_nothing_else() {
    let mut branch = opened();
    // A quay work is not built on the square, nor on a plot that is not.
    assert!(branch
        .invoke_projection_command("tiny-society.hand.plot.boathouse.p9")
        .is_err());
    assert!(branch
        .invoke_projection_command("tiny-society.hand.plot.bandstand.p2")
        .is_err());
    assert!(branch
        .invoke_projection_command("tiny-society.hand.plot.bandstand.p9")
        .is_ok());
    // Taken, and the bandstand is built once.
    assert!(branch
        .invoke_projection_command("tiny-society.hand.plot.clock_tower.p9")
        .is_err());
    assert!(branch
        .invoke_projection_command("tiny-society.hand.plot.bandstand.p10")
        .is_err());
    // Two deeds a day, plots or not.
    assert!(branch
        .invoke_projection_command("tiny-society.hand.plot.clock_tower.p10")
        .is_ok());
    let err = branch
        .invoke_projection_command("tiny-society.hand.plot.bookshop.p12")
        .unwrap_err();
    assert!(err.to_string().contains("enough for today"), "{err}");
}

/// Builds a flagpole and paints a design on it, and names Jonas's boat.
fn designed_and_named(branch: &mut TinySocietyBranch) -> (SelectionId, Design) {
    let flag = format!("tiny-society.hand.build.flagpole.{}", crate::HARBOR.0);
    branch.invoke_projection_command(&flag).unwrap();
    let snapshot = branch.projection_snapshot();
    let pole = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.label == "Flagpole")
        .unwrap();
    let design = pole.design.clone().expect("a flag can wear a design");
    assert_eq!(design.wears, world_projection::Wears::Flag);
    let painted = a_design();
    branch
        .invoke_projection_command(&command_with(&design.command, &painted.text()))
        .unwrap();
    let boat = item(&snapshot, SelectionId::Entity(crate::JONAS_BOAT));
    let naming = boat.naming.clone().expect("a boat can be named");
    branch
        .invoke_projection_command(&command_with(&naming.command, " Stormy Petrel "))
        .unwrap();
    (pole.id, painted)
}

#[test]
fn a_design_and_a_name_are_kept_and_replayed_exactly() {
    let mut branch = opened();
    let (pole, painted) = designed_and_named(&mut branch);
    let snapshot = branch.projection_snapshot();
    assert_eq!(item(&snapshot, pole).pattern, Some(painted.pattern()));
    let boat = item(&snapshot, SelectionId::Entity(crate::JONAS_BOAT));
    assert_eq!(boat.label, "Stormy Petrel");
    let named = branch
        .world()
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "named")
        .unwrap();
    assert_eq!(
        story::told(branch.world(), named).as_deref(),
        Some("You named the boat Stormy Petrel")
    );
    // Refused: what cannot wear a design, a broken design, a bad name.
    let bench = format!("tiny-society.hand.build.bench.{}", crate::PUB.0);
    branch.invoke_projection_command(&bench).unwrap_or_default();
    assert!(branch
        .invoke_projection_command(&format!(
            "tiny-society.mark.design.{}={}",
            crate::MARA.0,
            painted.text()
        ))
        .is_err());
    let SelectionId::Entity(pole_id) = pole else {
        unreachable!()
    };
    for bad in [
        format!("tiny-society.mark.design.{}=0000:1", pole_id.0),
        format!("tiny-society.mark.name.{}=", crate::JONAS_BOAT.0),
        format!("tiny-society.mark.name.{}=a\nb", crate::JONAS_BOAT.0),
        format!(
            "tiny-society.mark.name.{}={}",
            crate::JONAS_BOAT.0,
            "x".repeat(25)
        ),
        format!("tiny-society.mark.name.{}=Mara", crate::MARA.0),
    ] {
        assert!(branch.invoke_projection_command(&bad).is_err(), "{bad}");
    }
    // Replayed and reopened from its archive, exactly the same.
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
    let archive = branch.archive().unwrap();
    let reopened = TinySociety::resume_archive(&archive).unwrap();
    let again = reopened.projection_snapshot();
    assert_eq!(item(&again, pole).pattern, Some(painted.pattern()));
    assert_eq!(
        item(&again, SelectionId::Entity(crate::JONAS_BOAT)).label,
        "Stormy Petrel"
    );
}

#[test]
fn a_design_and_a_name_survive_a_world_code() {
    let mut branch = opened();
    let (pole, painted) = designed_and_named(&mut branch);
    for _ in 0..3 {
        pass(&mut branch);
    }
    let code = world_library::world_code_for_archive(branch.archive().unwrap(), None).unwrap();
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(crate::tiny_society_registration())
        .unwrap();
    let visit = world_library::WorldVisit::open_code(&code, &registry).unwrap();
    let snapshot = visit.snapshot();
    assert_eq!(item(&snapshot, pole).pattern, Some(painted.pattern()));
    assert_eq!(
        item(&snapshot, SelectionId::Entity(crate::JONAS_BOAT)).label,
        "Stormy Petrel"
    );
    let replayed = registry.open_archive(visit.archive()).unwrap();
    assert_eq!(replayed.snapshot(), snapshot);
}

/// Lets days pass until a baby is born; `None` if none is by `days`.
fn until_a_birth(branch: &mut TinySocietyBranch, days: usize) -> Option<world_core::EntityId> {
    for day in 0..days {
        let snapshot = branch.projection_snapshot();
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != story::WAIT_COMMAND
                && !command.id.contains(".mark.")
        }) {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if day % 3 == 0 {
            if let Some(deed) = snapshot.commands.iter().find(|command| {
                command.unavailable.is_none()
                    && command
                        .hand
                        .as_ref()
                        .is_some_and(|hand| hand.verb != "Undo")
            }) {
                let _ = branch.invoke_projection_command(&deed.id.clone());
            }
        }
        pass(branch);
        if let Some(Value::Entity(child)) = branch
            .world()
            .events_of_kind(&["born"])
            .last()
            .and_then(|born| born.payload.get("who"))
        {
            return Some(*child);
        }
    }
    None
}

#[test]
#[ignore]
fn a_newborns_parents_ask_for_a_name_and_it_is_used_everywhere() {
    let mut branch = opened();
    let child = until_a_birth(&mut branch, 900).expect("a baby in the harbour");
    let given = lives::name(branch.world().state(), child);
    let snapshot = branch.projection_snapshot();
    let card = snapshot
        .commands
        .iter()
        .filter(|command| {
            command
                .question
                .as_ref()
                .is_some_and(|question| question.id == format!("name-{}", child.0))
        })
        .collect::<Vec<_>>();
    assert_eq!(card.len(), 3, "the parents propose three names");
    assert_eq!(card[0].title, given, "the name they gave comes first");
    let baby = item(&snapshot, SelectionId::Entity(child));
    assert_eq!(baby.naming.as_ref().unwrap().proposals.len(), 3);
    // The player types their own.
    let naming = baby.naming.clone().unwrap();
    branch
        .invoke_projection_command(&command_with(&naming.command, "Marigold"))
        .unwrap();
    let world: &World = branch.world();
    assert_eq!(lives::name(world.state(), child), "Marigold");
    let born = world
        .events_of_kind(&["born"])
        .last()
        .copied()
        .unwrap()
        .clone();
    let told = story::told(world, &born).unwrap();
    assert!(
        told.contains("Marigold") && !told.contains(&given),
        "{told}"
    );
    // No card once named.
    let snapshot = branch.projection_snapshot();
    assert!(!snapshot
        .commands
        .iter()
        .any(|command| command.id.contains(&format!(".name.{}=", child.0))));
    assert_eq!(
        item(&snapshot, SelectionId::Entity(child)).label,
        "Marigold"
    );
    let Some(StoryPage::Legend(legend)) =
        branch.story(StoryRequest::Legend(SelectionId::Entity(child)))
    else {
        panic!("a legend");
    };
    assert!(legend.title.contains("Marigold"), "{legend:?}");
    assert!(
        legend.lines.iter().all(|line| !line.text.contains(&given)),
        "{legend:?}"
    );
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
}

/// The warm builder: the first question each day, and every third day the
/// first thing a plot offers (or, with every plot taken, the first thing to
/// make by hand).
pub(crate) fn warm_builder(
    days: usize,
    look: impl FnMut(usize, &TinySocietyBranch),
) -> TinySocietyBranch {
    warm(days, true, look)
}

fn warm(
    days: usize,
    plots: bool,
    mut look: impl FnMut(usize, &TinySocietyBranch),
) -> TinySocietyBranch {
    let mut branch = opened();
    for day in 1..=days {
        let snapshot = branch.projection_snapshot();
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.id != story::WAIT_COMMAND
        }) {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if day % 3 == 0 {
            let snapshot = branch.projection_snapshot();
            let offer = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .filter(|_| plots)
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone())
                .or_else(|| {
                    snapshot
                        .commands
                        .iter()
                        .find(|command| {
                            command.unavailable.is_none()
                                && command
                                    .hand
                                    .as_ref()
                                    .is_some_and(|hand| hand.verb != "Undo")
                        })
                        .map(|command| command.id.clone())
                });
            if let Some(offer) = offer {
                let _ = branch.invoke_projection_command(&offer);
            }
        }
        pass(&mut branch);
        look(day, &branch);
    }
    branch
}

#[test]
#[ignore]
fn a_warm_builder_draws_eight_newcomers_in_three_years() {
    let branch = warm_builder(1_080, |_, _| {});
    let world = branch.world();
    let drawn = world.events_of_kind(&["drawn_here"]);
    let built = world.events_of_kind(&["plot_finished"]).len();
    eprintln!(
        "{built} works finished on plots, {} newcomers drawn:",
        drawn.len()
    );
    for event in &drawn {
        eprintln!("  {}", story::told(world, event).unwrap_or_default());
    }
    let archive = branch.archive().unwrap();
    let code = world_library::world_code_for_archive(archive, None).unwrap();
    eprintln!(
        "the warm builder's World code at three years: {} characters",
        code.len()
    );
    assert!(drawn.len() >= 8, "{}", drawn.len());
    for event in &drawn {
        let who = event.targets[0];
        let Some(StoryPage::Legend(legend)) =
            branch.story(StoryRequest::Legend(SelectionId::Entity(who)))
        else {
            continue;
        };
        let by = match event.payload.get("drawn_by") {
            Some(Value::Entity(by)) => *by,
            _ => panic!("drawn by a work"),
        };
        let what = crate::plots::told_as(world.state(), by);
        assert!(
            legend.lines.iter().any(|line| line.text.contains(&what)),
            "{what}: {legend:?}"
        );
    }
    // The plan's 50,000 characters is for the warm player's harbour
    // (world-library's three_years, 47.9k). This one also houses the
    // eight people what the player built drew, each of whom the lives
    // System remembers much about: about 50.3k at v0.23.
    assert!(code.len() < 51_000, "{}", code.len());
}

/// Whether a line, translated, still has a word of English in it that is
/// not a name.
fn english(names: &BTreeSet<String>, text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word.len() > 1 && !names.contains(word))
}

/// Everything the player's mark shows in a harbour a warm builder has
/// lived in for a year, a design and names among it, is shown in Chinese.
#[test]
#[ignore]
fn the_mark_is_shown_in_chinese() {
    let mut catalog = world_i18n::Catalog::parse(include_str!(
        "../../../crates/world-builtins/locales/systems.zh-Hans.tsv"
    ));
    catalog.extend(crate::ZH_HANS);
    let mut shown = BTreeSet::new();
    let mut read = |snapshot: &ProjectionSnapshot| {
        for plot in &snapshot.canvas.plots {
            for offer in &plot.offers {
                shown.insert(offer.label.clone());
                shown.extend(offer.unavailable.clone());
            }
        }
        for item in &snapshot.canvas.items {
            if item.variant.is_some() {
                shown.insert(item.label.clone());
                shown.insert(item.detail.clone());
            }
        }
        for command in snapshot.commands.iter().filter(|c| c.id.contains(".mark.")) {
            shown.insert(command.detail.clone());
            shown.extend(command.question.as_ref().map(|q| q.prompt.clone()));
        }
    };
    let mut branch = warm(360, true, |_, branch| read(&branch.projection_snapshot()));
    // A design on everything that can wear one, and a name for everything
    // that can have one.
    let snapshot = branch.projection_snapshot();
    for item in &snapshot.canvas.items {
        if let Some(design) = &item.design {
            branch
                .invoke_projection_command(&command_with(&design.command, &a_design().text()))
                .unwrap();
        }
        if let Some(naming) = &item.naming {
            branch
                .invoke_projection_command(&command_with(&naming.command, "Stormy Petrel"))
                .unwrap();
        }
    }
    read(&branch.projection_snapshot());
    let world = branch.world();
    for event in world.events_of_kind(&[
        "built_by_hand",
        "plot_finished",
        "drawn_here",
        "named",
        "designed",
    ]) {
        shown.extend(story::told(world, event));
        shown.extend(story::line(event).map(|(_, said)| said));
        if event.kind == "drawn_here" {
            if let Some(StoryPage::Legend(legend)) =
                branch.story(StoryRequest::Legend(SelectionId::Entity(event.targets[0])))
            {
                shown.extend(legend.lines.into_iter().map(|line| line.text));
            }
        }
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
        .chain(crate::people_names().into_iter().map(str::to_string))
        .collect::<BTreeSet<_>>();
    let left = shown
        .iter()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let translated = catalog.translate(line).unwrap_or_else(|| line.clone());
            english(&names, &translated).then(|| format!("{line} => {translated}"))
        })
        .collect::<Vec<_>>();
    eprintln!("{} lines of the mark shown, among them:", shown.len());
    for line in shown.iter().step_by(20) {
        eprintln!(
            "  {line} => {}",
            catalog.translate(line).unwrap_or_default()
        );
    }
    assert!(left.is_empty(), "left in English:\n{}", left.join("\n"));
}

/// What a card says under a thing's name is the player's words: never an
/// engine word, what the World keeps it as, or a raw value.
pub(crate) fn check_details(snapshot: &ProjectionSnapshot) {
    const KEPT_AS: [&str; 8] = [
        "asset", "fixture", "entity", "resident", "order", "place", "person", "penguin",
    ];
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
            "{}: {:?}",
            item.label,
            item.detail
        );
    }
}

#[test]
fn every_card_speaks_the_players_words() {
    let mut branch = warm(45, true, |_, branch| {
        check_details(&branch.projection_snapshot())
    });
    // Named and painted, a boat is still a fishing boat.
    designed_and_named(&mut branch);
    let snapshot = branch.projection_snapshot();
    check_details(&snapshot);
    let boat = item(&snapshot, SelectionId::Entity(crate::JONAS_BOAT));
    assert_eq!(boat.detail, "A fishing boat in good repair");
}

#[test]
fn what_stands_on_a_plot_keeps_clear_of_what_is_put_up_after() {
    let mut branch = opened();
    // The harbour flag's plot, and a lamp post asked for right in front
    // of it.
    let (_, px) = build(&mut branch, "harbour_flag");
    let spot = ((px / crate::town::WIDTH) * 100.0).round() as u8;
    branch
        .invoke_projection_command(&hands::at_spot(
            &format!("tiny-society.hand.build.lamp.{}", crate::HARBOR.0),
            spot,
        ))
        .unwrap();
    let snapshot = branch.projection_snapshot();
    let lamp = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.label == "Lamp post")
        .unwrap();
    assert!(
        (lamp.px.unwrap() - px).abs() >= 0.08 - 1e-4,
        "the lamp at {:?}, the flag at {px}",
        lamp.px
    );
}
