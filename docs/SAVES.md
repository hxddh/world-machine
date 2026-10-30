# Saves: the 1.0 contract

World Machine 1.0 starts its own save format. No save, World code, fixture or digest from a v0.x preview opens or replays (the owner's decision of 2026-10-30, [REVIEW_v0.30](REVIEW_v0.30.md), "Already decided"). From 1.0 on, every release keeps opening the saves of every release before it.

## What a World file is

A `.world` file is one gzip stream of JSON, written atomically, with the file as it was before kept beside it as `<name>.world.bak`:

- `summary`: the Pack and its version, the World's time and event count, the file `schema` (`WORLD_FILE_SCHEMA`, 2 for 1.0) and the app that wrote it (`written_by`). A list of Worlds reads only this.
- `document`: host metadata. This is what Home shows (name, cover, colours) and the sitting ledger (`host_sittings`: how many sittings, the local day the latest began on, the World days it passed, the morning read). None of it is an Event or State. The World never reads it, replay never needs it, and the local day in the ledger is the only wall-clock reading anywhere in the file. The ledger travels with the file, so a World that is moved, synced, saved as a new file, exported or imported keeps its sittings.
- The archive, in the compact encoding: `format_version` (`WORLD_ARCHIVE_COMPACT_VERSION`, 4 for 1.0; in memory and on the Pack wire it is `WORLD_ARCHIVE_VERSION`, 3), the Pack, the recorded events, what is pending, and a checkpoint at the start of the latest season.

A World code is `wm3:` plus the document as deflated compact JSON (see `world_library::world_code`).

## What is refused

A save is refused whole, before any World is opened from it. Nothing is half-loaded, and the file is left exactly as it was.

| Found | Refused as | The player reads |
|---|---|---|
| a file `schema` above 2, an archive `format_version` above 4, or a Pack version above the installed Pack's | newer (`LibraryError::NewerSave`) | "This World was saved by a newer World Machine. Update to open it." |
| no file summary (before v0.25), a `schema` of 0 or 1 (v0.25–v0.30), an archive `format_version` of 1 or 2, a Pack version before `1.0.0`, or a `wm1:`/`wm2:` code | preview (`LibraryError::PreviewSave`) | "This World is from a preview version and can't be opened." |

Both sentences are in the app's Chinese and Japanese catalogs. Home lists a refused file with its sentence instead of hiding it. A newer file is never written over: not by a rename, a save, an export or an import.

A file that fails to load for any other reason (cut short, damaged) can be put back to its last good save. Home offers **Restore the last good save** when the `.bak` beside it reads as a 1.0 World file. The damaged file is kept beside it as `<name>.world.damaged` (`world_library::restore_backup`). Only the save before the last is kept.

## The rules, for anyone changing a Pack or the format

1. **Bump the Pack version whenever an Action, a System or a payload changes.** That covers anything that changes what an Action records, what replaying recorded events comes to, or what a payload means. Tiny Society and Pocket Universe are `1.0.0` at 1.0.
2. **A Pack keeps opening every release's saves.** A bumped Pack must still open the versions it shipped. It can keep replaying them exactly, or register the earlier version beside the new one: `WorldRegistry` keeps every registered version of a Pack for exact restoration. Today each Pack opens only its own `1.0.0`, so the first bump after 1.0 has to add that path. The release-saves test below fails the moment it does not.
3. **Bump `WORLD_FILE_SCHEMA`** when a file holds something an older app would misread or lose. **Bump the archive versions** when the archive's encoding changes. An older app then refuses the file as newer instead of misreading it.
4. **Add one save per Pack per release, from 1.0 on, and never rewrite one.** Each Pack's `tests/releases.rs` takes the harness with one line (`world_pack_testkit::release_saves!`, see `crates/world-pack-testkit/src/release.rs`), and a release adds its save on its own tree with one command:

   ```text
   WORLD_PACK_WRITE_RELEASE=v1.0.0 cargo test -p tiny-society --test releases -- --ignored
   WORLD_PACK_WRITE_RELEASE=v1.0.0 cargo test -p pocket-universe --test releases -- --ignored
   ```

   That writes `worlds/<pack>/tests/releases/v1.0.0-<name>.world`, a World played 30 days by the builder the app's way, to check in. Every later tree checks each one. It must open as the app opens it, replay event for event, show the same snapshot each time, play on with every offered command accepted, only append, and reopen to what it showed. Until 1.0 ships the folder is empty, and the same checks run on a save written by the tree itself.
5. **Determinism is tested on fresh Worlds.** `world_pack_testkit::replay::assert_fresh_world_replays` plays a new World and checks several things: its recorded events rebuild the very World that was played, the snapshot and its wire JSON byte for byte, and playing on from the save records what playing straight through records. No LLM or other decision maker is asked again.
6. **Wall-clock time never enters Events or State.** The sitting ledger is host metadata in the file. `worlds/tiny-society/tests/one_day_a_day.rs` plays the same sittings on two calendars far apart. It checks that the events and state come out the same and hold no number of either clock, while the file carries the ledger beside them.
