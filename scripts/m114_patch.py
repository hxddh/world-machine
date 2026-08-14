from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one match, found {count}")
    file.write_text(text.replace(old, new, 1))


replace_once(
    "worlds/pocket-universe/src/projection.rs",
    '''    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let (relationship_choice_available, intervention_choice_available) =
        choice_state(world, generation);
    let posture_choice_available = posture_choice_state(world, generation);
    let (nudge_title, nudge_detail) = if posture_choice_available {
        (
            "Let the next chapter wait",
            "Keep watching before deciding whether this World reaches outward or roots itself more deeply.",
        )
    } else {
        nudge_copy(
            seed_id(world),
            generation,
            relationship_choice_available,
            intervention_choice_available,
        )
    };''',
    '''    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let (relationship_choice_available, intervention_choice_available) =
        choice_state(world, generation);
    let posture_choice_available = posture_choice_state(world, generation);
    let legacy = text_component(world.state().entity(UNIVERSE), LEGACY, "forming");
    let (nudge_title, nudge_detail) = if posture_choice_available {
        (
            "Let the next chapter wait",
            "Keep watching before deciding whether this World reaches outward or roots itself more deeply.",
        )
    } else if legacy != "forming" {
        legacy_nudge_copy(seed_id(world), &legacy)
    } else {
        nudge_copy(
            seed_id(world),
            generation,
            relationship_choice_available,
            intervention_choice_available,
        )
    };''',
)

replace_once(
    "worlds/pocket-universe/src/projection.rs",
    '''fn nudge_copy(
    seed: &str,
    generation: i64,''',
    '''fn legacy_nudge_copy(seed: &str, legacy: &str) -> (&'static str, &'static str) {
    match (seed, legacy) {
        ("mars-colony", "ridge-network") => (
            "Let the ridge network carry on",
            "Let another sol move through the ridge routes and see what this durable expedition network changes next.",
        ),
        ("mars-colony", "competing-frontiers") => (
            "Let the competing frontiers advance",
            "Let another sol pass while rival survey routes keep defining different edges of Ares.",
        ),
        ("mars-colony", "habitat-commons") => (
            "Let the habitat commons deepen",
            "Let another sol move through the commons and see what shared life inside Ares makes durable next.",
        ),
        ("mars-colony", "sealed-districts") => (
            "Let the sealed districts settle",
            "Let another sol pass while Ares keeps organizing safety and trust around its separated districts.",
        ),
        ("1980s-town", "night-network") => (
            "Let the night network carry on",
            "Let another night move through the radio, arcade, bus, and people now connected by the network.",
        ),
        ("1980s-town", "rival-scenes") => (
            "Let the rival scenes keep moving",
            "Let another night pass while Maple Street's competing scenes keep pulling the neighborhood in different directions.",
        ),
        ("1980s-town", "neighborhood-commons") => (
            "Let the neighborhood commons deepen",
            "Let another night pass through the shared places and routines that now hold Maple Street together.",
        ),
        ("1980s-town", "split-blocks") => (
            "Let the split blocks settle",
            "Let another night pass while different blocks keep carrying different versions of neighborhood life.",
        ),
        ("penguin-civilization", "aurora-league") => (
            "Let the aurora league carry on",
            "Let another aurora move through the routes now coordinated between Icebridge and the outer colonies.",
        ),
        ("penguin-civilization", "rival-routes") => (
            "Let the rival routes advance",
            "Let another aurora pass while competing colony routes keep redrawing cooperation beyond Icebridge.",
        ),
        ("penguin-civilization", "winter-commons") => (
            "Let the winter commons deepen",
            "Let another aurora pass through the shared systems that now carry Icebridge through the dark season.",
        ),
        ("penguin-civilization", "divided-houses") => (
            "Let the divided houses settle",
            "Let another aurora pass while Icebridge's winter houses keep organizing life around separate loyalties.",
        ),
        _ => (
            "Let this legacy carry on",
            "Let one more persistent change unfold inside the World this legacy has already shaped.",
        ),
    }
}

fn nudge_copy(
    seed: &str,
    generation: i64,''',
)

replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.1";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.13.2";',
)
replace_once(
    "worlds/pocket-universe/Cargo.toml",
    'version = "0.13.1"',
    'version = "0.13.2"',
)
replace_once(
    "apps/pocket-universe-pack/Cargo.toml",
    'version = "0.13.1"',
    'version = "0.13.2"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'version: "0.13.1"',
    'version: "0.13.2"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'assert_eq!(packs[0].pack.version, "0.13.1");',
    'assert_eq!(packs[0].pack.version, "0.13.2");',
)
