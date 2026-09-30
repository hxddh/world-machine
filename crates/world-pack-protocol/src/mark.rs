//! The player's mark on the wire: plots, designs, names and how the town
//! built something. Every field is optional both ways: a Pack that predates
//! them sends none, and a host that predates them ignores them. What a Pack
//! sends is checked here too, so a malformed design or a runaway list never
//! reaches a screen.

use serde::{Deserialize, Serialize};
use world_projection::{
    clean_name, Design, Designable, MarkShape, Naming, Pattern, Plot, PlotOffer, Variant, Wears,
};

use crate::{MarkShapeWire, MOST_CANVAS_WIDTH};

/// The most plots one place offers.
pub const MOST_PLOTS: usize = 96;
/// The most things one plot offers.
pub const MOST_PLOT_OFFERS: usize = 16;
/// The most names proposed for one thing.
pub const MOST_PROPOSALS: usize = 8;
/// The longest command id a plot offer, a design or a name is sent with.
pub const MOST_MARK_COMMAND: usize = 256;

fn is_false(value: &bool) -> bool {
    !*value
}

fn command_ok(command: &str) -> bool {
    !command.trim().is_empty()
        && command.len() <= MOST_MARK_COMMAND
        && !command.chars().any(char::is_control)
}

/// A design as drawn: its 256 cells and its colours as `[r, g, b]`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatternWire {
    pub cells: String,
    pub palette: Vec<[u8; 3]>,
}

impl From<&Pattern> for PatternWire {
    fn from(pattern: &Pattern) -> Self {
        Self {
            cells: pattern.cells.clone(),
            palette: pattern.palette.clone(),
        }
    }
}

impl PatternWire {
    /// The pattern, if it is a whole one of the fixed palette's colours.
    pub fn known(self) -> Option<Pattern> {
        let pattern = Pattern {
            cells: self.cells,
            palette: self.palette,
        };
        Design::from_pattern(&pattern)
            .ok()
            .map(|design| design.pattern())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DesignableWire {
    pub command: String,
    /// What it is: `flag`, `sail`, `sign` or `quilt`; anything else reads
    /// as nothing to design on.
    pub wears: String,
}

impl From<&Designable> for DesignableWire {
    fn from(design: &Designable) -> Self {
        Self {
            command: design.command.clone(),
            wears: design.wears.id().into(),
        }
    }
}

impl DesignableWire {
    pub fn known(self) -> Option<Designable> {
        let wears = Wears::from_id(&self.wears)?;
        command_ok(&self.command).then_some(Designable {
            command: self.command,
            wears,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NamingWire {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposals: Vec<String>,
}

impl From<&Naming> for NamingWire {
    fn from(naming: &Naming) -> Self {
        Self {
            command: naming.command.clone(),
            proposals: naming.proposals.clone(),
        }
    }
}

impl NamingWire {
    pub fn known(self) -> Option<Naming> {
        command_ok(&self.command).then(|| Naming {
            command: self.command,
            proposals: self
                .proposals
                .iter()
                .filter_map(|name| clean_name(name).ok())
                .take(MOST_PROPOSALS)
                .collect(),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VariantWire {
    pub colour: [u8; 3],
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub join_left: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub join_right: bool,
}

impl From<Variant> for VariantWire {
    fn from(variant: Variant) -> Self {
        Self {
            colour: variant.colour,
            flip: variant.flip,
            join_left: variant.join_left,
            join_right: variant.join_right,
        }
    }
}

impl From<VariantWire> for Variant {
    fn from(variant: VariantWire) -> Self {
        Self {
            colour: variant.colour,
            flip: variant.flip,
            join_left: variant.join_left,
            join_right: variant.join_right,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlotOfferWire {
    pub command: String,
    pub label: String,
    #[serde(default)]
    pub shape: MarkShapeWire,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
    /// What the app draws it as; an older Pack sends none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlotWire {
    pub id: String,
    pub px: f32,
    #[serde(default)]
    pub row: u8,
    #[serde(default)]
    pub district: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offers: Vec<PlotOfferWire>,
}

impl From<&Plot> for PlotWire {
    fn from(plot: &Plot) -> Self {
        Self {
            id: plot.id.clone(),
            px: plot.px,
            row: plot.row,
            district: plot.district.clone(),
            offers: plot
                .offers
                .iter()
                .map(|offer| PlotOfferWire {
                    command: offer.command.clone(),
                    label: offer.label.clone(),
                    shape: offer.shape.into(),
                    cost: offer.cost.clone(),
                    unavailable: offer.unavailable.clone(),
                    art: offer.art.clone(),
                })
                .collect(),
        }
    }
}

impl PlotWire {
    /// The plot, if it lies somewhere on a panorama and has an id.
    pub fn known(self) -> Option<Plot> {
        (self.px.is_finite() && !self.id.trim().is_empty()).then(|| Plot {
            id: self.id,
            px: self.px.clamp(0.0, MOST_CANVAS_WIDTH),
            row: self.row,
            district: self.district,
            offers: self
                .offers
                .into_iter()
                .filter(|offer| command_ok(&offer.command))
                .take(MOST_PLOT_OFFERS)
                .map(|offer| PlotOffer {
                    command: offer.command,
                    label: offer.label,
                    shape: MarkShape::from(offer.shape),
                    cost: offer.cost,
                    unavailable: offer.unavailable,
                    art: crate::art_key(offer.art),
                })
                .collect(),
        })
    }
}
