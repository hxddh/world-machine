from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"missing expected block in {path}: {old[:80]!r}")
    p.write_text(text.replace(old, new, 1))


projection_path = "worlds/pocket-universe/src/projection.rs"
projection = Path(projection_path).read_text()
projection = projection.replace(
    "    LEGACY_SUMMARY, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, RELATIONSHIP,\n",
    "    LEGACY_CYCLES, LEGACY_SUMMARY, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, RELATIONSHIP,\n",
    1,
)

old_compass_detail = '''    let detail = match (shaping.is_empty(), nudge) {
        (true, Some(nudge)) => format!("Continue with ‘{}’. {}", nudge.title, nudge.detail),
        (false, Some(nudge)) => {
            let choices = shaping
                .iter()
                .map(|command| format!("‘{}’", command.title))
                .collect::<Vec<_>>()
                .join(" · ");
            format!(
                "Available now: {choices}. Or choose ‘{}’ and let current dynamics keep moving without a larger choice.",
                nudge.title
            )
        }
        (_, None) => "This World can continue from its current durable state.".into(),
    };

    BriefingItem {
'''
new_compass_detail = '''    let action_detail = match (shaping.is_empty(), nudge) {
        (true, Some(nudge)) => format!("Continue with ‘{}’. {}", nudge.title, nudge.detail),
        (false, Some(nudge)) => {
            let choices = shaping
                .iter()
                .map(|command| format!("‘{}’", command.title))
                .collect::<Vec<_>>()
                .join(" · ");
            format!(
                "Available now: {choices}. Or choose ‘{}’ and let current dynamics keep moving without a larger choice.",
                nudge.title
            )
        }
        (_, None) => "This World can continue from its current durable state.".into(),
    };
    let why_now = return_compass_context(
        world,
        generation,
        relationship_choice_available,
        intervention_choice_available,
        posture_choice_available,
        &legacy,
    );
    let detail = format!("Why now: {why_now} {action_detail}");

    BriefingItem {
'''
if old_compass_detail not in projection:
    raise SystemExit("missing M123 return compass detail block")
projection = projection.replace(old_compass_detail, new_compass_detail, 1)

helper_anchor = "fn return_digest_items(events: &[Event]) -> Vec<BriefingItem> {\n"
helpers = r'''fn return_compass_context(
    world: &World,
    generation: i64,
    relationship_choice_available: bool,
    intervention_choice_available: bool,
    posture_choice_available: bool,
    legacy: &str,
) -> String {
    if posture_choice_available {
        return posture_return_context(world);
    }
    if relationship_choice_available && intervention_choice_available {
        return format!(
            "{} {}",
            relationship_return_context(world),
            intervention_return_context(world, generation)
        );
    }
    if relationship_choice_available {
        return relationship_return_context(world);
    }
    if intervention_choice_available {
        return intervention_return_context(world, generation);
    }
    if legacy != "forming" {
        return legacy_return_context(world, legacy);
    }

    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    format!(
        "Generation {generation} is still carrying its current thread: {last_change}"
    )
}

fn relationship_return_context(world: &World) -> String {
    let relationship = world.state().entity(RELATIONSHIP);
    let trust = integer_entity_component(relationship, RELATIONSHIP_TRUST).unwrap_or_default();
    let tension =
        integer_entity_component(relationship, RELATIONSHIP_TENSION).unwrap_or_default();
    let last_dynamic = text_component(relationship, RELATIONSHIP_LAST_DYNAMIC, "");
    let dynamic = if last_dynamic.trim().is_empty() {
        String::new()
    } else {
        format!(" {last_dynamic}")
    };
    format!(
        "The central relationship is still forming at trust {trust} · tension {tension}.{dynamic} Its durable direction is still open."
    )
}

fn intervention_return_context(world: &World, generation: i64) -> String {
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    format!(
        "Generation {generation} has reached a larger intervention point. Current thread: {last_change}"
    )
}

fn posture_return_context(world: &World) -> String {
    let social_arc = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_SOCIAL_ARC,
        "forming",
    );
    let social_arc = match social_arc.as_str() {
        "partnership" => "partnership".into(),
        "fracture" => "fracture".into(),
        other => other.replace('-', " "),
    };
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    let influence = intervention_influence_copy(&decision)
        .map(|(title, _)| {
            title
                .strip_prefix("Your influence · ")
                .unwrap_or(title)
                .to_string()
        })
        .unwrap_or_else(|| decision.replace('-', " "));
    format!(
        "The first arc has settled as {social_arc}, and {influence} is already durable. The next choice now decides whether this World reaches outward or deepens home."
    )
}

fn legacy_return_context(world: &World, legacy: &str) -> String {
    let cycles = integer_component(world, LEGACY_CYCLES).unwrap_or_default();
    let legacy = legacy_label(legacy);
    if cycles <= 0 {
        let summary = text_component(
            world.state().entity(UNIVERSE),
            LEGACY_SUMMARY,
            "This World now carries a durable legacy from its earlier choices.",
        );
        return format!(
            "World legacy · {legacy} has formed but has not yet reinforced through a later cycle. {summary}"
        );
    }

    let pattern = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "legacy_reinforced")
        .and_then(|event| match event.payload.get("pattern") {
            Some(Value::Text(pattern)) => Some(pattern.as_str()),
            _ => None,
        });
    let cycle_word = if cycles == 1 { "cycle" } else { "cycles" };
    match pattern {
        Some(pattern) => format!(
            "World legacy · {legacy} has reinforced through {cycles} later {cycle_word}; its current pattern is {pattern}. Another continuation feeds that established pattern back into the World."
        ),
        None => format!(
            "World legacy · {legacy} has reinforced through {cycles} later {cycle_word}. Another continuation feeds that established pattern back into the World."
        ),
    }
}

'''
if helper_anchor not in projection:
    raise SystemExit("missing return_digest_items anchor")
