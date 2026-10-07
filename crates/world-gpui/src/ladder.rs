//! One scale for everything (the art bible's §3): every drawing's height
//! in **P**, the height of a standing grown-up resident at the same depth.
//!
//! A postbox is 0.75 P, a door 1.15 P, a lamp post 1.8 P, a cottage's
//! ridge 2.75 P, a pub 3.3 P, a windmill 5 P and the lighthouse 6 P. A
//! thing is drawn so that what it paints stands that tall beside the
//! people at its depth, whatever the width of the spot it was given; a
//! long, low thing (a pier, a pond, a court) is sized by its footprint
//! instead, and its height follows its own drawing.
//!
//! How tall a drawing paints for the box it is given is measured, once per
//! drawing, by painting it: so the ladder holds for every drawing in the
//! library, the harbour's own shapes and the Packs' drawings alike, and a
//! new drawing is held to it the moment it is added (see the tests).
//!
//! Everything here is presentation.

use crate::art::{self, Palette, Setting};
use crate::brush::{Brush, Rect, Shape};
use crate::works::{self, Art, Drawn};
use gpui::{Hsla, RenderImage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use world_projection::{Drawing, MarkShape, Mood, Stance};

/// A rung of the ladder: how tall something stands, or for a long, low
/// thing how wide it lies, in P.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rung {
    Tall(f32),
    Wide(f32),
}

