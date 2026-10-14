//! Scenes across the boundary: asked for with `story` like any page, of a
//! Pack that says it stages them, and told back whole; a cue an app cannot
//! stage is passed over, a kind it does not know is `other`, and what an
//! older Pack or app sends reads as it always did.

use serde_json::json;
use world_core::{EntityId, EventId};
use world_pack_protocol::{
    decode_request, decode_response, encode_request, encode_response, PackRequest,
    PackRequestEnvelope, PackResponse, PackResponseEnvelope, SceneScriptWire, StoryPageWire,
    StoryRequestWire, CAPABILITY_SCENES, PACK_PROTOCOL_VERSION, PACK_PROTOCOL_VERSION_V7,
};
use world_projection::{
    Act, AgeStage, Cue, Look, Mark, MomentKind, Prop, SceneScript, SelectionId, Stance, StoryPage,
    StoryRequest, MOST_SCENE_SECONDS,
};

fn someone(id: u64) -> SelectionId {
    SelectionId::Entity(EntityId::new(id))
}

fn cue(at: f32, who: Option<u64>, act: Act) -> Cue {
    Cue {
        at,
        who: who.map(someone),
        act,
    }
}

/// A farewell, with every act and every mark.
fn farewell() -> SceneScript {
    SceneScript {
        id: "scene-42".into(),
        event: EventId::new(42),
        kind: MomentKind::Farewell,
        place: Some(someone(100)),
        seconds: 8.0,
        cast: vec![
            (
                someone(1),
                Some(Look {
                    hair: Some(0x332211),
                    age: Some(AgeStage::Elder),
                    grey: true,
                    ..Look::default()
                }),
            ),
            (someone(2), None),
        ],
        cues: vec![
            cue(
                0.0,
                None,
                Act::Camera {
                    on: Mark::Place(someone(100)),
                    zoom: 1.2,
                },
            ),
            cue(0.0, None, Act::Weather(Prop::Rain)),
            cue(
                0.0,
                Some(1),
                Act::WalkTo {
                    to: Mark::Home(someone(1)),
                    stance: Stance::Carrying,
                },
            ),
            cue(
                0.0,
                None,
                Act::Prop {
                    prop: Prop::Ferry,
                    at: Mark::Offstage(true),
                    shown: true,
                },
            ),
            cue(
                0.0,
                Some(2),
                Act::WalkTo {
                    to: Mark::Beside(someone(100), -2),
                    stance: Stance::Standing,
                },
            ),
            cue(1.5, Some(2), Act::Strike(Stance::Waving)),
            cue(2.0, Some(1), Act::Say("Keep the lamp lit for me.".into())),
            cue(
                3.5,
                Some(1),
                Act::WalkTo {
                    to: Mark::Offstage(true),
                    stance: Stance::Walking,
                },
            ),
        ],
        caption: "Ada left on the morning ferry".into(),
    }
}

#[test]
fn a_scene_is_asked_for_and_told_back_whole() {
    let request = StoryRequest::Scene(EventId::new(42));
    let ask = PackRequest::Story {
        request: StoryRequestWire::from(&request),
    };
    let envelope = PackRequestEnvelope::for_version(PACK_PROTOCOL_VERSION, 1, ask).unwrap();
    let decoded = decode_request(&encode_request(&envelope).unwrap()).unwrap();
    let PackRequest::Story { request: asked } = decoded.request else {
        panic!("not a story");
    };
    assert_eq!(StoryRequest::from(asked), request);

    let page = StoryPage::Scene(farewell());
    let response = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION,
        1,
        PackResponse::Story {
            page: Some(StoryPageWire::from(&page)),
        },
    )
    .unwrap();
    let decoded = decode_response(&encode_response(&response).unwrap()).unwrap();
    let PackResponse::Story { page: Some(told) } = decoded.response else {
        panic!("no page");
    };
    assert_eq!(told.into_page(), Some(page));
    assert_eq!(CAPABILITY_SCENES, "scenes");
}

#[test]
fn every_kind_crosses_and_an_unknown_one_is_other() {
    for kind in MomentKind::ALL {
        let scene = SceneScript { kind, ..farewell() };
        let wire = SceneScriptWire::from(&scene);
        let json = serde_json::to_string(&wire).unwrap();
        let back: SceneScriptWire = serde_json::from_str(&json).unwrap();
        assert_eq!(back.into_scene(), Some(scene));
    }
    let mut odd = SceneScriptWire::from(&farewell());
    odd.kind = "coronation".into();
    assert_eq!(odd.into_scene().unwrap().kind, MomentKind::Other);
}

