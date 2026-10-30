#![forbid(unsafe_code)]

//! Sittings (v0.30 S4, "one day a day"), worked out from a World's sitting
//! ledger and the player's own day. The ledger is host metadata a World's
//! file carries beside its history (`world_document::DocumentSittings`),
//! never an Event or State: it says which sitting opening a World begins
//! and how many of its days it may pass, and nothing it holds is ever
//! replayed.

mod sitting;
pub use sitting::{
    local_day, sitting_at, Sitting, SittingLedger, FIRST_SITTING_DAYS, SECONDS_PER_DAY,
    SITTING_DAYS,
};