/// The rung of a drawing from the library. Every drawing has one: the
/// test `every_drawing_has_a_rung` keeps it so.
pub fn of_art(key: &str) -> Option<Rung> {
    use Rung::{Tall as T, Wide as W};
    Some(match key {
        // ---- At home anywhere -------------------------------------------
        "bandstand" => T(3.0),
        "roofed-bandstand" => T(3.2),
        "clock-tower" | "clock-tower-square" | "bell-tower" => T(5.0),
        "telescope" => T(1.3),
        "market-cross" => T(2.6),
        "sundial" | "wishing-fountain" | "wellhead" | "cairn" | "time-capsule"
        | "drinking-fountain" | "ice-well" | "marker-cairn" => T(1.0),
        "park-bench" | "metal-bench" | "memorial-bench" | "crater-bench" => T(0.8),
        "picnic-table" | "chess-tables" => T(0.75),
        "picnic-tables" => W(3.0),
        "mooring-bench" => T(0.7),
        // A turf mound over a cellar: about a door and a half.
        "ice-house" => T(1.5),
        "sheepfold" => T(0.8),
        // Low and mixed, along a path.
        "wildflowers" => T(0.45),
        "wildflower-patch" => T(0.4),
        // A painted board on two posts, about a notice board.
        "harbour-mural" => T(1.2),
        "allotments" => T(0.95),
        "beehives" | "bee-garden" => T(0.6),
        "school-garden" | "hen-house" | "apple-press" | "planter-racks" | "window-box-stand"
        | "fern-planter" => T(0.95),
        "herb-garden" => T(0.6),
        "swing-set" | "playground-swing" | "garden-swing" => T(1.7),
        "rowing-boat" => T(0.5),
        "gazebo" | "street-gazebo" | "summer-house" => T(2.5),
        "square-fountain" | "mist-fountain" | "water-pump" => T(1.15),
        "maypole" | "flagpole-street" => T(3.5),
        "windmill" => T(5.0),
        "greenhouse" | "glasshouse" | "school-greenhouse" | "greenhouse-tent" => T(2.0),
        "treehouse" | "maple-tree" => T(4.0),
        "bleachers" => T(1.4),
        "library" | "gym-hall" | "village-hall" => T(3.3),
        "science-shed" | "crab-shack" | "study-hut" | "herring-shed" | "quiet-pod" => T(2.2),
        "sailboat" => T(3.0),
        "fishing-dock" | "lake-dock" | "steel-footbridge" | "footbridge" | "dome-walkway"
        | "ice-bridge" | "rope-bridge" | "landing-pad" | "sea-wall" | "garden-plots"
        | "roller-floor" | "dance-floor" | "soapbox-track" | "winter-rink" | "skate-park" => W(4.0),
        "rope-swing" | "apple-tree" | "orchard" | "hilltop-swing" | "kites" => T(3.0),
        "flower-beds" | "vegetable-patch" | "herb-pots" | "quay-planters" | "herb-bed"
        | "flower-bed" | "median-beds" | "pebble-planters" | "kelp-beds" | "lichen-garden"
        | "algae-beds" | "glow-stones" => T(0.5),
        "town-clock" | "harbour-clock" => T(2.8),
        "ice-cream-stand" | "hot-dog-stand" => T(1.5),
        "boathouse" | "rowing-club" | "clubhouse" | "bus-depot" | "bunkhouse" => T(2.8),
        "sunflowers" => T(1.0),
        "flower-boxes" | "window-boxes" | "rose-bed" => T(0.6),
        "paper-lanterns" | "fairy-lights-street" | "flag-line" => T(2.2),
        "lamp-post" | "point-lamp" | "solar-lamp" | "lantern-post" | "neon-lamp" => T(1.8),
        "market-stall" | "flower-stall" | "fish-stall" | "pay-phone" => T(1.3),
        "birdhouse" | "birdhouses" | "bird-feeder" | "signpost" | "lookout-seat" => T(1.6),
        "wall-fountain" => T(1.1),
        "scaffold" => T(2.75),
        // ---- The harbour ------------------------------------------------
        "lighthouse" => T(6.0),
        "bin-gate" | "diner-board" | "tape-shelf" | "book-box" => T(1.2),
        "fishers-statue" => T(1.9),
        "fisherman-statue" | "founders-statue" | "mayor-statue" | "street-sign" => T(2.0),
        "roofed-well" | "wind-shelter" | "snow-marker" => T(1.4),
        // The bible's postbox, whatever it is called where it stands.
        "pillar-box" | "lamp-box" | "mailbox" | "message-stone" => T(0.75),
        "message-post" => T(1.0),
        "fingerpost" => T(1.7),
        "lifeboat-station" | "relay-hut" | "observatory-dome" => T(3.0),
        "bathing-huts" | "kayak-shelter" | "rest-canopy" | "fairy-lights-mars" | "supply-tent" => {
            T(1.8)
        }
        "smokehouse" | "boat-yard" | "music-shed" | "net-store" | "weaving-shed" | "dust-lock"
        | "seed-vault" | "rover-shed" | "rover-lift" | "survey-rig" | "mess-module"
        | "music-pod" | "rover-garage" | "carving-hall" | "diner" | "garage" | "snack-bar"
        | "streamers" | "bank-clock" | "story-berg" | "snow-flag" | "bridge-gate" | "bone-arch"
        | "weathervane" => T(2.4),
        "fish-market" | "workshop" | "joinery" | "hab-module" | "clinic-bay" | "school-pod"
        | "trading-post" | "library-module" | "flower-dome" | "workshop-dome"
        | "schoolroom-module" | "second-habitat" | "radio-dish" | "survey-station"
        | "radio-hut" | "darkroom" | "lookout-post" | "whale-watch" | "snow-hall" => T(2.6),
        "bread-oven" | "story-chair" | "notice-board" | "net-rack" | "pool-name-boards"
        | "windbreak-panels" | "kelp-racks" | "song-stone" | "thaw-marker" | "ridge-steps"
        | "name-wall" | "chick-slide" | "pebble-market" | "street-survey" | "market-stalls" => {
            T(1.4)
        }
        "puppet-theatre" | "park-stage" | "neon-sign" | "vault-ice-house" | "ice-market"
        | "greenhouse-annexe" | "kelp-frond" | "rose-arbour" | "windsock" | "fuel-tanks" => T(2.2),
        "duck-pond" | "sea-pool" | "shell-path" | "cliff-path" | "slipway" | "splash-pool"
        | "walk-of-fame" | "lake-path" | "council-ring" | "thaw-channel" | "swim-pool"
        | "snow-maze" | "fishing-hole" | "breathing-hole" | "moss-patch" | "lichen-patch" => W(3.0),
        "row-cottages" => T(2.75),
        "hill-beacon" | "ridge-beacon" => T(2.5),
        "ferry-shelter" | "seal-hide" | "fishers-shelter" | "bait-shed" | "storm-shelter"
        | "dust-shelter" | "second-vault" | "warming-hut" | "snow-arch" | "sail-sled"
        | "far-lantern" | "bell-post" | "bus-shelter" => T(2.0),
        "dovecote" | "lantern-walk" => T(2.6),
        "tide-board" | "dwarf-apple" => T(2.0),
        "reading-room" | "tea-rooms" | "bookshop" | "cheese-shop" | "surgery" | "pottery"
        | "gallery" | "quilting-room" | "machine-shop" | "storefront" | "video-store"
        | "record-library" | "study-room" | "youth-centre" | "computer-room"
        | "recording-studio" | "record-store" | "arcade" | "barber-shop" | "quilt-shop"
        | "second-street" | "row-house" | "bandshell" | "cable-pylons" | "rec-dome"
        | "welcome-arch" | "ice-drill" | "seal-watch" => T(3.0),
        "mural-wall" => T(1.8),
        "lookout-tower" | "mine-headframe" | "control-tower" => T(4.5),
        "net-loft" | "sail-loft" => T(3.3),
        "book-nook" | "bird-table" | "telescope-pad" | "solar-array" | "algae-farm"
        | "water-still" | "welcome-sign" | "bridge-lanterns" | "sea-slide" | "geyser-fountain"
        | "ice-survey" | "sculpture-garden" | "shell-horn" | "chick-nursery" => T(1.6),
        "bread-cart" => T(1.0),
        "pub-terrace" | "boat-rack" => T(1.6),
        "paddling-pool" | "pebble-mosaic" | "stepping-stones" | "salt-pans" | "elders-ramp"
        | "story-circle" | "song-circle" | "lantern-ring" | "frog-pond" => W(2.5),
        "windbreak" => T(1.2),
        "chapel" | "beacon-mast" | "radio-beacon" | "ice-spire" => T(4.0),
        "jetty-ladder" => T(1.2),
        "driftwood-bench" | "fishing-hole-bench" | "crate-seat" | "ice-bench" => T(0.65),
        "oak-rope-swing" | "berg-lookout" | "weather-mast" | "rialto-marquee"
        | "drive-in-screen" => T(3.5),
        "painted-stones" | "red-moss" => T(0.3),
        "sandpit" | "pebble-garden" | "seaweed-bed" => W(2.0),
        "skittle-alley" | "solar-field" | "ice-ledge" | "floe-bridge" | "skating-rink"
        | "dog-run" => W(3.5),
        "fire-pit" | "grow-tray" | "landing-lights" | "kelp-garden" | "tomato-patch"
        | "boat-planter" | "seal-fence" | "egg-warmer" => T(0.7),
        "story-stone" | "landing-stone" | "trail-marker" => T(1.2),
        "jar-lanterns" | "pennant-line" | "ice-sculpture" | "creche" | "fish-larder"
        | "ice-slide" | "kelp-swing" | "low-g-swing" => T(1.8),
        "new-pier" => W(5.0),
        "harbour-lamp" | "streetlamp" | "street-lamp-post" | "streetlights" => T(2.0),
        "supply-cache" | "nesting-box" | "party-speakers" | "aurora-seat" | "rover-seat" => T(1.0),
        // ---- Mars -------------------------------------------------------
        "supply-module" | "mars-dome" | "grow-dome" | "mess-hall-dome" | "hydroponics-dome"
        | "infirmary-dome" | "water-tank" => T(2.8),
        "comms-mast" | "tall-antenna" => T(5.0),
        "dust-sled" => T(0.8),
        "condenser" | "geyser" => T(1.4),
        "cargo-depot" => T(2.2),
        "cargo-crate" | "sandbag-wall" | "fish-crate" => T(0.6),
        "track-beacons" => T(0.8),
        "radio-tower" | "transmitter" | "water-tower" => T(6.0),
        // ---- Maple Street -----------------------------------------------
        "newspaper-bundle" => T(0.35),
        "parked-car" => T(0.85),
        "call-in-booth" => T(1.35),
        "crosswalk" => W(3.5),
        "basketball-court" => T(2.0),
        "radio-van" => T(1.3),
        "newsstand" => T(1.7),
        "bike-rack" | "school-bike-rack" | "run-markers" | "nest-row" | "second-nests" => T(0.6),
        "city-bus" => T(1.9),
        "arcade-upstairs" => T(3.6),
        // ---- Icebridge --------------------------------------------------
        // A penguin's home stands twice a penguin: the igloos are homes,
        // not hutches.
        "snow-nest" => T(2.1),
        "snow-house" => T(2.3),
        "ice-store" | "snow-dome" => T(2.7),
        "kayak" => T(0.35),
        "snow-wall" => T(0.7),
        "deep-ledge" => W(3.0),
        "night-beacon" | "far-beacon" | "ice-beacon" => T(3.0),
        _ => return None,
    })
}

