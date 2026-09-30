//! Shared test harnesses for World Packs.
//!
//! A Pack keeps only its own words, content and choices; the harnesses that
//! check it (replay-identical fixtures, long runs, five players, the red team
//! and the seams between Systems) live here, free of any one Pack's concepts.

pub mod density;
pub mod players;
pub mod red_team;
pub mod replay;
pub mod seams;
pub mod snapshot_json;
pub mod town;
