//! Legends, moments and almanacs as they cross the boundary (v7): asked
//! for with `story`, and the latest moments and a New Year's almanac sent
//! in the snapshot. Every field past the few that name a thing is optional
//! both ways, and what an app cannot show is left out rather than refused.
//!
//! A scene (one recorded Event staged for a few seconds) is asked for the
//! same way, of a Pack that says it can stage them
//! ([`crate::CAPABILITY_SCENES`]); a cue an app cannot stage (an act, a
//! mark, a stance or a prop it does not know) is passed over.

use crate::{LookWire, SelectionIdWire};
use serde::{Deserialize, Serialize};
use world_core::EventId;
use world_projection::{
    Act, Almanac, Cue, Legend, LegendLine, Mark, Moment, MomentKind, Named, Panel, PanelBeat,
    SceneScript, Stance, StoryPage, StoryRequest, MOST_SCENE_SECONDS,
};

/// The most lines one legend carries: the oldest are kept, since a life is
/// told from its start.
pub const MOST_LEGEND_LINES: usize = 1_000;
/// The most names one list of an almanac carries.
pub const MOST_ALMANAC_NAMES: usize = 200;
/// The most people or places one panel draws.
pub const MOST_PANEL_CAST: usize = 12;
/// The most props one panel shows.
pub const MOST_PANEL_PROPS: usize = 6;
/// The most years an app is told it can ask the almanac of.
pub const MOST_ALMANAC_YEARS: usize = 1_000;
/// The most people one scene stages.
pub const MOST_SCENE_CAST: usize = 12;
/// The most cues one scene has.
pub const MOST_SCENE_CUES: usize = 200;
/// The most a scene's camera zooms out, and in.
const ZOOM_RANGE: (f32, f32) = (0.25, 4.0);

/// What a player asks a World to tell.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryRequestWire {
    Legend {
        subject: SelectionIdWire,
    },
    Moment {
        id: String,
    },
    Almanac {
        year: u32,
    },
    /// One recorded Event, staged as a scene.
    Scene {
        event: u64,
    },
}

impl From<&StoryRequest> for StoryRequestWire {
    fn from(request: &StoryRequest) -> Self {
        match request {
            StoryRequest::Legend(subject) => Self::Legend {
                subject: (*subject).into(),
            },
            StoryRequest::Moment(id) => Self::Moment { id: id.clone() },
            StoryRequest::Almanac(year) => Self::Almanac { year: *year },
            StoryRequest::Scene(event) => Self::Scene { event: event.0 },
        }
    }
}

impl From<StoryRequestWire> for StoryRequest {
    fn from(request: StoryRequestWire) -> Self {
        match request {
            StoryRequestWire::Legend { subject } => Self::Legend(subject.into()),
            StoryRequestWire::Moment { id } => Self::Moment(id),
            StoryRequestWire::Almanac { year } => Self::Almanac(year),
            StoryRequestWire::Scene { event } => Self::Scene(EventId::new(event)),
        }
    }
}

/// What a World tells, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryPageWire {
    Legend {
        legend: LegendWire,
    },
    Moment {
        moment: MomentWire,
    },
    Almanac {
        almanac: AlmanacWire,
    },
    Scene {
        scene: SceneScriptWire,
    },
    /// A kind of page from a newer Pack (v8): no page.
    #[serde(other)]
    Unknown,
}

impl From<&StoryPage> for StoryPageWire {
    fn from(page: &StoryPage) -> Self {
        match page {
            StoryPage::Legend(legend) => Self::Legend {
                legend: legend.into(),
            },
            StoryPage::Moment(moment) => Self::Moment {
                moment: moment.into(),
            },
            StoryPage::Almanac(almanac) => Self::Almanac {
                almanac: almanac.into(),
            },
            StoryPage::Scene(scene) => Self::Scene {
                scene: scene.into(),
            },
        }
    }
}