/// What stands in a garden or a field: never taller than a grown-up
/// (a skep is about 0.45 P). Held to it by a test.
pub const FIELD_PROPS: &[&str] = &[
    "beehives",
    "bee-garden",
    "vegetable-patch",
    "herb-pots",
    "herb-bed",
    "herb-garden",
    "flower-beds",
    "flower-bed",
    "flower-boxes",
    "window-boxes",
    "wildflowers",
    "wildflower-patch",
    "sunflowers",
    "school-garden",
    "quay-planters",
    "tomato-patch",
    "rose-bed",
    "garden-plots",
    "allotments",
    "sheepfold",
    "hen-house",
    "apple-press",
    "window-box-stand",
    "boat-planter",
    "fern-planter",
    "planter-racks",
    "grow-tray",
    "kelp-garden",
    "lichen-garden",
    "algae-beds",
    "red-moss",
    "pebble-planters",
    "pebble-garden",
    "moss-patch",
    "seaweed-bed",
    "median-beds",
];

/// The rung of one of the harbour's own shapes (no library drawing):
/// standing as a building, or as a thing.
pub fn of_plain(shape: MarkShape, building: bool) -> Rung {
    use MarkShape as M;
    use Rung::{Tall as T, Wide as W};
    if building {
        return match shape {
            M::House => T(2.75),
            M::Shop => T(3.0),
            M::Tower => T(5.0),
            M::Lamp => T(3.0),
            // The harbour's glasshouse and its domed hall.
            M::Dome | M::Tree => T(2.2),
            M::Bridge | M::Pier => W(4.5),
            // Anything else standing as a place is about a cottage.
            _ => T(2.75),
        };
    }
    match shape {
        M::Rover => T(1.0),
        // A working boat, its mast and all: its hull is about 0.5 P.
        M::Boat => T(1.2),
        M::Parcel => T(0.35),
        M::Stall => T(1.3),
        M::Bunting => T(2.2),
        M::Pier | M::Bridge => W(5.0),
        M::Garden => T(0.6),
        M::Flag => T(2.6),
        M::Lantern | M::Lamp => T(1.8),
        M::Tent => T(1.8),
        M::Bench => T(0.8),
        M::Sprouts => T(0.4),
        M::Well | M::Fountain => T(1.0),
        M::Swing => T(1.7),
        M::Signpost | M::Birdhouse => T(1.6),
        M::Planter => T(0.5),
        M::Statue => T(2.0),
        M::Postbox => T(0.75),
        M::House => T(2.75),
        M::Shop => T(3.0),
        M::Tower => T(5.0),
        M::Dome => T(2.8),
        M::Tree => T(3.5),
    }
}

