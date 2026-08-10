use tiny_society::TinySociety;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut simulation = TinySociety::new()?;
    simulation.run_story()?;

    println!("Tiny Society — deterministic causal story\n");
    for event in simulation.causal_story() {
        let causes = if event.caused_by.is_empty() {
            "root".to_owned()
        } else {
            event
                .caused_by
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "t={:>2}  #{:<2}  {:<26} caused_by={}",
            event.world_time, event.id, event.kind, causes
        );
    }

    Ok(())
}
