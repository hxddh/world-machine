//! Legends, moments and almanacs as they cross the boundary (v7): asked
//! for with `story`, and the latest moments and a New Year's almanac sent
//! in the snapshot. Every field past the few that name a thing is optional
//! both ways, and what an app cannot show is left out rather than refused.

use crate::SelectionIdWire;
use serde::{Deserialize, Serialize};
use world_core::EventId;
use world_projection::{
    Almanac, Legend, LegendLine, Moment, MomentKind, Named, Panel, PanelBeat, StoryPage,
    StoryRequest,
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

/// What a player asks a World to tell.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryRequestWire {
    Legend { subject: SelectionIdWire },
    Moment { id: String },
    Almanac { year: u32 },
}

impl From<&StoryRequest> for StoryRequestWire {
    fn from(request: &StoryRequest) -> Self {
        match request {
            StoryRequest::Legend(subject) => Self::Legend {
                subject: (*subject).into(),
            },
            StoryRequest::Moment(id) => Self::Moment { id: id.clone() },
            StoryRequest::Almanac(year) => Self::Almanac { year: *year },
        }
    }
}

impl From<StoryRequestWire> for StoryRequest {
    fn from(request: StoryRequestWire) -> Self {
        match request {
            StoryRequestWire::Legend { subject } => Self::Legend(subject.into()),
            StoryRequestWire::Moment { id } => Self::Moment(id),
            StoryRequestWire::Almanac { year } => Self::Almanac(year),
        }
    }
}

/// What a World tells, as it crosses the boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoryPageWire {
    Legend { legend: LegendWire },
    Moment { moment: MomentWire },
    Almanac { almanac: AlmanacWire },
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
        }
    }
}

impl StoryPageWire {
    /// The page as an app shows it; a moment it could not draw is none.
    pub fn into_page(self) -> Option<StoryPage> {
        Some(match self {
            Self::Legend { legend } => StoryPage::Legend(legend.into()),
            Self::Moment { moment } => StoryPage::Moment(moment.into_moment()?),
            Self::Almanac { almanac } => StoryPage::Almanac(almanac.into()),
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
