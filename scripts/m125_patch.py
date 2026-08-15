from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{path}: expected exactly one match, found {count}: {old[:120]!r}")
    file.write_text(text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str) -> None:
    file = Path(path)
    text = file.read_text()
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise RuntimeError(f"{path}: expected exactly one regex match, found {count}: {pattern!r}")
    file.write_text(updated)


# Version sync: projection-only product increment.
replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.4";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.5";',
)
replace_once(
    "worlds/pocket-universe/Cargo.toml",
    'version = "0.14.4"',
    'version = "0.14.5"',
)
replace_once(
    "apps/pocket-universe-pack/Cargo.toml",
    'version = "0.14.4"',
    'version = "0.14.5"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'version: "0.14.4",',
    'version: "0.14.5",',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'assert_eq!(packs[0].pack.version, "0.14.4");',
    'assert_eq!(packs[0].pack.version, "0.14.5");',
)
regex_once(
    "Cargo.lock",
    r'(\[\[package\]\]\nname = "pocket-universe"\nversion = ")0\.14\.4(")',
    r'\g<1>0.14.5\2',
)
regex_once(
    "Cargo.lock",
    r'(\[\[package\]\]\nname = "pocket-universe-pack"\nversion = ")0\.14\.4(")',
    r'\g<1>0.14.5\2',
)

projection = "worlds/pocket-universe/src/projection.rs"

# Put rule-level signals directly on live commands.
replace_once(
    projection,
    """    let mut commands = vec![ProjectionCommand {
        id: NUDGE_COMMAND.into(),
        title: nudge_title.into(),
        detail: nudge_detail.into(),
    }];""",
    """    let mut commands = vec![ProjectionCommand {
        id: NUDGE_COMMAND.into(),
        title: nudge_title.into(),
        detail: command_detail_with_signal(world, NUDGE_COMMAND, nudge_detail),
    }];""",
)
replace_once(
    projection,
    """        commands.push(ProjectionCommand {
            id: SHARED_PROJECT_COMMAND.into(),
            title: "Give them a shared project".into(),
            detail: "Create a goal that neither actor can complete alone; future interactions will lean toward trust.".into(),
        });
        commands.push(ProjectionCommand {
            id: RIVALRY_COMMAND.into(),
            title: "Let rivalry sharpen them".into(),
            detail: "Keep both actors independent and let competition add pressure to future interactions.".into(),
        });""",
    """        commands.push(ProjectionCommand {
            id: SHARED_PROJECT_COMMAND.into(),
            title: "Give them a shared project".into(),
            detail: command_detail_with_signal(
                world,
                SHARED_PROJECT_COMMAND,
                "Create a goal that neither actor can complete alone; future interactions will lean toward trust.",
            ),
        });
        commands.push(ProjectionCommand {
            id: RIVALRY_COMMAND.into(),
            title: "Let rivalry sharpen them".into(),
            detail: command_detail_with_signal(
                world,
                RIVALRY_COMMAND,
                "Keep both actors independent and let competition add pressure to future interactions.",
            ),
        });""",
)
replace_once(
    projection,
    """        commands.push(ProjectionCommand {
            id: BOLD_PATH_COMMAND.into(),
            title: bold_title.into(),
            detail: bold_detail.into(),
        });
        commands.push(ProjectionCommand {
            id: CAREFUL_PATH_COMMAND.into(),
            title: careful_title.into(),
            detail: careful_detail.into(),
        });""",
    """        commands.push(ProjectionCommand {
            id: BOLD_PATH_COMMAND.into(),
            title: bold_title.into(),
            detail: command_detail_with_signal(world, BOLD_PATH_COMMAND, bold_detail),
        });
        commands.push(ProjectionCommand {
            id: CAREFUL_PATH_COMMAND.into(),
            title: careful_title.into(),
            detail: command_detail_with_signal(world, CAREFUL_PATH_COMMAND, careful_detail),
        });""",
)
replace_once(
    projection,
    """        commands.push(ProjectionCommand {
            id: OUTWARD_POSTURE_COMMAND.into(),
            title: outward_title.into(),
            detail: outward_detail.into(),
        });
        commands.push(ProjectionCommand {
            id: ROOTED_POSTURE_COMMAND.into(),
            title: rooted_title.into(),
            detail: rooted_detail.into(),
        });""",
    """        commands.push(ProjectionCommand {
            id: OUTWARD_POSTURE_COMMAND.into(),
            title: outward_title.into(),
            detail: command_detail_with_signal(world, OUTWARD_POSTURE_COMMAND, outward_detail),
        });
        commands.push(ProjectionCommand {
            id: ROOTED_POSTURE_COMMAND.into(),
            title: rooted_title.into(),
            detail: command_detail_with_signal(world, ROOTED_POSTURE_COMMAND, rooted_detail),
        });""",
)

