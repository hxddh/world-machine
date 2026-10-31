use super::*;
use crate::{story, TinySocietyBranch};
use std::sync::OnceLock;
use world_projection::ProjectionSnapshot;

/// A warm player's first year from their arrival, and the first days of
/// the next: each day they answer the first question on offer, make
/// something every third day, and let the day pass. Shared by the tests
/// here and the achievements'.
pub(crate) fn year_lived() -> &'static TinySocietyBranch {
    static LIVED: OnceLock<TinySocietyBranch> = OnceLock::new();
    LIVED.get_or_init(|| {
        let mut branch = TinySocietyBranch::new_world().unwrap();
        for day in 1..=YEAR_DAYS + 3 {
            warm_day(&mut branch, day);
        }
        branch
    })
}

/// One day of the warm player.
pub(crate) fn warm_day(branch: &mut TinySocietyBranch, day: u64) {
    let snapshot = branch.projection_snapshot();
    if let Some(answer) = snapshot.commands.iter().find(|command| {
        command.question.is_some()
            && command.unavailable.is_none()
            && command.id != story::WAIT_COMMAND
    }) {
        let _ = branch.invoke_projection_command(&answer.id.clone());
    }
    if day.is_multiple_of(3) {
        let snapshot = branch.projection_snapshot();
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
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
}

fn festival_of(world: &World, page: &Event) -> Option<String> {
    page.caused_by
        .iter()
        .filter_map(|cause| world.event(*cause))
        .find(|cause| cause.kind == "festival_held")
        .and_then(|cause| text(cause, "festival").map(str::to_string))
}

#[test]
fn the_year_fills_its_four_pages_in_order_then_closes_with_credits() {
    let world = year_lived().world();
    let pages = world.events_of_kind(&[PAGE_FILLED]);
    // Spring arrival, the summer fair, the autumn gale, the winter lights:
    // in order, each caused by its own day.
    let order = pages
        .iter()
        .map(|page| text(page, "page").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(order, ["spring", "summer", "autumn", "winter"]);
    for (page, beat) in pages.iter().zip(&ARC) {
        assert_eq!(festival_of(world, page).as_deref(), Some(beat.festival));
        assert_eq!(year_of(page.world_time), 1);
    }
    assert!(pages
        .windows(2)
        .all(|two| two[0].world_time < two[1].world_time));
    // The year closes after its last page, on its last day, caused by it.
    let closed = world.events_of_kind(&[YEAR_CLOSED]);
    assert_eq!(closed.len(), 1);
    let close = closed[0];
    assert!(close.world_time > pages[3].world_time);
    assert_eq!(close.payload.get("year"), Some(&Value::Integer(1)));
    assert!(close
        .caused_by
        .iter()
        .filter_map(|cause| world.event(*cause))
        .any(calendar::is_day_begun));
    // Its credits: the residents and what each did, then the player.
    let snapshot = year_lived().projection_snapshot();
    let keeping = snapshot.keeping.as_ref().expect("the Book");
    let year = &keeping.closed[0];
    assert_eq!(year.year, 1);
    assert_eq!(year.title, "The end of year 1");
    let credit = |name: &str| {
        year.credits
            .iter()
            .find(|credit| credit.name == name)
            .unwrap_or_else(|| panic!("no credit for {name}: {:?}", year.credits))
    };
    assert_eq!(
        credit("Sofia").deed,
        "Led Spring Clean and the Midsummer Fair"
    );
    assert_eq!(credit("Noah").who, Some(SelectionId::Entity(NOAH)));
    for name in ["Mara", "Jonas", "Leo", "Emma", "Mia", "Evan"] {
        assert!(!credit(name).deed.is_empty());
    }
    let yours = year
        .credits
        .iter()
        .filter(|credit| credit.name == "You")
        .map(|credit| credit.deed.as_str())
        .collect::<Vec<_>>();
    assert_eq!(yours[0], "Filled 4 of the year's 4 pages");
    assert!(
        yours.iter().any(|deed| deed.starts_with("Made ")),
        "{yours:?}"
    );
    assert!(
        yours.iter().any(|deed| deed.starts_with("Answered ")),
        "{yours:?}"
    );
    assert_eq!(
        year_lived().world().replay().unwrap().state(),
        world.state()
    );
}

#[test]
fn play_goes_on_into_year_two_with_its_pages_to_fill() {
    let snapshot = year_lived().projection_snapshot();
    let keeping = snapshot.keeping.as_ref().unwrap();
    assert_eq!(keeping.year, 2);
    assert_eq!(keeping.role, ROLE);
    assert_eq!(keeping.pages.len(), 4);
    assert!(keeping.pages.iter().all(|page| !page.filled));
    assert_eq!(keeping.pages[0].title, "Spring: the harbour wakes");
    assert_eq!(keeping.pages[0].line, "Spring Clean will fill this page");
    // And the year two spring page fills too, from the same World.
    let mut branch = year_lived().clone();
    for day in 0..4 {
        warm_day(&mut branch, day);
    }
    let snapshot = branch.projection_snapshot();
    let page = &snapshot.keeping.as_ref().unwrap().pages[0];
    assert!(page.filled, "{page:?}");
    assert!(page.line.contains("Spring Clean"), "{}", page.line);
}

#[test]
fn the_first_year_opens_with_the_keeper_arriving() {
    let branch = TinySocietyBranch::new_world().unwrap();
    let snapshot = branch.projection_snapshot();
    let keeping = snapshot.keeping.as_ref().unwrap();
    assert_eq!(keeping.title, TITLE);
    assert_eq!(keeping.role, ROLE_NEW);
    assert_eq!(keeping.year, 1);
    assert_eq!(keeping.pages[0].title, "Spring: a new keeper arrives");
    assert!(keeping.closed.is_empty());
    // The hello names the player's role.
    let hello = snapshot.voices.first().unwrap();
    assert_eq!(hello.speaker, SelectionId::Entity(LEO));
    assert_eq!(
        hello.line,
        "Welcome to the harbour! I'm Leo. You must be our new keeper of the Book."
    );
    // Nothing is aimed at that the first day has done.
    let aim = |label: &str| {
        keeping
            .aims
            .iter()
            .find(|aim| aim.label == label)
            .unwrap_or_else(|| panic!("{label} in {:?}", keeping.aims))
    };
    assert_eq!(
        (aim("The lighthouse").done, aim("The lighthouse").of),
        (0, 5)
    );
    assert_eq!(aim("The lighthouse").next, "Clear the path to the point");
    assert_eq!(aim("The school roof").of, 4);
    assert_eq!(aim("Mara").of, 5);
    assert_eq!(
        keeping
            .aims
            .iter()
            .filter(|aim| aim.group == "Hearts")
            .count(),
        8
    );
    let finds = aim("The book of finds");
    assert_eq!(finds.of as usize, snapshot.book.len());
}

/// What the player makes and the help they give move the projects on,
/// a step at a time; nothing else does.
#[test]
fn projects_move_with_the_players_deeds() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let step = |branch: &TinySocietyBranch, id: &str| {
        projects(branch.world())
            .into_iter()
            .find(|(project, _)| project.id == id)
            .unwrap()
            .1
    };
    assert_eq!(step(&branch, "lighthouse"), 0);
    assert_eq!(step(&branch, "school_roof"), 0);
    // Days pass with nothing done: nothing moves.
    for _ in 0..3 {
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    assert_eq!(step(&branch, "lighthouse"), 0);
    let mut made = 0;
    for _ in 0..12 {
        let snapshot = branch.projection_snapshot();
        let deed = snapshot.commands.iter().find(|command| {
            command.unavailable.is_none()
                && command
                    .hand
                    .as_ref()
                    .is_some_and(|hand| hand.verb != "Undo" && hand.verb != "Move")
        });
        if let Some(deed) = deed {
            if branch.invoke_projection_command(&deed.id.clone()).is_ok() {
                made += 1;
            }
        }
        if made >= 3 {
            break;
        }
        branch
            .invoke_projection_command(story::WAIT_COMMAND)
            .unwrap();
    }
    let done = deeds(branch.world(), 0..u64::MAX);
    assert!(done.made >= 3, "{done:?}");
    assert!(step(&branch, "lighthouse") >= 1);
    // A warm year funds more of both.
    let lived = year_lived().world();
    let lighthouse = projects(lived)
        .into_iter()
        .find(|(project, _)| project.id == "lighthouse")
        .unwrap()
        .1;
    let roof = projects(lived)
        .into_iter()
        .find(|(project, _)| project.id == "school_roof")
        .unwrap()
        .1;
    assert!(lighthouse >= 2, "{lighthouse}");
    assert!(roof >= 2, "{roof}");
}

/// An answer the player gives one of the lives System's situations is a
/// deed like any other (it keeps the choice as its `answer`): it counts
/// toward the school roof and the year's answered questions.
#[test]
fn an_answered_situation_is_the_players_answer() {
    let world = year_lived().world();
    let situations = world.events_of_kind(&["situation_answered"]);
    assert!(!situations.is_empty(), "a warm year answers situations");
    assert!(situations.iter().all(|event| answered_by_player(event)));
    assert!(answer_kinds().contains(&"situation_answered"));
    assert!(!answer_kinds().contains(&"situation_lapsed"));
    let storylets = world
        .events_of_kind(answer_kinds())
        .into_iter()
        .filter(|event| event.kind != "situation_answered")
        .filter(|event| answered_by_player(event))
        .count() as u32;
    let done = deeds(world, 0..u64::MAX);
    assert_eq!(done.answered, storylets + situations.len() as u32);
}

/// Every word the Book shows reads whole in Chinese and in Japanese.
#[test]
fn the_book_and_its_role_read_in_chinese_and_japanese() {
    let catalog = |own: &str, systems: &str| {
        let mut catalog = world_i18n::Catalog::default();
        catalog.extend(systems);
        catalog.extend(own);
        catalog
    };
    let languages = [
        (
            "zh",
            catalog(
                crate::ZH_HANS,
                include_str!("../../../crates/world-builtins/locales/systems.zh-Hans.tsv"),
            ),
        ),
        (
            "ja",
            catalog(
                crate::JA,
                include_str!("../../../crates/world-builtins/locales/systems.ja.tsv"),
            ),
        ),
    ];
    let shown = |snapshot: &ProjectionSnapshot| {
        let keeping = snapshot.keeping.clone().unwrap();
        let mut lines = vec![keeping.title, keeping.role];
        for page in keeping.pages {
            lines.push(page.title);
            lines.push(page.line);
        }
        for year in keeping.closed {
            lines.push(year.title);
            for credit in year.credits {
                lines.push(credit.name);
                lines.push(credit.deed);
            }
        }
        for aim in keeping.aims {
            lines.extend([aim.group, aim.label, aim.next]);
        }
        lines
    };
    let new = TinySocietyBranch::new_world()
        .unwrap()
        .projection_snapshot();
    let mut lines = shown(&new);
    lines.extend(shown(&year_lived().projection_snapshot()));
    lines.push(new.voices[0].line.clone());
    for beat in &ARC {
        lines.extend([beat.title, beat.first, beat.to_come].map(String::from));
    }
    for project in &PROJECTS {
        lines.extend(project.steps.iter().map(|(_, step)| step.to_string()));
    }
    lines.extend(HEART.map(|beat| beat.replace("{name}", "Sofia")));
    lines.extend(
        [
            ROLE,
            "Led Lantern Night",
            "Asked you a favour",
            "Wrote you a letter",
            "Was part of the harbour's year",
            "Made one thing by hand",
            "Answered one question",
            "Did one favour",
            "Did 3 favours",
        ]
        .map(String::from),
    );
    for festival in crate::almanac::FESTIVALS
        .iter()
        .filter(|festival| ["autumn_gale", "winter_lights"].contains(&festival.id))
    {
        lines.push(festival.name.into());
        lines.extend(festival.told.iter().map(|told| format!("{told}.")));
        lines.extend(festival.said.iter().map(|said| said.to_string()));
        if !festival.nears.is_empty() {
            lines.push(festival.nears.replace("{days}", "3"));
            lines.push(festival.getting_ready.replace("{days}", "3"));
        }
    }
    lines.retain(|line| !line.is_empty());
    lines.sort();
    lines.dedup();
    let mut missing = Vec::new();
    for (language, catalog) in &languages {
        for line in &lines {
            match catalog.translate(line) {
                Some(put)
                    if put
                        .split(|c: char| !c.is_ascii_alphabetic())
                        .all(|word| word.len() < 2 || word.starts_with(char::is_uppercase)) => {}
                put => missing.push(format!("{language}: {line:?} → {put:?}")),
            }
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}
