//! One recorded Event staged as a scene of a few seconds: who walks
//! where, what props stand about, where the camera looks, and the words
//! said, so the player sees the event happen rather than reads of it.
//!
//! A scene is never recorded. It is worked out from the recorded Event
//! whenever it is asked for, by one blocking template a kind of scene:
//! the same Event always stages the same scene, a World from before scenes
//! has them, and a replayed history stages them without asking anyone to
//! decide anything again. Nobody says anything in a scene but the words
//! the Event recorded (`said`); the caption is the Pack's telling of the
//! Event. Where things are is said by marks (a place, beside someone, off
//! the stage, someone's home), which the app finds on the stage it draws.

use crate::{entity, text, Beat, Teller};
use world_core::{EntityId, Event, EventId, StateChange, Value};
use world_projection::{
    scene_id, Act, Cue, Mark, MomentKind, Prop, SceneScript, SelectionId, Stance,
};

/// The most people a scene stages.
pub const MOST_SCENE_CAST: usize = 4;

/// What kind of scene an Event of the Systems every World shares plays
/// as: `lives`' life beats, bonds and days, `hands`' deeds, `calendar`'s
/// festivals, and the Pack's storms. Anything else is `Other`.
pub fn kind_of(teller: &(impl Teller + ?Sized), event: &Event) -> MomentKind {
    let kind = event.kind.as_str();
    if teller.storm_kinds().contains(&kind) {
        return MomentKind::Storm;
    }
    match kind {
        "born" => MomentKind::Birth,
        "died" => MomentKind::Death,
        "came_of_age" => MomentKind::ComingOfAge,
        "retired" | "left_home" => MomentKind::Farewell,
        "drawn_here" => MomentKind::Arrival,
        "bond_changed" => match text(event, "bond") {
            Some("fell_out") => MomentKind::Quarrel,
            Some("made_up") => MomentKind::MakingUp,
            _ => MomentKind::Other,
        },
        "lived" if had_words(event) => MomentKind::Quarrel,
        "invited_out" => MomentKind::Outing,
        // A favour the player arranged: someone lonely taken out, or taken
        // along; and a party thrown.
        "situation_answered" => match (text(event, "kind"), text(event, "answer")) {
            (Some("lonely"), Some("invite")) | (Some("invite"), Some("go")) => MomentKind::Outing,
            (Some("party"), Some("party" | "potluck")) => MomentKind::Festival,
            _ if crate::kit::goes(event) => MomentKind::Farewell,
            _ if brings_someone(teller, event) => MomentKind::Arrival,
            _ => MomentKind::Other,
        },
        "built_by_hand" | "plot_finished" => MomentKind::Build,
        "festival_held" => MomentKind::Festival,
        _ if marries(event) => MomentKind::Wedding,
        _ if crate::kit::goes(event) => MomentKind::Farewell,
        _ if brings_someone(teller, event) => MomentKind::Arrival,
        _ => MomentKind::Other,
    }
}

/// Whether a day two people spent together ended in words between them:
/// `lives` marks whom each last had words with.
fn had_words(event: &Event) -> bool {
    !event.targets.is_empty()
        && event.changes.iter().any(
            |change| matches!(change, StateChange::SetComponent { key, .. } if key == lives::ROW),
        )
}

/// Whether an Event joined two people for good.
fn marries(event: &Event) -> bool {
    event.changes.iter().any(|change| {
        matches!(change, StateChange::SetComponent { key, value: Value::Entity(_), .. }
            if key.ends_with("married"))
    })
}

fn brings_someone(teller: &(impl Teller + ?Sized), event: &Event) -> bool {
    event.changes.iter().any(
        |change| matches!(change, StateChange::CreateEntity(made) if teller.is_person(made.id)),
    )
}