marker = "\nfn choice_state(world: &World, generation: i64) -> (bool, bool) {"
helper = r'''

fn command_detail_with_signal(world: &World, command_id: &str, detail: &str) -> String {
    match command_choice_signal(world, command_id) {
        Some(signal) => format!("{detail} Choice signal: {signal}."),
        None => detail.into(),
    }
}

fn command_choice_signal(world: &World, command_id: &str) -> Option<String> {
    match command_id {
        NUDGE_COMMAND => Some(
            "one full cycle resolves under current rules: world growth, both actor turns, relationship update, then period consequences"
                .into(),
        ),
        SHARED_PROJECT_COMMAND | RIVALRY_COMMAND => {
            let relationship = world.state().entity(RELATIONSHIP);
            let trust = integer_entity_component(relationship, RELATIONSHIP_TRUST).unwrap_or_default();
            let tension = integer_entity_component(relationship, RELATIONSHIP_TENSION).unwrap_or_default();
            if command_id == SHARED_PROJECT_COMMAND {
                let next_trust = (trust + 2).clamp(0, 10);
                let next_tension = (tension - 1).clamp(0, 10);
                Some(format!(
                    "trust {trust} → {next_trust} · tension {tension} → {next_tension}; each later relationship shift also gains +1 trust and -1 tension"
                ))
            } else {
                let next_tension = (tension + 2).clamp(0, 10);
                Some(format!(
                    "trust {trust} → {trust} · tension {tension} → {next_tension}; each later relationship shift also gains +1 tension"
                ))
            }
        }
        BOLD_PATH_COMMAND => Some(intervention_choice_signal(seed_id(world), true).into()),
        CAREFUL_PATH_COMMAND => Some(intervention_choice_signal(seed_id(world), false).into()),
        OUTWARD_POSTURE_COMMAND => Some(
            "sets durable World direction to Outward; later growth and legacy formation read the outward posture"
                .into(),
        ),
        ROOTED_POSTURE_COMMAND => Some(
            "sets durable World direction to Rooted; later growth and legacy formation read the rooted posture"
                .into(),
        ),
        _ => None,
    }
}

fn intervention_choice_signal(seed: &str, bold: bool) -> &'static str {
    match (seed, bold) {
        ("mars-colony", true) => {
            "locks the first intervention to Signal expedition; Kestrel's durable status becomes signal expedition"
        }
        ("mars-colony", false) => {
            "locks the first intervention to Fortified habitat; Ares Habitat's durable status becomes storm sealed"
        }
        ("1980s-town", true) => {
            "locks the first intervention to Community arcade; Maple Arcade's durable status becomes community nights"
        }
        ("1980s-town", false) => {
            "locks the first intervention to Steady business; Maple Arcade's durable status becomes steady business"
        }
        ("penguin-civilization", true) => {
            "locks the first intervention to Winter feast; Fish Vault's durable reserve becomes festival opened"
        }
        ("penguin-civilization", false) => {
            "locks the first intervention to Conserved reserves; Fish Vault's durable reserve becomes winter conserved"
        }
        (_, true) => "locks a durable bold intervention that later growth can read",
        (_, false) => "locks a durable careful intervention that later growth can read",
    }
}
'''
replace_once(projection, marker, helper + marker)

# Return Compass now pairs every current action with its rule-level signal.
replace_once(
    projection,
    """        (false, Some(nudge)) => {
            let choices = shaping
                .iter()
                .map(|command| format!("‘{}’", command.title))
                .collect::<Vec<_>>()
                .join(" · ");
            format!(
                "Available now: {choices}. Or choose ‘{}’ and let current dynamics keep moving without a larger choice.",
                nudge.title
            )
        }""",
    """        (false, Some(nudge)) => {
            let choices = shaping
                .iter()
                .map(|command| {
                    let signal = command_choice_signal(world, command.id.as_str())
                        .unwrap_or_else(|| "changes durable World state through this action".into());
                    format!("‘{}’ — {signal}", command.title)
                })
                .collect::<Vec<_>>()
                .join(" · ");
            let nudge_signal = command_choice_signal(world, nudge.id.as_str())
                .unwrap_or_else(|| "continues from the current durable state".into());
            format!(
                "Choice signals: {choices}. ‘{}’ — {nudge_signal}.",
                nudge.title
            )
        }""",
)

