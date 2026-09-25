//! Play a Pocket Universe World the way a person would — take the first
//! offered choice, leave for a week, come back — and print every screen as
//! text, so the words a player reads can be reviewed without the app.
//!
//! ```bash
//! cargo run -p pocket-universe --example playthrough
//! ```

use pocket_universe::PocketUniverse;
use world_projection::ProjectionSnapshot;

fn dump(label: &str, s: &ProjectionSnapshot) {
    println!("\n==================== {label} ====================");
    println!("TITLE: {}   | World time {}", s.title, s.world_time);
    if let Some(b) = &s.briefing {
        println!("[{}] {}", b.eyebrow, b.title);
        for i in &b.items {
            println!("  * {} — {}", i.title, i.detail);
        }
    }
    for c in &s.commands {
        println!("  > [{}] {} — {}", c.id, c.title, c.detail);
    }
    println!("-- collection: {}", s.collection.title);
    for c in &s.collection.items {
        println!("   - {} | {}", c.title, c.subtitle);
    }
    println!("-- timeline ({} items), first 6:", s.timeline.items.len());
    for t in s.timeline.items.iter().take(6) {
        println!("   t={} {} | {}", t.world_time, t.title, t.subtitle);
    }
    println!("-- canvas: {} items", s.canvas.items.len());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut u = PocketUniverse::new()?;
    dump("FIRST OPEN", &u.projection_snapshot());
    for step in 0..14 {
        let snap = u.projection_snapshot();
        if let Some(c) = snap.commands.first() {
            let id = c.id.clone();
            println!("\n>>> USER CLICKS: {id}");
            u.invoke_projection_command(&id)?;
            dump(&format!("AFTER CLICK {step}"), &u.projection_snapshot());
        }
        let since = u.world().events().len();
        u.advance_periods(7)?;
        dump(
            &format!("RETURN AFTER AWAY {step}"),
            &u.projection_snapshot_since(Some(since)),
        );
    }
    Ok(())
}
