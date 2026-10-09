//! Talk with a goal in the harbour: a favour asked, done by talking, and
//! thanked; one let lapse; and every kind of player asked one early.

use crate::{story, TinySocietyBranch};
use conversation::favour::{self, Kind};
use world_core::{Event, Value, World};
use world_pack_testkit::players::{self, Player, PLAYERS};
use world_projection::SelectionId;

fn pass(branch: &mut TinySocietyBranch) {
    branch
        .invoke_projection_command(story::WAIT_COMMAND)
        .unwrap();
}

fn day(branch: &TinySocietyBranch) -> u64 {
    branch.world().world_time() / crate::persistence::WORLD_DAY_TICKS + 1
}

fn asks(world: &World) -> Vec<&Event> {
    world.events_of_kind(&[favour::ASKED])
}

fn kind_of(event: &Event) -> Kind {
    match event.payload.get("favour") {
        Some(Value::Text(kind)) => Kind::from_id(kind).unwrap(),
        _ => panic!("a favour says what it is"),
    }
}

/// A new harbour, played until someone asks a favour: never in the first
/// two days.
fn asked() -> TinySocietyBranch {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    while asks(branch.world()).is_empty() {
        assert!(
            branch.projection_snapshot().favour.is_none(),
            "day {}",
            day(&branch)
        );
        pass(&mut branch);
        assert!(day(&branch) <= 10, "no favour by day 10");
    }
    assert!(day(&branch) >= 3, "asked on day {}", day(&branch));
    branch
}

/// What a player says to do a favour of `kind` asked by `asker`.
fn words(kind: Kind, asker: &str) -> String {
    match kind {
        Kind::AskAfter => "How are you doing?".into(),
        Kind::Invite => "Fancy a walk with me?".into(),
        Kind::CheerUp => "You're doing a great job, you know.".into(),
        Kind::Sorry => format!("{asker} says sorry."),
    }
}

#[test]
fn a_favour_is_asked_done_by_talking_and_thanked() {
    let mut branch = asked();
    let ask = asks(branch.world())[0].clone();
    let asker = ask.actor.unwrap();
    let whom = ask.targets[0];
    let state = branch.world().state();
    let (asker_name, whom_name) = (
        lives::first_name(state, asker),
        lives::first_name(state, whom),
    );
    // The asker says it aloud, and the drawer keeps a note of it.
    let snapshot = branch.projection_snapshot();
    let line = favour::said(state, &ask).expect("asked aloud").1;
    assert!(line.contains(&whom_name), "{line}");
    assert!(
        snapshot
            .voices
            .iter()
            .any(|voice| voice.speaker == SelectionId::Entity(asker) && voice.line == line),
        "{:?}",
        snapshot.voices
    );
    let shown = snapshot.favour.clone().expect("a note in the drawer");
    assert!(!shown.done);
    assert_eq!(shown.asker, SelectionId::Entity(asker));
    assert!(
        shown
            .note
            .starts_with(&format!("{asker_name} asked you to")),
        "{}",
        shown.note
    );
    assert!(shown.hint.contains(&whom_name), "{}", shown.hint);

    // Talking to someone else does nothing for it.
    let other = crate::story::people_in(branch.world().state())
        .into_iter()
        .find(|person| *person != whom && *person != asker)
        .unwrap();
    let events = branch.say(other, "Hello there!").unwrap();
    assert_eq!(events.len(), 1, "only the exchange");

    // Talking to whom it is for does it.
    let kind = kind_of(&ask);
    let regard = lives::regard(branch.world().state(), asker);
    let events = branch.say(whom, &words(kind, &asker_name)).unwrap();
    let world = branch.world();
    let spoken = world.event(events[0]).unwrap();
    let reply = match spoken.payload.get("reply") {
        Some(Value::Text(reply)) => reply.clone(),
        _ => panic!("answered"),
    };
    let done = world
        .events_of_kind(&[favour::DONE])
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{kind:?} not done by {reply:?}"));
    assert!(events.contains(&done.id));
    // Caused by the ask and by the words, and thanked by the asker.
    assert!(done.caused_by.contains(&ask.id), "{:?}", done.caused_by);
    assert!(done.caused_by.contains(&spoken.id), "{:?}", done.caused_by);
    assert_eq!(done.actor, Some(asker));
    assert!(lives::regard(world.state(), asker) > regard);
    let snapshot = branch.projection_snapshot();
    let thanks = favour::said(world.state(), done).unwrap().1;
    assert!(thanks.contains("Thank"), "{thanks}");
    assert!(snapshot
        .voices
        .iter()
        .any(|voice| voice.speaker == SelectionId::Entity(asker) && voice.line == thanks));
    assert!(snapshot.favour.as_ref().is_some_and(|favour| favour.done));
    // Whom it was for could tell who sent the player, except a kindness,
    // which speaks for itself.
    if kind != Kind::CheerUp {
        assert!(reply.contains(&asker_name), "{reply}");
    }
    // It is done once: saying it again does nothing more.
    let again = branch.say(whom, &words(kind, &asker_name)).unwrap();
    assert_eq!(again.len(), 1);
    // The next day the note is gone.
    pass(&mut branch);
    assert!(branch.projection_snapshot().favour.is_none());
    // And the World replays to where it stands without asking anyone.
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
}