# Extend the existing return regressions so live commands and the return surface prove the same rules.
tests = "worlds/pocket-universe/tests/return_compass.rs"
replace_once(
    tests,
    """use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};""",
    """use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, OUTWARD_POSTURE_COMMAND,
    RIVALRY_COMMAND, ROOTED_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND, SHARED_PROJECT_COMMAND,
};""",
)
replace_once(
    tests,
    """    assert!(compass
        .detail
        .contains("Its durable direction is still open."));
    for command in &snapshot.commands {""",
    """    assert!(compass
        .detail
        .contains("Its durable direction is still open."));
    let shared = snapshot
        .commands
        .iter()
        .find(|command| command.id == SHARED_PROJECT_COMMAND)
        .expect("shared project should be available");
    let rivalry = snapshot
        .commands
        .iter()
        .find(|command| command.id == RIVALRY_COMMAND)
        .expect("rivalry should be available");
    let nudge = &snapshot.commands[0];
    assert!(shared.detail.contains(
        "Choice signal: trust 2 → 4 · tension 0 → 0; each later relationship shift also gains +1 trust and -1 tension."
    ));
    assert!(rivalry.detail.contains(
        "Choice signal: trust 2 → 2 · tension 0 → 2; each later relationship shift also gains +1 tension."
    ));
    assert!(nudge.detail.contains(
        "Choice signal: one full cycle resolves under current rules: world growth, both actor turns, relationship update, then period consequences."
    ));
    assert!(compass.detail.contains("Choice signals:"));
    assert!(compass.detail.contains(
        "trust 2 → 4 · tension 0 → 0; each later relationship shift also gains +1 trust and -1 tension"
    ));
    assert!(compass.detail.contains(
        "trust 2 → 2 · tension 0 → 2; each later relationship shift also gains +1 tension"
    ));
    for command in &snapshot.commands {""",
)
replace_once(
    tests,
    """    assert!(compass.detail.contains("Current thread:"));
    for command in &snapshot.commands {""",
    """    assert!(compass.detail.contains("Current thread:"));
    let bold = snapshot
        .commands
        .iter()
        .find(|command| command.id == BOLD_PATH_COMMAND)
        .expect("bold intervention should be available");
    let careful = snapshot
        .commands
        .iter()
        .find(|command| command.id == CAREFUL_PATH_COMMAND)
        .expect("careful intervention should be available");
    assert!(bold.detail.contains(
        "Choice signal: locks the first intervention to Signal expedition; Kestrel's durable status becomes signal expedition."
    ));
    assert!(careful.detail.contains(
        "Choice signal: locks the first intervention to Fortified habitat; Ares Habitat's durable status becomes storm sealed."
    ));
    assert!(compass.detail.contains(
        "locks the first intervention to Signal expedition; Kestrel's durable status becomes signal expedition"
    ));
    assert!(compass.detail.contains(
        "locks the first intervention to Fortified habitat; Ares Habitat's durable status becomes storm sealed"
    ));
    for command in &snapshot.commands {""",
)
replace_once(
    tests,
    """    assert!(
        compass.detail.contains("Signal expedition"),
        "posture context should reuse the durable intervention"
    );
    for command in &snapshot.commands {""",
    """    assert!(
        compass.detail.contains("Signal expedition"),
        "posture context should reuse the durable intervention"
    );
    let outward = snapshot
        .commands
        .iter()
        .find(|command| command.id == OUTWARD_POSTURE_COMMAND)
        .expect("outward posture should be available");
    let rooted = snapshot
        .commands
        .iter()
        .find(|command| command.id == ROOTED_POSTURE_COMMAND)
        .expect("rooted posture should be available");
    assert!(outward.detail.contains(
        "Choice signal: sets durable World direction to Outward; later growth and legacy formation read the outward posture."
    ));
    assert!(rooted.detail.contains(
        "Choice signal: sets durable World direction to Rooted; later growth and legacy formation read the rooted posture."
    ));
    assert!(compass.detail.contains(
        "sets durable World direction to Outward; later growth and legacy formation read the outward posture"
    ));
    assert!(compass.detail.contains(
        "sets durable World direction to Rooted; later growth and legacy formation read the rooted posture"
    ));
    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot_since(Some(since)),
        snapshot,
        "choice signals should be derived entirely from durable state and current rules"
    );
    for command in &snapshot.commands {""",
)