/// The rung of a Pack's own drawing, by its id: the harbour's lighthouse,
/// pub, bakery and school, Pocket Universe's places, and the things any
/// town puts up for a festival (`{town}-flag`, `-bunting`, `-lantern`,
/// `-stall`, `-tent`). A drawing the ladder does not know keeps the size
/// its place gives it.
pub fn of_drawing(id: &str) -> Option<Rung> {
    use Rung::{Tall as T, Wide as W};
    let suffix = id.rsplit('-').next().unwrap_or(id);
    Some(match id {
        "lighthouse" => T(6.0),
        "pub" | "school" => T(3.3),
        "bakery" | "arcade" => T(3.0),
        "habitat" | "council" => T(2.6),
        "greenhouse" => T(2.4),
        "fish-vault" => T(2.6),
        "radio" => T(5.0),
        "icebridge" => W(5.0),
        _ => match suffix {
            "flag" => T(2.6),
            "bunting" => T(2.2),
            "lantern" => T(1.8),
            "stall" => T(1.3),
            "tent" => T(2.2),
            _ => return None,
        },
    })
}

/// How much of the box it is given a drawing paints: (across, up), as
/// shares of the box's width and height. A bench drawn in a box 1 wide
/// paints 0.8 across; a parcel's box is mostly air above it.
fn reach(subject: &Subject) -> (f32, f32) {
    static REACH: OnceLock<Mutex<HashMap<String, (f32, f32)>>> = OnceLock::new();
    let key = subject.key();
    if let Some(found) = REACH
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
    {
        return *found;
    }
    let (w, h) = subject.unit_box();
    let measured = measure(|brush, x, base| subject.paint(brush, x, base, w, h), w, h)
        .map(|(across, up)| (across / w, up / h))
        .unwrap_or((1.0, 1.0));
    REACH
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, measured);
    measured
}