#[test]
fn every_kind_of_favour_can_be_done() {
    // Played on until each kind has been asked once, doing each.
    let mut branch = asked();
    let mut done = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..200 {
        let open = favour::open(
            branch.world().state(),
            &crate::speech::kit(branch.world().state()),
        );
        if let Some(open) = open {
            if seen.insert(open.kind.id()) {
                let name = lives::first_name(branch.world().state(), open.asker);
                let before = branch.world().events_of_kind(&[favour::DONE]).len();
                // An invitation may be turned down by someone worn out;
                // then it is tried again the next day.
                branch.say(open.whom, &words(open.kind, &name)).unwrap();
                if branch.world().events_of_kind(&[favour::DONE]).len() > before {
                    done.insert(open.kind.id());
                } else {
                    seen.remove(open.kind.id());
                }
            }
        }
        if done.len() == Kind::ALL.len() {
            break;
        }
        pass(&mut branch);
    }
    assert_eq!(done.len(), Kind::ALL.len(), "{done:?}");
}

#[test]
fn a_favour_not_done_lapses_quietly() {
    let mut branch = asked();
    let asked_on = day(&branch);
    for _ in 0..favour::OPEN_PERIODS {
        pass(&mut branch);
        assert!(branch.projection_snapshot().favour.is_some());
    }
    pass(&mut branch);
    // Gone from the drawer, with nothing recorded and nothing held against
    // the player.
    let snapshot = branch.projection_snapshot();
    assert!(snapshot.favour.is_none(), "day {}", day(&branch));
    let world = branch.world();
    assert!(world.events_of_kind(&[favour::DONE]).is_empty());
    assert!(favour::open(world.state(), &crate::speech::kit(world.state())).is_none());
    assert_eq!(asks(world).len(), 1);
    // The next is asked by someone else, and, the last having lapsed with
    // no word to anyone since, a fortnight or so later rather than a week.
    while asks(branch.world()).len() < 2 {
        pass(&mut branch);
        assert!(day(&branch) <= asked_on + 21, "day {}", day(&branch));
    }
    let world = branch.world();
    let [first, second] = [asks(world)[0], asks(world)[1]];
    assert_ne!(first.actor, second.actor);
    assert!(day(&branch) >= asked_on + 12, "day {}", day(&branch));
}

#[test]
fn five_players_see_a_favour_within_ten_days() {
    for player in PLAYERS {
        let branch = TinySocietyBranch::new_world().unwrap();
        let played = players::play(format!("{player:?}"), player, 10, branch);
        let asked = asks(&played.world);
        let first = asked
            .first()
            .unwrap_or_else(|| panic!("{player:?}: no favour in 10 days"));
        let on = first.world_time / crate::persistence::WORLD_DAY_TICKS + 1;
        eprintln!(
            "{player:?}: day {on}, {} favours in 10 days; first: {:?}",
            asked.len(),
            favour::said(played.world.state(), first).map(|(_, line)| line)
        );
        assert!((3..=10).contains(&on), "{player:?}: day {on}");
    }
}

