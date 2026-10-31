//! Which looks a window's living stage keeps (moved out of `stage_view`,
//! which may not grow): each look a want asks for, the look standing in
//! for one not ready, and always one look of the whole place.

use super::stage_view::{choose, Held, Variant, Want, SPANNED_FROM};

/// Which of `held` a window keeps: each look a want asks for (ready or on
/// its way to the display), and while one the frame shows is not ready,
/// the look standing in for it: for the camera's own (the first), and for
/// any other of a level baked whole. (v0.30 S1c: a close level's light
/// fading in stands in with whatever stands in for the first, rather than
/// keeping a close look of its own; a look baked ahead stands in for
/// nothing. Four close looks at once were more pixels than the budget.)
///
/// And always one ready look of the whole place, if one is held (1.0 P5:
/// the soak's unpainted frames). When the World changed (a day passed),
/// its old whole look answered nothing and was let go while the new one
/// waited behind the camera's own close look; a zoom out then found no
/// look covering what the camera saw, and showed nothing for 100-450 ms.
/// The whole look kept is the nearest the first want: its light, its
/// World, then the softest level (the least pixels).
///
/// Everything else is let go.
pub fn keep(held: &[Held], wants: &[Want]) -> Vec<bool> {
    let mut keep = vec![false; held.len()];
    for (nth, want) in wants.iter().enumerate() {
        if let Some(at) = held.iter().position(|held| held.answers(want)) {
            keep[at] = true;
            if held[at].ready {
                continue;
            }
        }
        if !want.shown || (nth > 0 && want.level >= SPANNED_FROM) {
            continue;
        }
        if let Some(at) = choose(held, want) {
            keep[at] = true;
        }
    }
    let whole = |held: &Held| held.ready && held.span.is_none() && held.variant == Variant::Main;
    let kept_whole = held
        .iter()
        .zip(&keep)
        .any(|(held, kept)| *kept && whole(held));
    if let (false, Some(first)) = (kept_whole, wants.first()) {
        if let Some((at, _)) = held
            .iter()
            .enumerate()
            .filter(|(_, held)| whole(held))
            .min_by_key(|(_, held)| (held.light != first.light, held.key != first.key, held.level))
        {
            keep[at] = true;
        }
    }
    keep
}