/// Who a scene stages, the one it is about first: whose beat it is, whom
/// it brought into the World, who acted, whom it concerned, whom it joined
/// them to, and then whoever else it touched; people only, at most [`MOST_SCENE_CAST`].
fn cast_of(teller: &impl Teller, event: &Event) -> Vec<EntityId> {
    let mut cast = Vec::new();
    let joined = event.changes.iter().filter_map(|change| match change {
        StateChange::SetComponent {
            key,
            value: Value::Entity(other),
            ..
        } if key.ends_with("married") => Some(*other),
        _ => None,
    });
    let touched = event.changes.iter().filter_map(|change| match change {
        StateChange::SetComponent { entity, .. } => Some(*entity),
        _ => None,
    });
    let brought = event.changes.iter().filter_map(|change| match change {
        StateChange::CreateEntity(made) => Some(made.id),
        _ => None,
    });
    for id in entity(event, "who")
        .into_iter()
        .chain(brought)
        .chain(event.actor)
        .chain(event.targets.iter().copied())
        .chain(joined)
        .chain(touched)
    {
        if cast.len() >= MOST_SCENE_CAST {
            break;
        }
        if !cast.contains(&id) && teller.is_person(id) {
            cast.push(id);
        }
    }
    cast
}

/// Whatever an Event made that is not a person: the bench built, the work
/// begun.
fn made_thing(teller: &impl Teller, event: &Event) -> Option<EntityId> {
    event.changes.iter().find_map(|change| match change {
        StateChange::CreateEntity(made) if !teller.is_person(made.id) => Some(made.id),
        _ => None,
    })
}

/// Where a scene happens: where the Event put whom it is about, the place
/// it names, the first thing it concerned that it did not make, or the
/// Pack's home. Read from the Event alone.
fn place_of(teller: &impl Teller, event: &Event, cast: &[EntityId]) -> Option<EntityId> {
    let made = made_thing(teller, event);
    cast.first()
        .and_then(|who| {
            event.changes.iter().find_map(|change| match change {
                StateChange::SetComponent {
                    entity,
                    key,
                    value: Value::Entity(place),
                } if entity == who && (key == "location" || key == lives::AT) => Some(*place),
                _ => None,
            })
        })
        .or_else(|| entity(event, "place"))
        .or_else(|| {
            event
                .targets
                .iter()
                .copied()
                .find(|id| Some(*id) != made && !teller.is_person(*id))
        })
        .or_else(|| teller.scene_home())
}

/// Whether a prop is how people come and go.
fn is_transport(prop: Prop) -> bool {
    matches!(prop, Prop::Ferry | Prop::Shuttle | Prop::Bus | Prop::Sled)
}

/// The beat a scene stages, as the Pack's [`Teller`] reads `event`: its
/// kind, its cast and place, and its caption in the World's words. How
/// people come and go here is shown at an arrival and a farewell.
pub fn beat<'a>(teller: &impl Teller, event: &'a Event) -> Beat<'a> {
    let kind = teller.scene_kind(event);
    let cast = cast_of(teller, event);
    let place = place_of(teller, event, &cast);
    let caption = teller.scene_caption(event).unwrap_or_default();
    let mut props = [Vec::new(), Vec::new(), Vec::new()];
    if matches!(kind, MomentKind::Arrival | MomentKind::Farewell) {
        props[1].extend(teller.transport());
    }
    Beat {
        event,
        kind,
        title: String::new(),
        cast,
        place,
        captions: [String::new(), caption, String::new()],
        moods: [None; 3],
        props,
    }
}

/// The scene of one recorded Event, with how everyone in it looks; none
/// when the World recorded no such Event.
pub fn scene(teller: &impl Teller, id: EventId) -> Option<SceneScript> {
    let event = teller.world().event(id)?;
    let mut script = script(&beat(teller, event));
    for (who, look) in &mut script.cast {
        if let SelectionId::Entity(id) = who {
            *look = teller.look(*id, event.world_time);
        }
    }
    Some(script)
}

/// The cues of a scene as they are written, put in time order.
struct Blocking {
    cues: Vec<Cue>,
}

impl Blocking {
    fn new() -> Self {
        Self { cues: Vec::new() }
    }

