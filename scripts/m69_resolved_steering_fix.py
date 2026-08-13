from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


lib_path = ROOT / "worlds/pocket-universe/src/lib.rs"
text = lib_path.read_text()
text = replace_once(
    text,
    '''    if integer_component(state, UNIVERSE, GENERATION)? < 2 {
        return Err(ActionError::Invalid(
            "the relationship has not developed enough to steer yet".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)? != "none" {
''',
    '''    if integer_component(state, UNIVERSE, GENERATION)? < 2 {
        return Err(ActionError::Invalid(
            "the relationship has not developed enough to steer yet".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)? != "forming" {
        return Err(ActionError::Invalid(
            "this relationship has already resolved into a social arc".into(),
        ));
    }
    if text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_DIRECTION)? != "none" {
''',
    "action social-arc guard",
)
anchor = '''    #[test]
    fn deterministic_default_mind_keeps_identical_worlds_reproducible() {
'''
test = r'''    #[test]
    fn resolved_social_arc_closes_relationship_steering() {
        let mut universe = PocketUniverse::new().unwrap();
        universe
            .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
            .unwrap();
        universe.advance_periods(5).unwrap();

        assert_eq!(
            universe
                .world()
                .state()
                .entity(RELATIONSHIP)
                .unwrap()
                .component(RELATIONSHIP_SOCIAL_ARC),
            Some(&Value::Text("partnership".into()))
        );
        assert_eq!(
            universe
                .world()
                .state()
                .entity(RELATIONSHIP)
                .unwrap()
                .component(RELATIONSHIP_DIRECTION),
            Some(&Value::Text("none".into()))
        );
        let snapshot = universe.projection_snapshot();
        assert!(snapshot.command(SHARED_PROJECT_COMMAND).is_none());
        assert!(snapshot.command(RIVALRY_COMMAND).is_none());

        let before = universe.archive().unwrap();
        let error = universe
            .invoke_projection_command(RIVALRY_COMMAND)
            .expect_err("resolved relationship must reject later steering");
        assert!(error
            .to_string()
            .contains("already resolved into a social arc"));
        assert_eq!(universe.archive().unwrap(), before);
    }

'''
text = replace_once(text, anchor, test + anchor, "resolved steering regression")
lib_path.write_text(text)

projection_path = ROOT / "worlds/pocket-universe/src/projection.rs"
projection = projection_path.read_text()
projection = replace_once(
    projection,
    '''    NUDGE_COMMAND, RELATIONSHIP, RELATIONSHIP_DIRECTION, RIVALRY_COMMAND, SEED_1980S_TOWN_COMMAND,
''',
    '''    NUDGE_COMMAND, RELATIONSHIP, RELATIONSHIP_DIRECTION, RELATIONSHIP_SOCIAL_ARC,
    RIVALRY_COMMAND, SEED_1980S_TOWN_COMMAND,
''',
    "projection social-arc import",
)
projection = replace_once(
    projection,
    '''    let relationship_direction = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_DIRECTION,
        "none",
    );
    if generation >= 2 && relationship_direction == "none" {
''',
    '''    let relationship_direction = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_DIRECTION,
        "none",
    );
    let relationship_social_arc = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_SOCIAL_ARC,
        "forming",
    );
    if generation >= 2
        && relationship_direction == "none"
        && relationship_social_arc == "forming"
    {
''',
    "projection resolved steering guard",
)
projection_path.write_text(projection)