/// The v0.28 bar: a player who never talks is still asked a favour now
/// and then, about once every three weeks, for as long as they play;
/// never left out for good.
#[test]
fn a_quiet_player_is_still_asked_about_every_three_weeks() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    for _ in 0..150 {
        pass(&mut branch);
    }
    let on = asks(branch.world())
        .iter()
        .map(|ask| ask.world_time / crate::persistence::WORLD_DAY_TICKS + 1)
        .collect::<Vec<_>>();
    let gaps = on
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    eprintln!("a quiet player was asked on days {on:?}");
    assert!(on.len() >= 7, "{on:?}");
    assert!(
        gaps.iter()
            .all(|gap| *gap as i64 <= favour::QUIET_PERIODS + 3),
        "{on:?}"
    );
    assert!(day(&branch) - on.last().unwrap() <= 24, "{on:?}");
}

/// The v0.27 bars, for players who answer the first question, the last,
/// or none, over their first 30 days: every favour is asked aloud by
/// someone on the scene, at the moment the window opens on (so the camera
/// turns to them), with its note in the drawer and its words on whom it
/// is for's card; and none lapses unseen. Nobody here talks, so the asks
/// slow down after each lapse.
#[test]
fn players_see_every_favour_asked() {
    for player in [Player::Warm, Player::Last, Player::Never] {
        let (mut asked, mut seen) = (0, 0);
        let mut branch = TinySocietyBranch::new_world().unwrap();
        for _ in 0..30 {
            let before = branch.world().events().len();
            let snapshot = branch.projection_snapshot();
            let questions = snapshot
                .commands
                .iter()
                .filter(|command| {
                    command.question.is_some()
                        && command.unavailable.is_none()
                        && command.id != story::WAIT_COMMAND
                })
                .collect::<Vec<_>>();
            let answer = match player {
                Player::Warm => questions.first(),
                Player::Last => questions.last(),
                _ => None,
            };
            if let Some(answer) = answer {
                let _ = branch.invoke_projection_command(&answer.id.clone());
            }
            pass(&mut branch);
            let world = branch.world();
            let Some(ask) = world.events()[before..]
                .iter()
                .find(|event| event.kind == favour::ASKED)
            else {
                continue;
            };
            asked += 1;
            let after = branch.projection_snapshot();
            let asker = SelectionId::Entity(ask.actor.unwrap());
            let line = favour::said(world.state(), ask).unwrap().1;
            let on_scene = after.canvas.items.iter().any(|item| item.id == asker);
            let said_now =
                after.voices.iter().any(|voice| {
                    voice.speaker == asker
                        && voice.line == line
                        && after.timeline.items.iter().any(|item| {
                            item.id == voice.moment && item.world_time == after.world_time
                        })
                });
            let shown = after.favour.as_ref().is_some_and(|favour| {
                !favour.done && favour.asker == asker && !favour.reply.is_empty()
            });
            if on_scene && said_now && shown {
                seen += 1;
            } else {
                eprintln!(
                    "{player:?} day {}: unseen ({on_scene} {said_now} {shown})",
                    day(&branch)
                );
            }
        }
        eprintln!("{player:?}: {seen} of {asked} favours seen in 30 days");
        assert!(asked >= 2, "{player:?}: {asked} asked");
        assert_eq!(
            seen, asked,
            "{player:?}: every favour seen, none lapses unseen"
        );
        // Nobody here talks, so the asks slow to about one in three weeks.
        assert!(asked <= favour::MOST_LAPSED as usize, "{player:?}: {asked}");
    }
}

/// The person card's quick reply does its favour in every language the
/// app speaks: said in English, or as the catalogs put it in Chinese or
/// Japanese, it is heard as doing it.
#[test]
fn quick_replies_do_the_favour_in_every_language() {
    use conversation::hear;
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
    let mut branch = asked();
    let mut tried = std::collections::BTreeSet::new();
    for _ in 0..400 {
        if tried.len() == Kind::ALL.len() {
            break;
        }
        let state = branch.world().state();
        let kit = crate::speech::kit(state);
        if let Some(open) = favour::open(state, &kit) {
            if tried.insert(open.kind.id()) {
                let reply = branch.projection_snapshot().favour.unwrap().reply;
                let mut said = vec![("en", reply.clone())];
                for (language, catalog) in &languages {
                    let put = catalog
                        .translate(&reply)
                        .unwrap_or_else(|| panic!("{language}: no words for {reply:?}"));
                    said.push((language, put.to_string()));
                }
                for (language, words) in said {
                    let heard = hear(state, &kit, open.whom, &words);
                    assert!(
                        favour::done_by(open.kind, open.asker, heard, || true),
                        "{language} {:?}: {words:?} heard as {heard:?}",
                        open.kind
                    );
                }
            }
        }
        // Played on, nobody talking, until each kind has been asked; a
        // word to someone now and then keeps the favours coming.
        if day(&branch).is_multiple_of(9) {
            let someone = crate::story::people_in(branch.world().state())[0];
            branch.say(someone, "Morning!").unwrap();
        }
        pass(&mut branch);
    }
    assert_eq!(tried.len(), Kind::ALL.len(), "{tried:?}");
}

