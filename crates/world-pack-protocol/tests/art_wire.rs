//! The look a Pack declares crosses the boundary whole: a drawing's rung
//! on the art bible's ladder, and a cluster's ground, which must be one the
//! app paints.

use world_pack_protocol::{ClusterWire, DrawingWire, GroundWire, RungWire};
use world_projection::{Drawing, Ground, Rung};

#[test]
fn a_drawings_declared_rung_crosses_the_boundary() {
    let drawing = Drawing::new("hall", 1.1, Vec::new()).standing(Rung::Tall(3.3));
    let wire = DrawingWire::from(&drawing);
    assert_eq!(wire.rung, Some(RungWire::Tall(3.3)));
    let json = serde_json::to_string(&wire).unwrap();
    assert!(json.contains(r#""rung":{"tall":3.3}"#), "{json}");
    let back = Drawing::from(serde_json::from_str::<DrawingWire>(&json).unwrap());
    assert_eq!(back.rung, Some(Rung::Tall(3.3)));
    // An older Pack sends none; a rung nobody could stand by is dropped.
    let older =
        serde_json::from_str::<DrawingWire>(r#"{"id":"hall","aspect":1.0,"parts":[]}"#).unwrap();
    assert_eq!(Drawing::from(older).rung, None);
    let silly = serde_json::from_str::<DrawingWire>(
        r#"{"id":"hall","aspect":1.0,"parts":[],"rung":{"wide":1e9}}"#,
    )
    .unwrap();
    assert_eq!(Drawing::from(silly).rung, None);
}

#[test]
fn an_unknown_ground_fails_loudly() {
    let known = r#"{"id":"front","label":"the Front","from":0.1,"to":0.4,"ground":"cobbles","rows":[0.3,0.7]}"#;
    let cluster = serde_json::from_str::<ClusterWire>(known).unwrap();
    assert_eq!(cluster.ground, GroundWire(Ground::Cobbles));
    let unknown = known.replace("cobbles", "lava");
    let error = serde_json::from_str::<ClusterWire>(&unknown).unwrap_err();
    assert!(error.to_string().contains("lava"), "{error}");
    // Said nothing: plain worn ground, as before clusters had grounds.
    let silent = r#"{"id":"front","label":"the Front","from":0.1,"to":0.4}"#;
    assert_eq!(
        serde_json::from_str::<ClusterWire>(silent).unwrap().ground,
        GroundWire(Ground::Worn)
    );
    for ground in Ground::ALL {
        let json = serde_json::to_string(&GroundWire(ground)).unwrap();
        assert_eq!(serde_json::from_str::<GroundWire>(&json).unwrap().0, ground);
    }
}