    fn cue(&mut self, at: f32, who: Option<EntityId>, act: Act) {
        self.cues.push(Cue {
            at,
            who: who.map(SelectionId::Entity),
            act,
        });
    }

    /// Someone in place at the start, or walking somewhere later.
    fn walk(&mut self, at: f32, who: EntityId, to: Mark, stance: Stance) {
        self.cue(at, Some(who), Act::WalkTo { to, stance });
    }

    fn strike(&mut self, at: f32, who: EntityId, stance: Stance) {
        self.cue(at, Some(who), Act::Strike(stance));
    }

    fn prop(&mut self, at: f32, prop: Prop, mark: Mark, shown: bool) {
        self.cue(
            at,
            None,
            Act::Prop {
                prop,
                at: mark,
                shown,
            },
        );
    }

    fn camera(&mut self, at: f32, on: Mark, zoom: f32) {
        self.cue(at, None, Act::Camera { on, zoom });
    }

    fn done(mut self) -> Vec<Cue> {
        // In time order, and in the order written at the same time.
        self.cues.sort_by(|a, b| a.at.total_cmp(&b.at));
        self.cues
    }
}

fn at(who: EntityId) -> Mark {
    Mark::Place(SelectionId::Entity(who))
}

fn beside(of: Mark, steps: i8) -> Mark {
    match of {
        Mark::Place(id) | Mark::Home(id) => Mark::Beside(id, steps),
        other => other,
    }
}

fn home(who: EntityId) -> Mark {
    Mark::Home(SelectionId::Entity(who))
}

/// Steps along from a mark for the `n`th of a crowd: 2, -2, 3, -3...
fn spread(n: usize, from: i8) -> i8 {
    let step = from + (n / 2) as i8;
    if n.is_multiple_of(2) {
        step
    } else {
        -step
    }
}