projection = projection.replace(helper_anchor, helpers + helper_anchor, 1)
Path(projection_path).write_text(projection)

# Strengthen M123 tests around the new state-derived context and add the posture-stage regression.
test_path = "worlds/pocket-universe/tests/return_compass.rs"
test = Path(test_path).read_text()

relationship_anchor = '''    assert_eq!(
        snapshot.commands.len(),
        3,
        "nudge plus the two relationship choices"
    );
'''
relationship_extra = relationship_anchor + '''    assert!(compass.detail.starts_with("Why now: "));
    assert!(
        compass.detail.contains("trust ") && compass.detail.contains("tension "),
        "relationship context should expose the current durable relationship pressure"
    );
    assert!(compass.detail.contains("Its durable direction is still open."));
'''
if relationship_anchor not in test:
    raise SystemExit("missing relationship test anchor")
test = test.replace(relationship_anchor, relationship_extra, 1)

simultaneous_anchor = '''    assert_eq!(
        snapshot.commands.len(),
        5,
        "one nudge plus two relationship and two intervention choices should be open"
    );
'''
simultaneous_extra = simultaneous_anchor + '''    assert!(compass.detail.starts_with("Why now: "));
    assert!(
        compass
            .detail
            .contains("Generation 3 has reached a larger intervention point."),
        "the compass should explain why the larger intervention is open now"
    );
    assert!(compass.detail.contains("Current thread:"));
'''
if simultaneous_anchor not in test:
    raise SystemExit("missing simultaneous test anchor")
test = test.replace(simultaneous_anchor, simultaneous_extra, 1)

legacy_anchor = '''    let continuation = &snapshot.commands[0];
    assert!(compass.detail.contains(&continuation.title));
'''
legacy_extra = '''    let continuation = &snapshot.commands[0];
    assert!(compass.detail.starts_with("Why now: "));
    assert!(compass.detail.contains("World legacy · Ridge Network"));
    assert!(compass.detail.contains("1 later cycle"));
    assert!(compass.detail.contains("adaptive cycle 1"));
    assert!(compass.detail.contains(&continuation.title));
'''
if legacy_anchor not in test:
    raise SystemExit("missing living legacy test anchor")
test = test.replace(legacy_anchor, legacy_extra, 1)

legacy_end_anchor = '''    assert!(
        compass.detail.contains(&continuation.detail),
        "when continuation is the only action, the compass should reuse its semantic explanation"
    );

    Ok(())
}
'''
legacy_end_extra = '''    assert!(
        compass.detail.contains(&continuation.detail),
        "when continuation is the only action, the compass should reuse its semantic explanation"
    );

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot_since(Some(since)),
        snapshot,
        "return context should be derived entirely from durable state and event history"
    );

    Ok(())
}
'''
if legacy_end_anchor not in test:
    raise SystemExit("missing living legacy test ending")
test = test.replace(legacy_end_anchor, legacy_end_extra, 1)

posture_test = r'''
#[test]
fn return_compass_explains_why_world_direction_is_open() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    let since = universe.world().events().len();
    universe.advance_periods(3)?;

    let snapshot = universe.projection_snapshot_since(Some(since));
    let compass = snapshot
        .briefing
        .as_ref()
        .expect("returning Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == "Your turn · World direction")
        .expect("the return compass should explain why the second-arc posture choice is open");

    assert!(compass.detail.starts_with("Why now: "));
    assert!(
        compass.detail.contains("The first arc has settled as partnership"),
        "posture context should reuse the durable social arc"
    );
    assert!(
        compass.detail.contains("Signal expedition"),
        "posture context should reuse the durable intervention"
    );
    for command in &snapshot.commands {
        assert!(
            compass.detail.contains(&command.title),
            "the contextual compass must still name every actually available command: {}",
            command.title
        );
    }

    Ok(())
}

'''
legacy_test_anchor = "#[test]\nfn return_compass_explains_how_to_continue_a_living_legacy()"
if legacy_test_anchor not in test:
    raise SystemExit("missing living legacy test function anchor")
test = test.replace(legacy_test_anchor, posture_test + legacy_test_anchor, 1)
Path(test_path).write_text(test)

# Presentation-only release bump: 0.14.3 -> 0.14.4.
for path in [
    "worlds/pocket-universe/Cargo.toml",
    "apps/pocket-universe-pack/Cargo.toml",
    "apps/world-machine-desktop/src/included_packs.rs",
    "worlds/pocket-universe/src/lib.rs",
]:
    p = Path(path)
    text = p.read_text()
    if "0.14.3" not in text:
        raise SystemExit(f"missing 0.14.3 in {path}")
    p.write_text(text.replace("0.14.3", "0.14.4"))

lock = Path("Cargo.lock")
lock_text = lock.read_text()
for package in ["pocket-universe", "pocket-universe-pack"]:
    pattern = re.compile(
        rf'(\[\[package\]\]\nname = "{re.escape(package)}"\nversion = ")0\.14\.3(")'
    )
    lock_text, count = pattern.subn(r'\g<1>0.14.4\g<2>', lock_text, count=1)
    if count != 1:
        raise SystemExit(f"expected one Cargo.lock version for {package}, got {count}")
lock.write_text(lock_text)
