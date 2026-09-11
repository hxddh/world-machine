//! Where everything goes when a World is drawn as the place it is.
//!
//! The desktop window can only be built and run on macOS, so any arithmetic it
//! does is arithmetic nobody can check until CI gets round to it. The geometry
//! of the scene therefore lives here, where it is ordinary Rust with ordinary
//! tests, and the drawing code is left with nothing to decide except what
//! colour things are.
//!
//! The shape of the picture is a waterfront: a strip of sky, a row of
//! buildings standing on a quay, people on the ground in front of them, and
//! water along the foot with a jetty running out over it.

use crate::{CanvasItem, CanvasItemKind, CanvasItemState, SelectionId};

/// A rectangle in scene coordinates, with the origin at the top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn centre_x(&self) -> f32 {
        self.x + self.width / 2.0
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

/// A building on the quay.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingShape {
    pub id: SelectionId,
    pub label: String,
    pub state: CanvasItemState,
    /// The walls. The roof sits on top of this and the sign across its head.
    pub body: Rect,
    pub roof_peak: (f32, f32),
    pub sign: Rect,
    pub door: Rect,
    pub windows: Vec<Rect>,
}

/// Somebody standing in the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct FigureSpot {
    pub id: SelectionId,
    pub label: String,
    pub state: CanvasItemState,
    /// Where their feet are.
    pub feet: (f32, f32),
    /// The colour hue this entity is drawn in, everywhere in the app.
    pub hue: f32,
}

/// A thing that is neither a building nor a person: a boat at a mooring, an
/// order on a counter.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectSpot {
    pub id: SelectionId,
    pub label: String,
    pub state: CanvasItemState,
    pub at: (f32, f32),
    /// Whether this thing sits on the water rather than on the quay.
    ///
    /// Two things a Pack calls Objects can be as unlike as a boat and a sack
    /// of flour, and which is which is not something a drawing should guess
    /// from the name. What the World does say is where each one is. Only a
    /// thing at the waterside place is drawn afloat.
    pub afloat: bool,
}

/// The whole picture, ready to be coloured in.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenePlan {
    pub width: f32,
    pub height: f32,
    /// Where the quay's surface is: buildings stand on this line.
    pub ground_y: f32,
    /// Where the quay meets the water.
    pub water_y: f32,
    /// The jetty decking, when the scene has a place out over the water.
    pub jetty: Option<Rect>,
    pub buildings: Vec<BuildingShape>,
    pub folk: Vec<FigureSpot>,
    pub objects: Vec<ObjectSpot>,
}

const SKY: f32 = 0.10;
const GROUND: f32 = 0.62;
const WATER: f32 = 0.80;
const GAP: f32 = 18.0;
const SIGN_H: f32 = 22.0;
const DOOR_W: f32 = 30.0;
const DOOR_H: f32 = 44.0;
/// Below this much wall a building drops its windows rather than cram them.
const WALL_FOR_WINDOWS: f32 = 62.0;
const FIGURE_SPREAD: f32 = 40.0;
/// How close a crowd will stand before the picture would rather clip than
/// squash them further.
const FIGURE_MIN_SPREAD: f32 = 20.0;
/// The margin a figure keeps from the edge of the frame.
const EDGE: f32 = 34.0;
const FIGURE_HEIGHT: f32 = 40.0;

/// A stable colour seed for one entity, used to decide where a cast starts on
/// the wheel. It is deterministic, so the same World always comes out the same
/// colours, but it is NOT how a figure's hue is chosen: telling eight people
/// apart is a property of the eight together, which no per-id hash can promise
/// — hash them into 360 slots and two of them collide soon enough. `plan`
/// spreads the cast evenly instead, and uses this only to rotate the wheel.
pub fn hue_for(id: SelectionId) -> f32 {
    // The id's own stable key, so the colour survives everything that key
    // survives: the wire, an archive, a fork.
    let seed = id.stable_key().bytes().fold(0_u64, |acc, byte| {
        acc.wrapping_mul(131).wrapping_add(byte as u64)
    });
    // Stir before taking the hue. The rolling hash alone maps consecutive ids
    // to consecutive numbers, so a town of eight residents came out as eight
    // shades of one green, 1° apart. This is the SplitMix64 finalizer: it
    // costs nothing and it scatters neighbours across the wheel.
    let mut mixed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;
    (mixed % 360) as f32
}