/// A beat staged as a scene, by its kind's blocking: a pure function of
/// the beat, and so of the recorded Event. Everyone in the cast is staged
/// but the one who died at a death, who is named but not walked on. The
/// only words said are the Event's own `said`, by whoever recorded them
/// (or, if they are not on the stage, by no one in particular).
pub fn script(beat: &Beat<'_>) -> SceneScript {
    let event = beat.event;
    let cast = &beat.cast;
    let a = cast.first().copied();
    let b = cast.get(1).copied();
    let rest = cast.iter().skip(2).copied().collect::<Vec<_>>();
    let place = beat
        .place
        .map(|place| Mark::Place(SelectionId::Entity(place)))
        .or(a.map(home))
        .unwrap_or(Mark::Offstage(true));
    let said = text(event, "said").map(str::to_string);
    let speaker = event.actor.filter(|actor| {
        cast.contains(actor) && !(beat.kind == MomentKind::Death && Some(*actor) == a)
    });
    let transport = beat.props[1]
        .iter()
        .copied()
        .find(|prop| is_transport(*prop));
    let mut stage = Blocking::new();
    let say = |stage: &mut Blocking, at: f32| {
        if let Some(words) = &said {
            stage.cue(at, speaker, Act::Say(words.clone()));
        }
    };
    // The crowd, gathered round the place and taking up a stance together.
    let crowd = |stage: &mut Blocking, people: &[EntityId], from: i8, stance: Stance| {
        for (n, who) in people.iter().enumerate() {
            stage.walk(0.0, *who, beside(place, spread(n, from)), stance);
        }
    };
    let seconds = match beat.kind {
        MomentKind::Arrival => {
            // Someone new comes on with a bag, and those already here wave.
            stage.camera(0.0, place, 1.0);
            if let Some(transport) = transport {
                stage.prop(0.0, transport, Mark::Offstage(false), true);
                stage.prop(6.5, transport, Mark::Offstage(false), false);
            }
            let others = cast.iter().skip(1).copied().collect::<Vec<_>>();
            crowd(&mut stage, &others, 2, Stance::Standing);
            if let Some(a) = a {
                stage.walk(0.0, a, Mark::Offstage(false), Stance::Carrying);
                stage.prop(0.0, Prop::Suitcase, beside(at(a), 0), true);
                stage.walk(0.5, a, place, Stance::Walking);
                stage.camera(1.0, at(a), 1.6);
                stage.strike(4.0, a, Stance::Waving);
            }
            for who in &others {
                stage.strike(4.2, *who, Stance::Waving);
            }
            say(&mut stage, 4.5);
            8.0
        }
        MomentKind::Quarrel => {
            // Words between two, and one of them goes off home.
            stage.camera(0.0, a.map(at).unwrap_or(place), 1.8);
            if let Some(a) = a {
                stage.walk(0.0, a, place, Stance::Talking);
                if let Some(b) = b {
                    stage.walk(0.0, b, beside(at(a), 1), Stance::Talking);
                    stage.walk(3.5, b, home(b), Stance::Walking);
                }
                stage.strike(5.0, a, Stance::LookingAround);
            }
            crowd(&mut stage, &rest, 3, Stance::Standing);
            say(&mut stage, 1.0);
            7.0
        }
        MomentKind::MakingUp => {
            // Two come from either side to meet in the middle.
            stage.camera(0.0, place, 1.4);
            if let Some(a) = a {
                stage.walk(0.0, a, beside(place, -3), Stance::Standing);
                stage.walk(0.5, a, beside(place, -1), Stance::Walking);
                stage.strike(3.0, a, Stance::Talking);
                stage.strike(5.0, a, Stance::Celebrating);
            }
            if let Some(b) = b {
                stage.walk(0.0, b, beside(place, 3), Stance::Standing);
                stage.walk(0.5, b, beside(place, 1), Stance::Walking);
                stage.strike(5.0, b, Stance::Celebrating);
            }
            crowd(&mut stage, &rest, 4, Stance::Standing);
            say(&mut stage, 3.2);
            7.0
        }
        MomentKind::Outing => {
            // Whoever was invited goes from home to where they were asked.
            stage.prop(0.0, Prop::Table, beside(place, 1), true);
            stage.camera(0.0, a.map(home).unwrap_or(place), 1.3);
            stage.camera(2.0, place, 1.2);
            if let Some(a) = a {
                stage.walk(0.0, a, home(a), Stance::Standing);
                stage.walk(0.5, a, place, Stance::Walking);
                stage.strike(4.0, a, Stance::Celebrating);
                stage.strike(6.0, a, Stance::Dancing);
            }
            let others = cast.iter().skip(1).copied().collect::<Vec<_>>();
            crowd(&mut stage, &others, 2, Stance::Standing);
            for who in &others {
                stage.strike(4.0, *who, Stance::Talking);
            }
            say(&mut stage, 5.0);
            8.0
        }
        MomentKind::Build => {
            // Scaffolding round what is going up, tools, and hands at work;
            // once it is finished the scaffolding comes down.
            let made = made_or_target(beat).map(at).unwrap_or(place);
            let begun = event.kind == "built_by_hand" && event.payload.contains_key("plot");
            stage.camera(0.0, made, 1.5);
            stage.prop(0.0, Prop::Scaffold, made, true);
            stage.prop(0.0, Prop::Tools, beside(made, -1), true);
            for (n, who) in cast.iter().enumerate() {
                stage.walk(0.0, *who, beside(made, spread(n, 1)), Stance::Standing);
                stage.strike(0.5, *who, Stance::Working);
            }
            if !begun {
                stage.prop(5.5, Prop::Scaffold, made, false);
                stage.prop(5.5, Prop::Tools, beside(made, -1), false);
                stage.camera(6.0, made, 1.0);
                for who in cast {
                    stage.strike(6.0, *who, Stance::Celebrating);
                }
            }
            say(&mut stage, 3.0);
            8.0
        }
        MomentKind::Festival => {
            // Bunting and tables, everyone coming from home, and dancing.
            stage.prop(0.0, Prop::Bunting, place, true);
            stage.prop(0.0, Prop::Table, beside(place, 2), true);
            stage.camera(0.0, place, 0.9);
            for (n, who) in cast.iter().enumerate() {
                stage.walk(0.0, *who, home(*who), Stance::Standing);
                stage.walk(
                    0.3 + 0.4 * n as f32,
                    *who,
                    beside(place, spread(n, 1)),
                    Stance::Walking,
                );
                stage.strike(4.0, *who, Stance::Dancing);
            }
            say(&mut stage, 4.5);
            // In close while they gather, so the dance is seen.
            stage.camera(1.5, place, 1.3);
            9.0
        }
        MomentKind::Storm => {
            // Rain sets in, and everyone hurries home to a lit lamp.
            stage.cue(0.0, None, Act::Weather(Prop::Rain));
            stage.camera(0.0, place, 0.8);
            crowd(&mut stage, cast, 1, Stance::Standing);
            for who in cast {
                stage.walk(1.0, *who, home(*who), Stance::Walking);
            }
            say(&mut stage, 2.0);
            if let Some(a) = a {
                stage.prop(3.0, Prop::Lamp, home(a), true);
            }
            8.0
        }
        MomentKind::Birth => {
            // A parent carries the baby out, the cradle by them, and the
            // neighbours come to see.
            let parent = b.or(a);
            if let Some(parent) = parent {
                stage.walk(0.0, parent, place, Stance::Carrying);
                stage.camera(1.5, at(parent), 1.8);
            }
            if let (Some(baby), Some(parent)) = (a, b) {
                stage.walk(0.0, baby, beside(at(parent), 0), Stance::Standing);
            }
            stage.camera(0.0, place, 1.2);
            stage.prop(0.0, Prop::Cradle, beside(place, 1), true);
            for (n, who) in rest.iter().enumerate() {
                stage.walk(0.0, *who, beside(place, spread(n, 5)), Stance::Standing);
                stage.walk(1.0, *who, beside(place, spread(n, 2)), Stance::Walking);
                stage.strike(3.0, *who, Stance::Waving);
                stage.strike(5.5, *who, Stance::Celebrating);
            }
            say(&mut stage, 2.5);
            8.0
        }
        MomentKind::Wedding => {
            // Two side by side under the bunting, and everyone dancing.
            stage.prop(0.0, Prop::Bunting, place, true);
            stage.camera(0.0, place, 1.0);
            if let Some(a) = a {
                stage.walk(0.0, a, beside(place, -1), Stance::Standing);
                stage.strike(5.0, a, Stance::Celebrating);
            }
            if let Some(b) = b {
                stage.walk(0.0, b, beside(place, 1), Stance::Standing);
                stage.prop(0.0, Prop::Bouquet, beside(at(b), 0), true);
                stage.strike(5.0, b, Stance::Celebrating);
            }
            crowd(&mut stage, &rest, 3, Stance::Standing);
            for who in &rest {
                stage.strike(5.5, *who, Stance::Dancing);
            }
            stage.camera(2.0, place, 1.6);
            say(&mut stage, 3.0);
            9.0
        }
        MomentKind::Death => {
            // A lamp at their window; the others come and lay a wreath.
            if let Some(a) = a {
                stage.prop(0.0, Prop::Lamp, home(a), true);
            }
            stage.camera(0.0, place, 1.4);
            for (n, who) in cast.iter().skip(1).enumerate() {
                stage.walk(0.0, *who, home(*who), Stance::Standing);
                stage.walk(0.5, *who, beside(place, spread(n, 1)), Stance::Walking);
                stage.strike(3.5, *who, Stance::Standing);
            }
            stage.prop(3.0, Prop::Wreath, place, true);
            say(&mut stage, 4.0);
            8.0
        }
        MomentKind::Farewell => {
            // Bag in hand, a last wave, and off the edge of the stage.
            stage.camera(0.0, place, 1.2);
            if let Some(transport) = transport {
                stage.prop(0.0, transport, Mark::Offstage(true), true);
                stage.prop(7.0, transport, Mark::Offstage(true), false);
            }
            let others = cast.iter().skip(1).copied().collect::<Vec<_>>();
            crowd(&mut stage, &others, 1, Stance::Standing);
            for who in &others {
                stage.strike(1.5, *who, Stance::Waving);
            }
            if let Some(a) = a {
                stage.walk(0.0, a, place, Stance::Carrying);
                stage.prop(0.0, Prop::Suitcase, beside(at(a), 0), true);
                stage.walk(3.5, a, Mark::Offstage(true), Stance::Walking);
                stage.camera(3.5, at(a), 1.2);
            }
            say(&mut stage, 2.0);
            8.0
        }
        MomentKind::ComingOfAge => {
            // From home to work, and the tools taken up.
            stage.prop(0.0, Prop::Tools, beside(place, 1), true);
            stage.camera(0.0, place, 1.2);
            if let Some(a) = a {
                stage.walk(0.0, a, home(a), Stance::Standing);
                stage.walk(0.5, a, place, Stance::Walking);
                stage.strike(3.5, a, Stance::Working);
            }
            if let Some(b) = b {
                stage.walk(0.0, b, beside(place, -2), Stance::Standing);
                stage.strike(4.0, b, Stance::Waving);
            }
            crowd(&mut stage, &rest, 3, Stance::Standing);
            say(&mut stage, 4.5);
            7.0
        }
        MomentKind::WorkOpened => {
            // The scaffolding comes down, the ribbon is cut, and a cheer.
            stage.prop(0.0, Prop::Scaffold, place, true);
            stage.prop(0.0, Prop::Ribbon, beside(place, 0), true);
            stage.camera(0.0, place, 1.0);
            crowd(&mut stage, cast, 2, Stance::Standing);
            stage.prop(2.0, Prop::Scaffold, place, false);
            stage.prop(3.0, Prop::Ribbon, beside(place, 0), false);
            for who in cast {
                stage.strike(3.2, *who, Stance::Celebrating);
            }
            stage.camera(3.2, place, 1.4);
            say(&mut stage, 3.5);
            8.0
        }
        MomentKind::Other => {
            // Whoever it is about, at the place, telling it.
            stage.camera(0.0, a.map(at).unwrap_or(place), 1.5);
            if let Some(a) = a {
                stage.walk(0.0, a, place, Stance::Talking);
                if let Some(b) = b {
                    stage.walk(0.0, b, beside(at(a), 1), Stance::Standing);
                    stage.strike(3.0, b, Stance::Talking);
                }
            }
            crowd(&mut stage, &rest, 3, Stance::Standing);
            say(&mut stage, 1.0);
            6.0
        }
    };
    // Whatever else the beat shows, about the place from the start.
    for prop in beat.props[1].iter().copied() {
        if is_transport(prop) || stage.shows(prop) {
            continue;
        }
        stage.prop(0.0, prop, place, true);
    }
    SceneScript {
        id: scene_id(event.id),
        event: event.id,
        kind: beat.kind,
        place: beat.place.map(SelectionId::Entity),
        seconds,
        cast: cast
            .iter()
            .map(|who| (SelectionId::Entity(*who), None))
            .collect(),
        cues: stage.done(),
        caption: beat.captions[1].clone(),
    }
}

/// What a build scene is of: whatever the Event made, or else what it
/// concerned first that is not one of the cast.
fn made_or_target(beat: &Beat<'_>) -> Option<EntityId> {
    let event = beat.event;
    event
        .changes
        .iter()
        .find_map(|change| match change {
            StateChange::CreateEntity(made) if !beat.cast.contains(&made.id) => Some(made.id),
            _ => None,
        })
        .or_else(|| {
            event
                .targets
                .iter()
                .copied()
                .find(|id| !beat.cast.contains(id))
        })
}

impl Blocking {
    /// Whether the scene already shows a prop.
    fn shows(&self, prop: Prop) -> bool {
        self.cues
            .iter()
            .any(|cue| matches!(&cue.act, Act::Prop { prop: shown, .. } if *shown == prop))
    }
}