/// What is being sized: a thing (drawn by its width), or a building
/// (drawn in a box), as the library, the harbour's shapes or the Pack draw
/// it.
pub enum Subject<'a> {
    Thing(MarkShape, Setting, Option<Art>),
    Building(MarkShape, Setting, Option<Art>),
    Drawing(&'a Drawing),
}

impl Subject<'_> {
    fn drawn(&self) -> Option<Drawn> {
        match self {
            Subject::Thing(shape, setting, art) | Subject::Building(shape, setting, art) => {
                Some(match art {
                    Some(art) => Drawn::Art(*art),
                    None => works::drawn(None, *shape, *setting),
                })
            }
            Subject::Drawing(_) => None,
        }
    }

    fn key(&self) -> String {
        // The library's keys and the plain shapes' names can coincide
        // ("harbour-lamp"), so each says which it is.
        let drawn = match self.drawn() {
            Some(Drawn::Art(art)) => format!("art:{}", art.key()),
            Some(Drawn::Plain(shape)) => format!("plain:{shape:?}"),
            None => String::new(),
        };
        match self {
            Subject::Thing(..) => format!("thing:{drawn}"),
            Subject::Building(shape, ..) => format!("building:{shape:?}:{drawn}"),
            Subject::Drawing(drawing) => format!("drawing:{}:{}", drawing.id, drawing.aspect),
        }
    }

    /// The rung it stands on.
    pub fn rung(&self) -> Option<Rung> {
        match self {
            Subject::Drawing(drawing) => of_drawing(&drawing.id),
            _ => Some(match self.drawn()? {
                Drawn::Art(art) => of_art(art.key())?,
                Drawn::Plain(shape) => of_plain(shape, self.boxed()),
            }),
        }
    }

    /// Whether it is drawn in a box of its own (a building), rather than
    /// by its width alone (a thing, even one standing as a place).
    fn boxed(&self) -> bool {
        use MarkShape as M;
        match self {
            Subject::Thing(..) => false,
            Subject::Building(..) => match self.drawn() {
                Some(Drawn::Plain(shape)) => matches!(
                    shape,
                    M::House | M::Shop | M::Tower | M::Dome | M::Tree | M::Lamp | M::Bridge
                ),
                _ => true,
            },
            Subject::Drawing(_) => true,
        }
    }

    /// The box it is measured in: 100 high, as wide as it is usually
    /// drawn.
    fn unit_box(&self) -> (f32, f32) {
        match self {
            Subject::Building(..) if self.boxed() => match self.drawn() {
                Some(Drawn::Art(art)) => (100.0 / art.tall(), 100.0),
                _ => (115.0, 100.0),
            },
            Subject::Thing(..) | Subject::Building(..) => (100.0, 100.0),
            Subject::Drawing(drawing) => (100.0 * drawing.aspect, 100.0),
        }
    }

    fn palette(&self) -> Palette {
        let (setting, art) = match self {
            Subject::Thing(_, setting, art) | Subject::Building(_, setting, art) => {
                (*setting, *art)
            }
            Subject::Drawing(_) => (Setting::Harbour, None),
        };
        let mut palette = Palette::of_in("ladder", false, setting);
        palette.art = art;
        palette
    }

    /// Paints it standing on (`x`, `base`) in the box `w` by `h` (a thing
    /// is drawn by its width alone).
    pub fn paint(&self, brush: &mut dyn Brush, x: f32, base: f32, w: f32, h: f32) {
        let palette = self.palette();
        match self {
            Subject::Thing(shape, ..) => art::paint_thing(brush, x, base, w, *shape, &palette, 0.0),
            Subject::Building(shape, ..) => {
                art::paint_building(brush, x, base, w, h, *shape, &palette)
            }
            Subject::Drawing(drawing) => art::paint_drawing(
                brush,
                x,
                base,
                w,
                h,
                drawing,
                &art::Inks::of_place(&palette),
                Stance::Standing,
                Mood::Content,
                0.0,
                0.0,
                1.0,
            ),
        }
    }

    /// The box to draw it in so that it stands on its rung at a depth
    /// where a grown-up is `p` tall: (w, h). A thing's `h` is unused. A
    /// building's box keeps `nominal_w` (its spot's width) when its own
    /// drawing does not fix its proportions. `None`: it has no rung, and
    /// keeps the size its place gives it.
    pub fn sized(&self, p: f32, nominal_w: f32) -> Option<(f32, f32)> {
        let rung = self.rung()?;
        let (across, up) = reach(self);
        let (bw, bh) = self.unit_box();
        // The box that paints `rung`, scaled from the unit box.
        let k = match rung {
            Rung::Tall(tall) => tall * p / (up * bh).max(1e-3),
            Rung::Wide(wide) => wide * p / (across * bw).max(1e-3),
        };
        Some(match self {
            Subject::Thing(..) => (bw * k, bh * k),
            Subject::Building(..) => match (self.drawn(), rung) {
                (Some(Drawn::Plain(_)), Rung::Tall(_)) if self.boxed() => (nominal_w, bh * k),
                _ => (bw * k, bh * k),
            },
            Subject::Drawing(_) => (bw * k, bh * k),
        })
    }
}