impl StoryPageWire {
    /// The page as an app shows it; a moment it could not draw, or a scene
    /// it could not stage, is none.
    pub fn into_page(self) -> Option<StoryPage> {
        Some(match self {
            Self::Legend { legend } => StoryPage::Legend(legend.into()),
            Self::Moment { moment } => StoryPage::Moment(moment.into_moment()?),
            Self::Almanac { almanac } => StoryPage::Almanac(almanac.into()),
            Self::Scene { scene } => StoryPage::Scene(scene.into_scene()?),
            Self::Unknown => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendWire {
    pub subject: SelectionIdWire,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<LegendLineWire>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendLineWire {
    #[serde(default)]
    pub day: u32,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<u64>,
}

impl From<&Legend> for LegendWire {
    fn from(legend: &Legend) -> Self {
        Self {
            subject: legend.subject.into(),
            title: legend.title.clone(),
            lines: legend
                .lines
                .iter()
                .map(|line| LegendLineWire {
                    day: line.day,
                    text: line.text.clone(),
                    because: line.because.clone(),
                    event: line.event.map(|id| id.0),
                    cause: line.cause.map(|id| id.0),
                })
                .collect(),
        }
    }
}

impl From<LegendWire> for Legend {
    fn from(legend: LegendWire) -> Self {
        Self {
            subject: legend.subject.into(),
            title: legend.title,
            lines: legend
                .lines
                .into_iter()
                .filter(|line| !line.text.trim().is_empty())
                .take(MOST_LEGEND_LINES)
                .map(|line| {
                    // A cause with no words is not shown, nor words with
                    // no cause.
                    let (because, cause) = match (line.because, line.cause) {
                        (Some(because), Some(cause)) if !because.trim().is_empty() => {
                            (Some(because), Some(EventId::new(cause)))
                        }
                        _ => (None, None),
                    };
                    LegendLine {
                        day: line.day,
                        text: line.text,
                        because,
                        event: line.event.map(EventId::new),
                        cause,
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PanelWire {
    #[serde(default)]
    pub caption: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cast: Vec<SelectionIdWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<SelectionIdWire>,
    /// A mood this build does not know is drawn as none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,
    /// "before", "moment" or "after"; the panel's place in the strip says
    /// the same, and wins.
    #[serde(default)]
    pub beat: String,
    /// What else the panel shows ("ferry", "bunting"); an id this build
    /// does not know is left out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MomentWire {
    pub id: String,
    #[serde(default)]
    pub day: u32,
    /// A kind this build does not know is drawn as `other`.
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    /// Before, the moment and after, in that order.
    pub panels: Vec<PanelWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<u64>,
}

impl From<&Panel> for PanelWire {
    fn from(panel: &Panel) -> Self {
        Self {
            caption: panel.caption.clone(),
            cast: panel.cast.iter().copied().map(Into::into).collect(),
            place: panel.place.map(Into::into),
            mood: panel.mood.map(|mood| mood.id().to_string()),
            beat: panel.beat.id().to_string(),
            props: panel
                .props
                .iter()
                .map(|prop| prop.id().to_string())
                .collect(),
        }
    }
}

impl From<&Moment> for MomentWire {
    fn from(moment: &Moment) -> Self {
        Self {
            id: moment.id.clone(),
            day: moment.day,
            kind: moment.kind.id().to_string(),
            title: moment.title.clone(),
            panels: moment.panels.iter().map(Into::into).collect(),
            event: moment.event.map(|id| id.0),
        }
    }
}

impl MomentWire {
    /// The moment as an app draws it: one with no id, or without its three
    /// panels, is none.
    pub fn into_moment(self) -> Option<Moment> {
        if self.id.trim().is_empty() || self.panels.len() != 3 {
            return None;
        }
        let mut panels = self
            .panels
            .into_iter()
            .zip(PanelBeat::ALL)
            .map(|(panel, beat)| Panel {
                caption: panel.caption,
                cast: panel
                    .cast
                    .into_iter()
                    .take(MOST_PANEL_CAST)
                    .map(Into::into)
                    .collect(),
                place: panel.place.map(Into::into),
                mood: panel
                    .mood
                    .as_deref()
                    .and_then(world_projection::Mood::from_id),
                beat,
                props: panel
                    .props
                    .iter()
                    .filter_map(|prop| world_projection::Prop::from_id(prop))
                    .take(MOST_PANEL_PROPS)
                    .collect(),
            });
        let panels = [panels.next()?, panels.next()?, panels.next()?];
        Some(Moment {
            id: self.id,
            day: self.day,
            kind: MomentKind::from_id(&self.kind),
            title: self.title,
            panels,
            event: self.event.map(EventId::new),
        })
    }

    pub(crate) fn selections(&self) -> impl Iterator<Item = SelectionIdWire> + '_ {
        self.panels
            .iter()
            .flat_map(|panel| panel.cast.iter().copied().chain(panel.place))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedWire {
    pub name: String,
    pub who: SelectionIdWire,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AlmanacWire {
    pub year: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arrived: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub born: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub died: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub built: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cast: Vec<NamedWire>,
}

impl AlmanacWire {
    pub(crate) fn selections(&self) -> impl Iterator<Item = SelectionIdWire> + '_ {
        self.cast.iter().map(|named| named.who)
    }
}

impl From<&Almanac> for AlmanacWire {
    fn from(almanac: &Almanac) -> Self {
        Self {
            year: almanac.year,
            title: almanac.title.clone(),
            arrived: almanac.arrived.clone(),
            left: almanac.left.clone(),
            born: almanac.born.clone(),
            died: almanac.died.clone(),
            built: almanac.built.clone(),
            best: almanac.best.clone(),
            cast: almanac
                .cast
                .iter()
                .map(|named| NamedWire {
                    name: named.name.clone(),
                    who: named.who.into(),
                })
                .collect(),
        }
    }
}

fn names(names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| !name.trim().is_empty())
        .take(MOST_ALMANAC_NAMES)
        .collect()
}

impl From<AlmanacWire> for Almanac {
    fn from(almanac: AlmanacWire) -> Self {
        Self {
            year: almanac.year,
            title: almanac.title,
            arrived: names(almanac.arrived),
            left: names(almanac.left),
            born: names(almanac.born),
            died: names(almanac.died),
            built: names(almanac.built),
            best: almanac.best.filter(|best| !best.trim().is_empty()),
            cast: almanac
                .cast
                .into_iter()
                .filter(|named| !named.name.trim().is_empty())
                .take(MOST_ALMANAC_NAMES * 5)
                .map(|named| Named {
                    name: named.name,
                    who: named.who.into(),
                })
                .collect(),
        }
    }
}

/// Somewhere on the stage, as it crosses the boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarkWire {
    Place {
        of: SelectionIdWire,
    },
    Beside {
        of: SelectionIdWire,
        #[serde(default)]
        steps: i8,
    },
    Offstage {
        #[serde(default)]
        right: bool,
    },
    Home {
        of: SelectionIdWire,
    },
    /// A mark from a newer Pack: its cue is passed over.
    #[serde(other)]
    Unknown,
}

impl From<Mark> for MarkWire {
    fn from(mark: Mark) -> Self {
        match mark {
            Mark::Place(of) => Self::Place { of: of.into() },
            Mark::Beside(of, steps) => Self::Beside {
                of: of.into(),
                steps,
            },
            Mark::Offstage(right) => Self::Offstage { right },
            Mark::Home(of) => Self::Home { of: of.into() },
        }
    }
}

impl MarkWire {
    fn mark(self) -> Option<Mark> {
        Some(match self {
            Self::Place { of } => Mark::Place(of.into()),
            Self::Beside { of, steps } => Mark::Beside(of.into(), steps),
            Self::Offstage { right } => Mark::Offstage(right),
            Self::Home { of } => Mark::Home(of.into()),
            Self::Unknown => return None,
        })
    }
}

/// What happens at one cue, as it crosses the boundary: stances and props
/// by their ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActWire {
    WalkTo {
        to: MarkWire,
        /// A stance this build does not know is walked standing.
        #[serde(default)]
        stance: String,
    },
    Strike {
        stance: String,
    },
    Say {
        words: String,
    },
    Prop {
        prop: String,
        at: MarkWire,
        #[serde(default)]
        shown: bool,
    },
    Camera {
        on: MarkWire,
        #[serde(default = "one")]
        zoom: f32,
    },
    Weather {
        prop: String,
    },
    /// An act from a newer Pack: passed over.
    #[serde(other)]
    Unknown,
}

fn one() -> f32 {
    1.0
}

impl From<&Act> for ActWire {
    fn from(act: &Act) -> Self {
        match act {
            Act::WalkTo { to, stance } => Self::WalkTo {
                to: (*to).into(),
                stance: stance.id().into(),
            },
            Act::Strike(stance) => Self::Strike {
                stance: stance.id().into(),
            },
            Act::Say(words) => Self::Say {
                words: words.clone(),
            },
            Act::Prop { prop, at, shown } => Self::Prop {
                prop: prop.id().into(),
                at: (*at).into(),
                shown: *shown,
            },
            Act::Camera { on, zoom } => Self::Camera {
                on: (*on).into(),
                zoom: *zoom,
            },
            Act::Weather(prop) => Self::Weather {
                prop: prop.id().into(),
            },
        }
    }
}

impl ActWire {
    /// The act as an app stages it; one it cannot is none.
    fn act(self) -> Option<Act> {
        let prop = |id: &str| world_projection::Prop::from_id(id);
        Some(match self {
            Self::WalkTo { to, stance } => Act::WalkTo {
                to: to.mark()?,
                stance: Stance::from_id(&stance).unwrap_or_default(),
            },
            Self::Strike { stance } => Act::Strike(Stance::from_id(&stance)?),
            Self::Say { words } if !words.trim().is_empty() => Act::Say(words),
            Self::Say { .. } => return None,
            Self::Prop {
                prop: id,
                at,
                shown,
            } => Act::Prop {
                prop: prop(&id)?,
                at: at.mark()?,
                shown,
            },
            Self::Camera { on, zoom } => Act::Camera {
                on: on.mark()?,
                zoom: if zoom.is_finite() {
                    zoom.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1)
                } else {
                    1.0
                },
            },
            Self::Weather { prop: id } => Act::Weather(prop(&id)?),
            Self::Unknown => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CueWire {
    #[serde(default)]
    pub at: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<SelectionIdWire>,
    pub act: ActWire,
}

/// Someone a scene stages, and how they look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CastWire {
    pub who: SelectionIdWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<LookWire>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneScriptWire {
    pub id: String,
    pub event: u64,
    /// A kind this build does not know is staged as `other`.
    #[serde(default)]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<SelectionIdWire>,
    pub seconds: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cast: Vec<CastWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<CueWire>,
    #[serde(default)]
    pub caption: String,
}

impl From<&SceneScript> for SceneScriptWire {
    fn from(scene: &SceneScript) -> Self {
        Self {
            id: scene.id.clone(),
            event: scene.event.0,
            kind: scene.kind.id().to_string(),
            place: scene.place.map(Into::into),
            seconds: scene.seconds,
            cast: scene
                .cast
                .iter()
                .map(|(who, look)| CastWire {
                    who: (*who).into(),
                    look: look.map(LookWire::from),
                })
                .collect(),
            cues: scene
                .cues
                .iter()
                .map(|cue| CueWire {
                    at: cue.at,
                    who: cue.who.map(Into::into),
                    act: (&cue.act).into(),
                })
                .collect(),
            caption: scene.caption.clone(),
        }
    }
}

impl SceneScriptWire {
    /// The scene as an app stages it: one with no id, or no time to play
    /// in, is none. It plays for at most [`MOST_SCENE_SECONDS`]; cues it
    /// cannot stage, or that fall outside its time or move someone not in
    /// its cast, are passed over, and the rest put in time order.
    pub fn into_scene(self) -> Option<SceneScript> {
        if self.id.trim().is_empty() || !self.seconds.is_finite() || self.seconds <= 0.0 {
            return None;
        }
        let seconds = self.seconds.min(MOST_SCENE_SECONDS);
        let cast = self
            .cast
            .into_iter()
            .take(MOST_SCENE_CAST)
            .map(|cast| (cast.who.into(), cast.look.map(Into::into)))
            .collect::<Vec<(world_projection::SelectionId, _)>>();
        let mut cues = self
            .cues
            .into_iter()
            .filter(|cue| cue.at.is_finite() && cue.at >= 0.0 && cue.at < seconds)
            .filter_map(|cue| {
                let who = cue.who.map(world_projection::SelectionId::from);
                if who.is_some_and(|who| !cast.iter().any(|(cast, _)| *cast == who)) {
                    return None;
                }
                Some(Cue {
                    at: cue.at,
                    who,
                    act: cue.act.act()?,
                })
            })
            .take(MOST_SCENE_CUES)
            .collect::<Vec<_>>();
        cues.sort_by(|a, b| a.at.total_cmp(&b.at));
        Some(SceneScript {
            id: self.id,
            event: EventId::new(self.event),
            kind: MomentKind::from_id(&self.kind),
            place: self.place.map(Into::into),
            seconds,
            cast,
            cues,
            caption: self.caption,
        })
    }
}
