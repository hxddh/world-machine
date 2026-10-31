# The free demo

The demo is the desktop app built with `--features demo`
(`src/demo.rs`); everything else is the full app's. It plays one slice of
Tiny Society, the same one a new player of the full game meets:

1. **The first session.** A new World's first sitting passes six World days
   (`world_observer::FIRST_SITTING_DAYS`, the designed first session walked
   by `tests/first_session.rs`) and ends on the Goodnight card.
2. **A second morning.** The demo always plays freely, so Goodnight offers
   "Sleep now": the next sitting begins at once through the sitting ledger
   (`SittingLedger::sleep_now`, the same path Settings' "Play freely" takes
   in the full app), and the window opens on that sitting's morning card.
   The ledger is host metadata in the World's file; the World itself is only
   told that a sitting began, never when. Nothing waits for a real day.
3. **The wishlist card.** That sitting's one day is the demo's last
   (`demo::last_day`, day 7). Letting it pass shows the closing card at
   dusk instead: "Thanks for playing the demo", a recap of what the player
   did from the World's own record, a resident's goodbye, a postcard to
   keep, and **Wishlist Tiny Society on Steam** (the game's name comes from
   its Pack's descriptor). Choosing it opens the store page in the browser;
   nothing is sent anywhere. The day is held, not spent: "Stay a while"
   puts the card away and the evening goes on.

## Pacing

`pacing.json` holds the paces the demo is walked at and when each beat must
arrive. `tests/demo_walk.rs` plays the whole slice headlessly, through the
same ledger and gate as the window, to the wishlist card. At the intended
pace the demo is about 32 minutes (the bar is 25 to 45): Goodnight at about
26 minutes, the second morning right after it, and the wishlist card read by
about 32. A player who goes straight to the cards reaches it in about five.

## Saves carry over

The demo keeps its World in the ordinary Library, the same Worlds folder
as the full app, with its sittings in the World's file. Installing the full
game later opens that World where the demo left it: the same day, the same
sitting (its last day still to play that day), and everything it recorded
(`the_demo_world_opens_in_the_full_app_and_plays_on`). There is no
separate demo save and nothing to import. Both apps can be installed side
by side; a lock file keeps them from writing one World at once.

## For the owner: the store link

The wishlist opens `demo::WISHLIST_URL`: the URL given at build time in
`WORLD_MACHINE_WISHLIST_URL`, else `demo::STORE_PAGE_PLACEHOLDER`, which is
Steam's front page until the game has a store page. Set the variable in the
release workflow (or replace the constant) once the page exists.