/// How much ground an item takes when it is drawn on the ladder at a
/// depth where a grown-up is `p` tall, in the same units as `p`: the room
/// the composition should keep for it. As a place (`building`) or a
/// thing; drawn as the Pack's own `drawing`, or as its library art or
/// shape in `setting`. `None`: it has no rung (a person, or a drawing the
/// ladder does not know), and keeps the room it is given.
pub fn footprint(
    item: &world_projection::CanvasItem,
    drawing: Option<&Drawing>,
    setting: Setting,
    p: f32,
    building: bool,
) -> Option<f32> {
    let art = item.art.as_deref().and_then(Art::from_key);
    let shape = item.shape.unwrap_or(if building {
        MarkShape::House
    } else {
        MarkShape::Parcel
    });
    let subject = match drawing {
        Some(drawing) => Subject::Drawing(drawing),
        None if building => Subject::Building(shape, setting, art),
        None => Subject::Thing(shape, setting, art),
    };
    let (w, _) = subject.sized(p, p * 2.2)?;
    let (across, _) = reach(&subject);
    // A thing's box is wider than what it paints; a building's is what
    // it paints.
    Some(if subject.boxed() { w } else { w * across })
}

/// What a drawing paints when it is painted by `paint` standing on the
/// foot of a canvas: how far across and how far up from its foot it
/// reaches, in the units it was painted in. Soft shadows and glows are
/// left out; they are light on the ground, not the thing.
pub fn measure(paint: impl FnOnce(&mut dyn Brush, f32, f32), w: f32, h: f32) -> Option<(f32, f32)> {
    let side = (w.max(h) * 3.0).ceil() as u32;
    let mut canvas = crate::painter::Canvas::new(side, side, 1.0, (0.0, 0.0))?;
    let (x, base) = (side as f32 / 2.0, side as f32 * 0.8);
    {
        let mut hard = Hard(&mut canvas);
        paint(&mut hard, x, base);
    }
    let data = canvas.pixmap.data();
    let (mut top, mut left, mut right) = (u32::MAX, u32::MAX, 0);
    for row in 0..(base.ceil() as u32).min(side) {
        for column in 0..side {
            if data[((row * side + column) * 4 + 3) as usize] > 60 {
                top = top.min(row);
                left = left.min(column);
                right = right.max(column);
            }
        }
    }
    (top != u32::MAX).then(|| ((right + 1 - left) as f32, base - top as f32))
}