#[test]
fn a_cue_an_app_cannot_stage_is_passed_over() {
    let mut wire = serde_json::to_value(SceneScriptWire::from(&farewell())).unwrap();
    let cues = wire["cues"].as_array_mut().unwrap();
    // From a newer Pack: an act, a mark, a stance and a prop this build
    // does not know, a cue past the end, and one moving a stranger.
    cues.push(json!({"at": 1.0, "act": {"type": "juggle", "balls": 3}}));
    cues.push(json!({"at": 1.0, "act": {"type": "camera", "on": {"type": "moon"}}}));
    cues.push(json!({"at": 1.0, "who": {"type": "entity", "id": 1},
        "act": {"type": "strike", "stance": "cartwheeling"}}));
    cues.push(json!({"at": 1.0, "act": {"type": "weather", "prop": "aurora"}}));
    cues.push(json!({"at": 99.0, "act": {"type": "say", "words": "Too late"}}));
    cues.push(json!({"at": 1.0, "who": {"type": "entity", "id": 77},
        "act": {"type": "say", "words": "Who am I?"}}));
    // A walk in a stance this build does not know is walked standing.
    cues.push(json!({"at": 0.5, "who": {"type": "entity", "id": 2},
        "act": {"type": "walk_to", "to": {"type": "offstage"}, "stance": "moonwalking"}}));
    let wire: SceneScriptWire = serde_json::from_value(wire).unwrap();
    let staged = wire.into_scene().unwrap();
    let mut expected = farewell();
    expected.cues.insert(
        5,
        cue(
            0.5,
            Some(2),
            Act::WalkTo {
                to: Mark::Offstage(false),
                stance: Stance::Standing,
            },
        ),
    );
    assert_eq!(staged, expected);

    // No time to play in, or no id, and there is no scene; too long a one
    // plays for the longest a scene may.
    let mut endless = SceneScriptWire::from(&farewell());
    endless.seconds = 600.0;
    assert_eq!(endless.into_scene().unwrap().seconds, MOST_SCENE_SECONDS);
    for seconds in [0.0, -1.0, f32::NAN] {
        let mut wire = SceneScriptWire::from(&farewell());
        wire.seconds = seconds;
        assert!(wire.into_scene().is_none());
    }
    let mut nameless = SceneScriptWire::from(&farewell());
    nameless.id = " ".into();
    assert!(nameless.into_scene().is_none());
}

/// What an older Pack sends still reads: a story page from before scenes,
/// and a request for any other story; and a page of a kind this build
/// does not know is no page.
#[test]
fn what_came_before_scenes_still_reads() {
    let older = json!({"type": "almanac", "almanac": {"year": 2}});
    let page: StoryPageWire = serde_json::from_value(older).unwrap();
    assert!(matches!(page.into_page(), Some(StoryPage::Almanac(_))));
    let older = json!({"type": "moment", "id": "moment-7"});
    let request: StoryRequestWire = serde_json::from_value(older).unwrap();
    assert_eq!(
        StoryRequest::from(request),
        StoryRequest::Moment("moment-7".into())
    );
    let newer = json!({"type": "ballad", "verses": []});
    let page: StoryPageWire = serde_json::from_value(newer).unwrap();
    assert_eq!(page.into_page(), None);
    // A v7 response without a page is still none.
    let none = PackResponseEnvelope::for_version(
        PACK_PROTOCOL_VERSION_V7,
        1,
        PackResponse::Story { page: None },
    )
    .unwrap();
    let decoded = decode_response(&encode_response(&none).unwrap()).unwrap();
    assert_eq!(decoded.response, PackResponse::Story { page: None });
    // A scene with only what it must have is staged, with nobody in it.
    let bare = json!({"id": "scene-3", "event": 3, "seconds": 6.0});
    let bare: SceneScriptWire = serde_json::from_value(bare).unwrap();
    let bare = bare.into_scene().unwrap();
    assert_eq!(bare.kind, MomentKind::Other);
    assert!(bare.cast.is_empty() && bare.cues.is_empty());
}
