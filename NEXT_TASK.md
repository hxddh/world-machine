# Next Coding Task — M28D Durable Background Progression

Connect the M28 background-living capability to durable `.world` documents without weakening the transaction, integrity, or wall-clock boundaries already established.

Current baseline:

- `TinySocietyBranch::advance_days(days)` deterministically advances one Tiny Society day per 10 World ticks.
- Society Today aggregates factual background activity into `While you were away`.
- `WorldSession::advance_background(periods)` is a generic Host lifecycle hook with a default no-op.
- Tiny Society maps one background period to one living day; Future Archaeologist remains static by default.
- Host integrity gates validate archives both before Pack openers and before Registry-managed sessions expose archive output.

Requirements:

1. Add `DurableWorldSession::advance_background(periods, registry, library)` using the same transaction shape as a normal user intent:
   - verify the current document revision;
   - capture the current checked archive;
   - reopen a candidate session through `WorldRegistry`;
   - run `candidate.advance_background(periods)`;
   - obtain the candidate's checked archive;
   - verify the document revision again;
   - atomically persist the candidate archive;
   - only then replace the live session/revision.
2. A background progression failure, integrity failure, document conflict, or persistence failure must leave the current live `DurableWorldSession` unchanged.
3. `periods == 0` must remain a strict no-op and should avoid unnecessary document writes.
4. Add Library tests proving a Tiny Society durable document advances, persists, and reopens with the new World time/history.
5. Add a failure regression proving that if persistence fails after candidate progression, the live durable session remains at the original World time/history.
6. Add a conflict regression proving that a stale durable session cannot overwrite a document changed by another session during background progression.
7. Do not read wall-clock time in `world-core`, `world-host`, `world-library`, Tiny Society domain logic, or `.world` archives. M28D accepts only an explicit number of abstract background periods.
8. Do not add a background-progression `ProjectionIntent` or visible command. This is a document/lifecycle operation, not a World user action.
9. Keep Future Archaeologist behavior unchanged through the default no-op Host implementation.
10. Linux semantic CI, macOS `world-library` tests, all desktop regressions, and release `World Machine.app` artifact build must remain green.

After M28D is green, the next product step is to add observer-side last-visit metadata and a bounded wall-clock-to-period policy in the desktop shell. That metadata must remain outside World truth and `.world` history.