/// A brush that paints everything but soft light.
struct Hard<'a>(&'a mut dyn Brush);

impl Brush for Hard<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        self.0.rect(x, y, w, h, radius, colour);
    }
    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        self.0.fill(shape, colour);
    }
    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        self.0.stroke(shape, width, colour);
    }
    fn soft(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Hsla) {}
    #[allow(clippy::too_many_arguments)]
    fn gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f32,
        from: (Hsla, f32),
        to: (Hsla, f32),
    ) {
        self.0.gradient(x, y, w, h, angle, from, to);
    }
    fn picture(&mut self, image: &Arc<RenderImage>, rect: Rect, clip: Rect) {
        self.0.picture(image, rect, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: f32 = 60.0;

    /// Paints `subject` sized for `P` and reads how tall it stands, in P.
    fn height_in_p(subject: &Subject) -> f32 {
        let (w, h) = subject.sized(P, P * 3.0).expect("a rung");
        let (_, up) = measure(|brush, x, base| subject.paint(brush, x, base, w, h), w, h)
            .expect("it paints something");
        up / P
    }

    fn width_in_p(subject: &Subject) -> f32 {
        let (w, h) = subject.sized(P, P * 3.0).expect("a rung");
        let (across, _) = measure(|brush, x, base| subject.paint(brush, x, base, w, h), w, h)
            .expect("it paints something");
        across / P
    }

    fn holds(subject: &Subject, name: &str) -> Option<String> {
        let rung = subject.rung()?;
        let (got, want) = match rung {
            Rung::Tall(want) => (height_in_p(subject), want),
            Rung::Wide(want) => (width_in_p(subject), want),
        };
        ((got - want).abs() > want * 0.1 + 0.05)
            .then(|| format!("{name}: {got:.2} P, its rung is {rung:?}"))
    }

    #[test]
    fn every_drawing_has_a_rung() {
        let missing = Art::all()
            .filter(|art| of_art(art.key()).is_none())
            .map(|art| art.key())
            .collect::<Vec<_>>();
        assert!(missing.is_empty(), "no rung for {missing:?}");
    }

    /// Reads every drawing's height in P as it is painted, as a thing and
    /// as a building, and holds it to its rung.
    #[test]
    fn every_drawing_stands_on_its_rung() {
        let mut wrong = Vec::new();
        for art in Art::all() {
            let setting = art.family().setting();
            for subject in [
                Subject::Thing(MarkShape::Parcel, setting, Some(art)),
                Subject::Building(MarkShape::House, setting, Some(art)),
            ] {
                wrong.extend(holds(&subject, art.key()));
            }
        }
        use MarkShape as M;
        // A lamp in the harbour is a place (a lit house), never a thing.
        for shape in [
            M::House,
            M::Shop,
            M::Tower,
            M::Dome,
            M::Tree,
            M::Lamp,
            M::Bridge,
            M::Rover,
            M::Boat,
            M::Parcel,
            M::Stall,
            M::Bunting,
            M::Pier,
            M::Garden,
            M::Flag,
            M::Lantern,
            M::Tent,
            M::Bench,
            M::Sprouts,
            M::Well,
            M::Swing,
            M::Fountain,
            M::Signpost,
            M::Birdhouse,
            M::Planter,
            M::Statue,
            M::Postbox,
        ] {
            for setting in Setting::ALL {
                if (shape, setting) == (M::Lamp, Setting::Harbour) {
                    continue;
                }
                wrong.extend(holds(
                    &Subject::Thing(shape, setting, None),
                    &format!("{shape:?} in {setting:?}"),
                ));
            }
            wrong.extend(holds(
                &Subject::Building(shape, Setting::Harbour, None),
                &format!("{shape:?} as a building"),
            ));
        }
        assert!(wrong.is_empty(), "off the ladder:\n{}", wrong.join("\n"));
    }

    /// The bible's own table, read off the paint.
    #[test]
    fn the_bibles_ladder_holds() {
        let thing = |key: &str| {
            let art = Art::from_key(key).unwrap();
            height_in_p(&Subject::Thing(
                MarkShape::Parcel,
                art.family().setting(),
                Some(art),
            ))
        };
        let building = |key: &str| {
            let art = Art::from_key(key).unwrap();
            height_in_p(&Subject::Building(
                MarkShape::House,
                art.family().setting(),
                Some(art),
            ))
        };
        let cottage = height_in_p(&Subject::Building(MarkShape::House, Setting::Harbour, None));
        assert!((cottage - 2.75).abs() < 0.3, "a cottage is {cottage:.2} P");
        for postbox in ["pillar-box", "lamp-box", "mailbox"] {
            let got = thing(postbox);
            assert!(got <= 0.8, "a postbox ({postbox}) is {got:.2} P");
        }
        let telescope = thing("telescope");
        assert!(
            telescope <= 0.6 * cottage,
            "a telescope is {telescope:.2} P to a cottage's {cottage:.2}"
        );
        for (key, want) in [
            ("park-bench", 0.8),
            ("wellhead", 1.0),
            ("market-stall", 1.3),
            ("lamp-post", 1.8),
            ("maypole", 3.5),
            ("rowing-boat", 0.5),
        ] {
            let got = thing(key);
            assert!(
                (got - want).abs() <= want * 0.12,
                "{key} is {got:.2} P, not {want}"
            );
        }
        for (key, want) in [("windmill", 5.0), ("lighthouse", 6.0), ("bookshop", 3.0)] {
            let got = building(key);
            assert!(
                (got - want).abs() <= want * 0.12,
                "{key} is {got:.2} P, not {want}"
            );
        }
    }
}