/// The folder of a fresh set kept in the repository.
fn fresh_dir(set: &str) -> String {
    format!(
        "{}/../../systems/conversation/tests/{set}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// How well the World's own ears hear phrases written fresh for v0.27
/// (`systems/conversation/tests/fresh27`), in one split, per language:
/// (meanings heard right, of how many; favours done, of how many).
fn fresh_heard(split: &str) -> Vec<(&'static str, usize, usize, usize, usize)> {
    let dir = std::env::var("FRESH_DIR").unwrap_or_else(|_| fresh_dir("fresh27"));
    fresh_heard_in(&dir, split, split == "held")
}

/// The file of one split of a set (`fresh_en.held.tsv`), or the set's
/// one file if it has no splits (`fresh_en.tsv`).
fn in_split(stem: &str, split: &str) -> String {
    let split_file = format!("{stem}.{split}.tsv");
    if std::path::Path::new(&split_file).exists() {
        split_file
    } else {
        format!("{stem}.tsv")
    }
}

/// How well the World's own ears hear the fresh set in `dir`, in one
/// split, per language. A `blind` set's misses are never printed, only
/// its totals.
fn fresh_heard_in(
    dir: &str,
    split: &str,
    blind: bool,
) -> Vec<(&'static str, usize, usize, usize, usize)> {
    use conversation::{hear, Intent};
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let actions = crate::build_action_registry().unwrap();
    let world = branch.world().clone();
    let state = world.state();
    let kit = crate::speech::kit(state);
    let named = |name: &str| {
        crate::story::people_in(state)
            .into_iter()
            .find(|id| lives::first_name(state, *id) == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    let lines = |path: String| {
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{path}: {error}"))
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let mut fields = line.split('\t');
                let mut next = || fields.next().unwrap_or_default().to_string();
                (next(), next(), next())
            })
            .collect::<Vec<_>>()
    };
    let mut scores = Vec::new();
    for lang in ["en", "zh", "ja"] {
        let (mut right, mut total) = (0, 0);
        for (meant, about, words) in lines(in_split(&format!("{dir}/fresh_{lang}"), split)) {
            let meant = Intent::from_id(&meant).unwrap();
            let heard = hear(state, &kit, crate::EMMA, &words);
            let ok =
                heard.intent == meant && (about.is_empty() || heard.about == Some(named(&about)));
            total += 1;
            right += usize::from(ok);
            if !ok && !blind {
                eprintln!(
                    "FRESH {lang} {split} meant {} heard {heard:?} | {words}",
                    meant.id()
                );
            }
        }
        let (mut done, mut asked) = (0, 0);
        for (kind, asker, words) in lines(in_split(&format!("{dir}/favour_{lang}"), split)) {
            let kind = Kind::from_id(&kind).unwrap();
            let asker = if asker.is_empty() {
                crate::MARA
            } else {
                named(&asker)
            };
            // Favour words are said while that favour is open, asked by
            // `asker` for Emma: the context they are heard in.
            let checkpoint = branch.world.checkpoint();
            branch
                .world
                .execute(
                    actions,
                    &world_core::ActionRequest::new("conversation_favour_ask")
                        .actor(asker)
                        .arg("asker", Value::Entity(asker))
                        .arg("whom", Value::Entity(crate::EMMA))
                        .arg("favour", kind.id()),
                )
                .expect("a favour opened for Emma");
            let open_state = branch.world.state();
            let heard = hear(
                open_state,
                &crate::speech::kit(open_state),
                crate::EMMA,
                &words,
            );
            branch.world.rollback(checkpoint);
            let ok = favour::done_by(kind, asker, heard, || true);
            asked += 1;
            done += usize::from(ok);
            if !ok && !blind {
                eprintln!(
                    "FAVOUR {lang} {split} {} heard {heard:?} | {words}",
                    kind.id()
                );
            }
        }
        eprintln!(
            "{} {split} {lang}: heard {right}/{total} ({:.1}%), favours done {done}/{asked} ({:.1}%)",
            dir.trim_end_matches('/').rsplit('/').next().unwrap_or(dir),
            100.0 * right as f64 / total as f64,
            100.0 * done as f64 / asked.max(1) as f64
        );
        scores.push((lang, right, total, done, asked));
    }
    scores
}

/// The v0.27 bar: phrases never tuned on (the held-out half) are heard
/// right at least 85% of the time in each language, and favours done with
/// them at least 75% of the time (the honest floor today; the bar is 90%).
/// What was tuned on stays heard.
#[test]
fn fresh_phrases_are_heard() {
    for (lang, right, total, ..) in fresh_heard("held") {
        assert!(
            right * 100 >= total * 85,
            "{lang}: {right} of {total} fresh phrases heard right"
        );
    }
    let (done, asked) = fresh_heard("held")
        .iter()
        .fold((0, 0), |(done, asked), score| {
            (done + score.3, asked + score.4)
        });
    assert!(done * 100 >= asked * 75, "favours: {done} of {asked}");
    for split in ["tune", "dev2", "dev3", "dev4"] {
        for (lang, right, total, done, asked) in fresh_heard(split) {
            assert!(right * 100 >= total * 95, "{split} {lang}: {right}/{total}");
            assert!(
                done * 100 >= asked * 95,
                "{split} {lang}: favours {done}/{asked}"
            );
        }
    }
}

/// v0.28's development set (`systems/conversation/tests/fresh28dev`),
/// written before v0.28's hearing work and tuned on: what was tuned on
/// stays heard.
#[test]
fn fresh28_development_phrases_stay_heard() {
    for split in ["dev", "check", "check2", "check3", "check4", "check5"] {
        for (lang, right, total, done, asked) in
            fresh_heard_in(&fresh_dir("fresh28dev"), split, false)
        {
            assert!(
                right * 100 >= total * 95,
                "fresh28dev {split} {lang}: {right}/{total}"
            );
            assert!(
                done * 100 >= asked * 95,
                "fresh28dev {split} {lang}: favours {done}/{asked}"
            );
        }
    }
}

/// v0.29's development set (`systems/conversation/tests/fresh29dev`),
/// written after the blind `fresh29` was frozen, in other registers:
/// `dev` tuned on and held at 95%; `check`, written after tuning on `dev`
/// and only partly tuned on, held at 90%.
#[test]
fn fresh29_development_phrases_stay_heard() {
    for (split, bar) in [("dev", 95), ("check", 90)] {
        for (lang, right, total, done, asked) in
            fresh_heard_in(&fresh_dir("fresh29dev"), split, false)
        {
            assert!(
                right * 100 >= total * bar,
                "fresh29dev {split} {lang}: {right}/{total}"
            );
            assert!(
                done * 100 >= asked * bar,
                "fresh29dev {split} {lang}: favours {done}/{asked}"
            );
        }
    }
}

/// Measures a blind fresh set kept outside the repository, once, and
/// prints only its totals: `FRESH_DIR` names its folder and `FRESH_SPLIT`
/// its split (`held` if unset). Bars (v0.29): at least 85% heard per
/// language, and at least 80% of favours done in all (a step toward 90%).
/// Favour words are heard with their favour open, as in play.
#[test]
#[ignore = "measures a blind set kept outside the repository"]
fn a_blind_fresh_set_is_heard() {
    let dir = std::env::var("FRESH_DIR").expect("FRESH_DIR names the blind set's folder");
    let split = std::env::var("FRESH_SPLIT").unwrap_or_else(|_| "held".into());
    let scores = fresh_heard_in(&dir, &split, true);
    let (done, asked) = scores.iter().fold((0, 0), |(done, asked), score| {
        (done + score.3, asked + score.4)
    });
    eprintln!(
        "blind favours: {done}/{asked} ({:.1}%)",
        100.0 * done as f64 / asked.max(1) as f64
    );
    for (lang, right, total, ..) in &scores {
        eprintln!(
            "blind heard {lang}: {right}/{total} ({:.1}%)",
            100.0 * *right as f64 / (*total).max(1) as f64
        );
    }
    for (lang, right, total, ..) in &scores {
        assert!(
            right * 100 >= total * 85,
            "{lang}: {right} of {total} heard right"
        );
    }
    assert!(done * 100 >= asked * 80, "favours: {done} of {asked}");
}