/// Lay a World out as a waterfront.
///
/// Places are taken in the order the Pack put them along the canvas's x axis,
/// which is how a Pack says what order its street is in. The last place is
/// treated as the harbour and given the jetty, because the water is on that
/// side; a World whose places are all inland simply has a jetty nobody stands
/// on, which is why it is optional.
pub fn plan(items: &[CanvasItem], width: f32, height: f32) -> ScenePlan {
    let ground_y = height * GROUND;
    let water_y = height * WATER;
    let sky_y = height * SKY;

    let mut places = items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Place)
        .collect::<Vec<_>>();
    places.sort_by(|a, b| a.x.total_cmp(&b.x));

    let jetty = places.last().map(|_| Rect {
        x: width * 0.70,
        y: ground_y + 6.0,
        width: width * 0.32,
        height: 13.0,
    });
    // The buildings share the quay to the left of the jetty.
    let street = jetty.map(|deck| deck.x - GAP).unwrap_or(width);
    let housed = places
        .len()
        .saturating_sub(if jetty.is_some() { 1 } else { 0 });

    let mut buildings = Vec::new();
    if housed > 0 {
        let each = (street - GAP * (housed as f32 + 1.0)) / housed as f32;
        for (index, place) in places.iter().take(housed).enumerate() {
            let body_w = each.clamp(90.0, 190.0);
            let x = GAP + index as f32 * (each + GAP) + (each - body_w) / 2.0;
            // Taller buildings in the middle of the row would look staged, so
            // height comes from the place's own y: a Pack that gives them all
            // the same y gets a level street, which is the honest default.
            let body_h = (ground_y - sky_y) * (0.62 + 0.18 * (1.0 - place.y.clamp(0.0, 1.0)));
            let body = Rect {
                x,
                y: ground_y - body_h,
                width: body_w,
                height: body_h,
            };
            // A wall is whatever height the scene can spare, so what is
            // painted on it is a fraction of the wall rather than a fixed
            // number of pixels. In pixels the parts collided as soon as the
            // scene was short: a 44px door on a 48px wall climbed into the
            // sign, and clamping the door alone only turned it into a third
            // window sitting between the other two.
            let sign_h = (body_h * 0.28).clamp(10.0, SIGN_H);
            let gap = (body_h * 0.08).clamp(3.0, 10.0);
            let sign = Rect {
                x: body.x + 6.0,
                y: body.y + gap * 0.7,
                width: body.width - 12.0,
                height: sign_h,
            };

            // Windows are the first thing a short wall gives up. A sign and a
            // door still read as a shopfront; a sign, a door and two shutters
            // crammed into 48 pixels read as nothing at all.
            let window_h = body_h * 0.22;
            let windows = if body_h >= WALL_FOR_WINDOWS {
                let window_count = if body_w >= 150.0 { 3 } else { 2 };
                let ww = 30.0;
                let wgap = (body_w - window_count as f32 * ww) / (window_count as f32 + 1.0);
                (0..window_count)
                    .map(|i| Rect {
                        x: body.x + wgap + i as f32 * (ww + wgap),
                        y: sign.bottom() + gap,
                        width: ww,
                        height: window_h,
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            // The door reaches the street, and starts below whatever is above
            // it. It gives way rather than overpainting: a shop with a stub of
            // a door still reads as a shop; one whose name is covered does not.
            let above = windows
                .last()
                .map(|window| window.bottom())
                .unwrap_or_else(|| sign.bottom());
            let door_top = (ground_y - DOOR_H).max(above + gap);
            let door = Rect {
                x: body.centre_x() - DOOR_W / 2.0,
                y: door_top,
                width: DOOR_W,
                height: (ground_y - door_top).max(2.0),
            };
            buildings.push(BuildingShape {
                id: place.id,
                label: place.label.clone(),
                state: place.state,
                roof_peak: (body.centre_x(), body.y - 30.0),
                body,
                sign,
                door,
                windows,
            });
        }
    }

    let anchor = |at: Option<SelectionId>| -> (f32, f32) {
        if let Some(place) = at {
            if let Some(building) = buildings.iter().find(|b| b.id == place) {
                return (building.body.centre_x(), ground_y + FIGURE_HEIGHT);
            }
            if let Some(deck) = jetty {
                if places.last().is_some_and(|last| last.id == place) {
                    return (deck.x + deck.width * 0.45, deck.y);
                }
            }
        }
        (width / 2.0, ground_y + FIGURE_HEIGHT)
    };

    let mut standing: Vec<(Option<SelectionId>, usize)> = Vec::new();
    let mut folk = Vec::new();
    for actor in items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
    {
        let slot = match standing.iter_mut().find(|(at, _)| *at == actor.at) {
            Some((_, count)) => {
                *count += 1;
                *count - 1
            }
            None => {
                standing.push((actor.at, 1));
                0
            }
        };
        let crowd = items
            .iter()
            .filter(|item| item.kind == CanvasItemKind::Actor && item.at == actor.at)
            .count();
        let (cx, cy) = anchor(actor.at);
        // A crowd closes ranks rather than running off the edge. Once
        // everybody in the town works nowhere they all stand on the quay
        // together, and at full spacing the far end of that line was outside
        // the frame — eight people around the harbour reached x=1068 in a
        // window 1000 wide.
        let spread = {
            let room = (width - 2.0 * EDGE) / crowd.max(1) as f32;
            FIGURE_SPREAD.min(room.max(FIGURE_MIN_SPREAD))
        };
        let line = (crowd as f32 - 1.0) * spread;
        let start = (cx - line / 2.0)
            .max(EDGE)
            .min((width - EDGE - line).max(EDGE));
        folk.push(FigureSpot {
            id: actor.id,
            label: actor.label.clone(),
            state: actor.state,
            feet: (start + slot as f32 * spread, cy),
            hue: 0.0, // assigned below, once the whole cast is known
        });
    }

    let objects = items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Object)
        .map(|object| {
            let afloat = jetty.is_some()
                && object.at.is_some()
                && places.last().is_some_and(|last| Some(last.id) == object.at);
            let at = match jetty {
                Some(deck) if afloat => (deck.right() - 90.0, water_y + 24.0),
                _ => {
                    // On the quay, beside the people standing in front of the
                    // place rather than on top of them: centred on the crowd,
                    // its name landed on top of theirs. Clamped clear of the
                    // water too — without that an inland thing was drawn
                    // below the waterline, floating.
                    let (cx, _) = anchor(object.at);
                    let crowd = items
                        .iter()
                        .filter(|item| item.kind == CanvasItemKind::Actor && item.at == object.at)
                        .count();
                    let beside = (crowd as f32 * FIGURE_SPREAD) / 2.0 + 34.0;
                    let feet = ground_y + FIGURE_HEIGHT;
                    (
                        (cx + beside).min(width - 62.0),
                        (feet + 18.0).min(water_y - 14.0),
                    )
                }
            };
            ObjectSpot {
                id: object.id,
                label: object.label.clone(),
                state: object.state,
                at,
                afloat,
            }
        })
        .collect();

    // Spread the cast evenly round the wheel, in a deterministic order, so no
    // two residents in one picture land on the same colour. Rotated by the
    // cast's own seed so different Worlds do not all open on the same red.
    let mut order: Vec<SelectionId> = folk.iter().map(|spot| spot.id).collect();
    order.sort_by_key(|id| id.stable_key());
    let rotation = order.first().copied().map(hue_for).unwrap_or(0.0);
    let step = if order.is_empty() {
        0.0
    } else {
        360.0 / order.len() as f32
    };
    for spot in &mut folk {
        let rank = order
            .iter()
            .position(|id| *id == spot.id)
            .expect("every figure is in the cast");
        spot.hue = (rotation + rank as f32 * step) % 360.0;
    }

    ScenePlan {
        width,
        height,
        ground_y,
        water_y,
        jetty,
        buildings,
        folk,
        objects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::EntityId;

    fn place(id: u64, label: &str, x: f32, state: CanvasItemState) -> CanvasItem {
        CanvasItem {
            id: SelectionId::Entity(EntityId::new(id)),
            kind: CanvasItemKind::Place,
            label: label.into(),
            detail: String::new(),
            x,
            y: 0.42,
            at: None,
            state,
        }
    }

    fn actor(id: u64, label: &str, at: Option<u64>) -> CanvasItem {
        CanvasItem {
            id: SelectionId::Entity(EntityId::new(id)),
            kind: CanvasItemKind::Actor,
            label: label.into(),
            detail: String::new(),
            x: 0.5,
            y: 0.68,
            at: at.map(|id| SelectionId::Entity(EntityId::new(id))),
            state: CanvasItemState::Working,
        }
    }

    fn town() -> Vec<CanvasItem> {
        vec![
            place(101, "First", 0.14, CanvasItemState::Working),
            place(102, "Second", 0.38, CanvasItemState::Stopped),
            place(103, "Third", 0.62, CanvasItemState::Working),
            place(104, "Water", 0.88, CanvasItemState::Working),
            actor(1, "Ann", Some(101)),
            actor(2, "Bo", Some(102)),
            actor(3, "Cai", Some(102)),
            actor(4, "Dee", Some(104)),
            actor(5, "Eve", Some(104)),
            CanvasItem {
                id: SelectionId::Entity(EntityId::new(201)),
                kind: CanvasItemKind::Object,
                label: "Boat".into(),
                detail: String::new(),
                x: 0.94,
                y: 0.86,
                at: Some(SelectionId::Entity(EntityId::new(104))),
                state: CanvasItemState::Gone,
            },
        ]
    }

    #[test]
    fn no_two_buildings_stand_in_the_same_place() {
        let plan = plan(&town(), 1100.0, 430.0);
        for (i, one) in plan.buildings.iter().enumerate() {
            for other in plan.buildings.iter().skip(i + 1) {
                assert!(
                    !one.body.overlaps(&other.body),
                    "{} and {} overlap: {:?} {:?}",
                    one.label,
                    other.label,
                    one.body,
                    other.body
                );
            }
        }
    }

    /// The window is not always tall. It was checked at one generous height,
    /// and the one thing that breaks when it is not generous — the door
    /// climbing into the sign — went out in a screenshot.
    #[test]
    fn a_name_is_never_painted_over_however_short_the_scene() {
        for height in [120.0_f32, 150.0, 180.0, 220.0, 300.0, 430.0] {
            let plan = plan(&town(), 1100.0, height);
            for building in &plan.buildings {
                assert!(
                    building.door.y >= building.sign.bottom(),
                    "at {height}px, {}'s door is drawn over its name: door {:?}, sign {:?}",
                    building.label,
                    building.door,
                    building.sign
                );
                assert!(
                    building.door.height > 0.0 && building.door.bottom() <= body_floor(building),
                    "at {height}px, {}'s door is not a door: {:?}",
                    building.label,
                    building.door
                );
                // Sign, then the row of windows if there is one, then the
                // door: three bands down the wall. The windows are a row, not
                // a stack, so they share a band.
                let mut floor = building.sign.bottom();
                for window in &building.windows {
                    assert!(
                        window.y >= floor,
                        "at {height}px, {}'s name is painted over its windows",
                        building.label
                    );
                }
                if let Some(lowest) = building
                    .windows
                    .iter()
                    .map(|window| window.bottom())
                    .max_by(f32::total_cmp)
                {
                    floor = lowest;
                }
                assert!(
                    building.door.y >= floor,
                    "at {height}px, {}'s door is above what is painted over it",
                    building.label
                );
                assert!(
                    building.sign.y >= building.body.y
                        && building.door.bottom() <= body_floor(building),
                    "at {height}px, {} paints outside its wall",
                    building.label
                );
            }
        }
    }

    fn body_floor(building: &BuildingShape) -> f32 {
        building.body.bottom() + 0.01
    }

    #[test]
    fn a_building_keeps_its_own_parts_inside_itself() {
        let plan = plan(&town(), 1100.0, 430.0);
        for building in &plan.buildings {
            for (name, part) in [("sign", building.sign), ("door", building.door)]
                .into_iter()
                .chain(building.windows.iter().map(|window| ("window", *window)))
            {
                assert!(
                    part.x >= building.body.x
                        && part.right() <= building.body.right()
                        && part.y >= building.body.y
                        && part.bottom() <= building.body.bottom() + 0.01,
                    "{} of {} escapes the wall: {:?} vs {:?}",
                    name,
                    building.label,
                    part,
                    building.body
                );
            }
            if let Some(first) = building.windows.first() {
                assert!(
                    building.sign.bottom() <= first.y,
                    "{}'s name is painted over its windows",
                    building.label
                );
                assert!(
                    building.windows.last().unwrap().bottom() <= building.door.y,
                    "{}'s windows sit on its door",
                    building.label
                );
            }
        }
    }

    #[test]
    fn everybody_stands_at_the_place_they_are_at() {
        let plan = plan(&town(), 1100.0, 430.0);
        let second = plan
            .buildings
            .iter()
            .find(|b| b.label == "Second")
            .expect("the second building is drawn");
        for name in ["Bo", "Cai"] {
            let spot = plan.folk.iter().find(|f| f.label == name).unwrap();
            assert!(
                (spot.feet.0 - second.body.centre_x()).abs() <= FIGURE_SPREAD,
                "{name} stands at Second, not at {:.0}",
                spot.feet.0
            );
        }
        let deck = plan.jetty.expect("the last place is out over the water");
        for name in ["Dee", "Eve"] {
            let spot = plan.folk.iter().find(|f| f.label == name).unwrap();
            assert!(
                spot.feet.0 >= deck.x && spot.feet.0 <= deck.right(),
                "{name} stands on the jetty, not at {:.0}",
                spot.feet.0
            );
            assert_eq!(spot.feet.1, deck.y, "and on its decking");
        }
    }

    #[test]
    fn two_people_at_one_place_do_not_stand_on_each_other() {
        let plan = plan(&town(), 1100.0, 430.0);
        let bo = plan.folk.iter().find(|f| f.label == "Bo").unwrap();
        let cai = plan.folk.iter().find(|f| f.label == "Cai").unwrap();
        assert!(
            (bo.feet.0 - cai.feet.0).abs() >= FIGURE_SPREAD - 0.01,
            "Bo at {:.0} and Cai at {:.0} are the same person",
            bo.feet.0,
            cai.feet.0
        );

        // And the pair of them stand in front of their building rather than
        // starting at its middle and trailing off to the right.
        let second = plan.buildings.iter().find(|b| b.label == "Second").unwrap();
        let midpoint = (bo.feet.0 + cai.feet.0) / 2.0;
        assert!(
            (midpoint - second.body.centre_x()).abs() < 0.01,
            "the crowd's middle is at {midpoint:.1}, the building's at {:.1}",
            second.body.centre_x()
        );
    }

    #[test]
    fn the_whole_picture_fits_in_its_frame() {
        let plan = plan(&town(), 1100.0, 430.0);
        for building in &plan.buildings {
            assert!(building.body.x >= 0.0 && building.body.right() <= 1100.0);
            assert!(building.roof_peak.1 >= 0.0, "a roof pokes out of the sky");
        }
        for spot in &plan.folk {
            assert!(spot.feet.0 >= 0.0 && spot.feet.0 <= 1100.0, "{:?}", spot);
            assert!(spot.feet.1 <= 430.0, "{:?} stands below the frame", spot);
        }
        for object in &plan.objects {
            assert!(object.at.0 >= 0.0 && object.at.0 <= 1100.0, "{:?}", object);
        }
    }

    #[test]
    fn a_thing_that_is_gone_still_gets_somewhere_to_be_missing_from() {
        let plan = plan(&town(), 1100.0, 430.0);
        let boat = plan
            .objects
            .iter()
            .find(|o| o.label == "Boat")
            .expect("the boat is still part of the picture");
        assert_eq!(boat.state, CanvasItemState::Gone);
        let deck = plan.jetty.unwrap();
        assert!(boat.at.0 >= deck.x, "her mooring is at the jetty");
        assert!(boat.at.1 > plan.water_y, "and it is in the water");
    }

    #[test]
    fn the_same_entity_is_the_same_colour_every_time() {
        let one = hue_for(SelectionId::Entity(EntityId::new(3)));
        assert_eq!(one, hue_for(SelectionId::Entity(EntityId::new(3))));
        assert_ne!(one, hue_for(SelectionId::Entity(EntityId::new(4))));
        assert!((0.0..360.0).contains(&one));
    }

    #[test]
    fn a_world_with_no_places_draws_nothing_rather_than_panicking() {
        let plan = plan(&[], 1100.0, 430.0);
        assert!(plan.buildings.is_empty());
        assert!(plan.jetty.is_none());
        assert!(plan.folk.is_empty());
    }
}

#[cfg(test)]
mod object_shape_tests {
    use super::*;
    use crate::{CanvasItem, CanvasItemKind, CanvasItemState};

    fn place(id: u64, label: &str) -> CanvasItem {
        CanvasItem {
            id: SelectionId::Entity(crate::EntityId(id)),
            kind: CanvasItemKind::Place,
            label: label.into(),
            detail: String::new(),
            x: 0.0,
            y: 0.0,
            at: None,
            state: CanvasItemState::Working,
        }
    }

    fn thing(id: u64, label: &str, at: u64) -> CanvasItem {
        CanvasItem {
            id: SelectionId::Entity(crate::EntityId(id)),
            kind: CanvasItemKind::Object,
            label: label.into(),
            detail: String::new(),
            x: 0.0,
            y: 0.0,
            at: Some(SelectionId::Entity(crate::EntityId(at))),
            state: CanvasItemState::Working,
        }
    }

    #[test]
    fn a_thing_on_the_quay_does_not_stand_on_the_people() {
        // Its name landed on top of theirs when it shared their centre.
        let items = vec![
            place(1, "Shop"),
            place(2, "Water"),
            CanvasItem {
                id: SelectionId::Entity(crate::EntityId::new(20)),
                kind: CanvasItemKind::Actor,
                label: "Ann".into(),
                detail: String::new(),
                x: 0.0,
                y: 0.0,
                at: Some(SelectionId::Entity(crate::EntityId::new(1))),
                state: CanvasItemState::Working,
            },
            thing(10, "Sack", 1),
        ];
        let plan = plan(&items, 1100.0, 300.0);
        let who = &plan.folk[0];
        let what = &plan.objects[0];
        let apart = (who.feet.0 - what.at.0).abs();
        assert!(
            apart >= 30.0,
            "the sack is {apart:.0}px from Ann, so their names overlap"
        );
    }

    #[test]
    fn only_a_thing_at_the_harbour_is_afloat() {
        // Both are Objects. Drawing every Object as a boat put a mast and a
        // sail on a thing sitting indoors, which is what this distinguishes.
        let items = vec![
            place(1, "Shop"),
            place(2, "Water"),
            thing(10, "Sack", 1),
            thing(11, "Boat", 2),
        ];
        let plan = plan(&items, 1100.0, 300.0);

        let order = plan
            .objects
            .iter()
            .find(|object| object.label == "Sack")
            .expect("the sack is in the picture");
        let boat = plan
            .objects
            .iter()
            .find(|object| object.label == "Boat")
            .expect("the boat is in the picture");

        assert!(!order.afloat, "a thing indoors is not on the water");
        assert!(boat.afloat, "a thing at the waterside place floats");
        assert!(
            order.at.1 < plan.water_y,
            "it sits on the quay, above the waterline at {:.0}, not at {:.0}",
            plan.water_y,
            order.at.1
        );
        assert!(
            boat.at.1 > plan.water_y,
            "the boat floats below the waterline at {:.0}, not at {:.0}",
            plan.water_y,
            boat.at.1
        );
    }
}

#[cfg(test)]
mod hue_spread_tests {
    use super::*;
    use crate::{CanvasItem, CanvasItemKind, CanvasItemState, EntityId};

    fn place(id: u64, label: &str) -> CanvasItem {
        CanvasItem {
            id: SelectionId::Entity(EntityId::new(id)),
            kind: CanvasItemKind::Place,
            label: label.into(),
            detail: String::new(),
            x: 0.0,
            y: 0.0,
            at: None,
            state: CanvasItemState::Working,
        }
    }

    #[test]
    fn neighbouring_entities_are_told_apart_by_colour() {
        // The first drawing of a real World came out with every resident in
        // the same green: consecutive entity ids differ by one byte, and the
        // rolling hash moved the result by about as much, so eight people
        // landed within 8° of each other. Hashing harder only trades that for
        // collisions. Telling a cast apart is the scene's job, and this is
        // the test of it.
        let places = vec![place(1, "Shop")];
        let cast: Vec<CanvasItem> = (10..18)
            .map(|id| CanvasItem {
                id: SelectionId::Entity(EntityId::new(id)),
                kind: CanvasItemKind::Actor,
                label: format!("Person {id}"),
                detail: String::new(),
                x: 0.0,
                y: 0.0,
                at: Some(SelectionId::Entity(EntityId::new(1))),
                state: CanvasItemState::Working,
            })
            .collect();
        let items: Vec<CanvasItem> = places.into_iter().chain(cast).collect();
        let hues: Vec<f32> = plan(&items, 1100.0, 300.0)
            .folk
            .iter()
            .map(|spot| spot.hue)
            .collect();
        for (i, a) in hues.iter().enumerate() {
            for b in hues.iter().skip(i + 1) {
                let apart = (a - b).abs().min(360.0 - (a - b).abs());
                assert!(
                    apart >= 25.0,
                    "two of the eight residents are {apart:.0}° apart, which reads as one colour: {hues:?}"
                );
            }
        }
    }
}

#[cfg(test)]
mod crowd_tests {
    use super::*;
    use crate::{CanvasItem, CanvasItemKind, CanvasItemState, EntityId};

    /// A town where everybody has ended up in the same place still fits.
    ///
    /// Once whereabouts follow work, the end of Harbour Town is all eight
    /// residents standing on the quay together. At full spacing the far end
    /// of that line was at x=1068 in a window 1000 wide: two people simply
    /// off the edge of the picture.
    ///
    /// Eighteen of them, because eight is not the interesting case — a line
    /// of eight fits any of these frames once it is kept clear of the edges,
    /// and a Pack with a real crowd in one place is what makes the spacing
    /// have to give.
    #[test]
    fn a_whole_town_in_one_place_still_fits_the_frame() {
        let mut items = vec![CanvasItem {
            id: SelectionId::Entity(EntityId::new(1)),
            kind: CanvasItemKind::Place,
            label: "Water".into(),
            detail: String::new(),
            x: 0.0,
            y: 0.0,
            at: None,
            state: CanvasItemState::Working,
        }];
        for id in 10..28 {
            items.push(CanvasItem {
                id: SelectionId::Entity(EntityId::new(id)),
                kind: CanvasItemKind::Actor,
                label: format!("Person {id}"),
                detail: String::new(),
                x: 0.0,
                y: 0.0,
                at: Some(SelectionId::Entity(EntityId::new(1))),
                state: CanvasItemState::Working,
            });
        }

        for width in [420.0_f32, 700.0, 1000.0, 1100.0] {
            let plan = plan(&items, width, 300.0);
            for spot in &plan.folk {
                assert!(
                    spot.feet.0 >= 0.0 && spot.feet.0 <= width,
                    "{} stands at {:.0} in a picture {width:.0} wide",
                    spot.label,
                    spot.feet.0
                );
            }
            let mut xs: Vec<f32> = plan.folk.iter().map(|spot| spot.feet.0).collect();
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.windows(2) {
                assert!(
                    pair[1] - pair[0] >= FIGURE_MIN_SPREAD - 0.01,
                    "two of them stand {:.0} apart, which is on top of each other",
                    pair[1] - pair[0]
                );
            }
        }
    }
}
