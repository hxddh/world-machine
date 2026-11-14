# World Machine v1.0.0-pre.1: what stood between it and a stranger using it

Reviewed on 2026-11-14 at `7a47710`. The owner asked for a review of the latest version, with no compatibility to keep, so that the next version is genuinely usable.

## How it was reviewed

- **Played.** The release app was built and played in its real window under Xvfb (`scripts/linux-preview.sh`), from a first launch through the whole first sitting to Goodnight, a return after 30 hours, Settings in Chinese at 200% text, Play freely, export and the demo library.
- **Read.** Three independent reads of the code: saves and the library; the Pack runtime and host; the desktop shell and the World window. Each finding was checked against the code before it was acted on.
- **Checked.** The boundary script, the CI and nightly history, and the releases.

## What was found

The loop works: the first sitting ran six days to Goodnight with no crash, hang or soft lock, and the Chinese UI showed no English. The kernel is healthy (`world-core` is 3,100 lines with no dependencies, and the boundary check passes), and the writer, the lock and the bounds on untrusted input are sound. What stood in the way:

1. **A turn was three blocking round trips to a Pack process**, each with a 5 s limit, on the window's thread. A slow Pack froze the window for up to 15 s; one that timed out was killed, and every later click failed with "stdin is closed", with recovery only in the Mac menu bar. Pocket Universe's narrator waited 30 s inside a turn the host cut off at 5 s. The app already carried Tiny Society compiled in, as a fallback.
2. **Off the Mac, half the app was missing.** Import World, Install World Pack, Reload from Disk and going back to Home existed only in the macOS menu bar, and nothing opened a file named on the command line, so a double-click on Windows did nothing.
3. **Saves failed on ordinary disks.** Save As and Export published only by hard link (FAT, exFAT and many network folders cannot), refused a file the save dialog had already agreed to replace, and an emptied file was called "from a preview version" with no Restore offered. The `.bak` was one save old; "St. Ives" became `St.world`; non-ASCII names became dashes.
4. **A new player was not told the loop.** Help did not mention talking, making, favours, Goodnight or Play freely, and mentioned a pinch that does not exist. The title said Day 7 where My Worlds said Day 6. "Waits for you" showed while playing, the day's card had a "Choose" button, and Home said "durable round-trip".
5. **200% text broke the layout:** Settings wrapped "100%" a character a line, the title box covered Continue, and the toolbar's glyphs shrank to blank pills.
6. **Choosing a thing to make built it at once.** A click on a tile in the Make panel also reached the building behind it, which took the click as "put it here": the bench went up at once, half off screen, without asking where.
7. **Smaller:** the Packs screen hashed the Pack program on every redraw; Settings started `defaults` twice on every redraw; a lost sound device silenced the session for good; Photo and moment strips failed silently; Enter in the talk box could send words still being composed; the save-failure dialogs were English only.

## What v1.0.0 does about it

All of the above, as the [changelog](../CHANGELOG.md) lists, each checked again in the real window: New opens at once, the menu has My Worlds and Import, the bench waits for a spot and stands where the quay was clicked, and Settings, the title and the toolbar hold together at 200% in Chinese. The one structural change is that Tiny Society runs inside the app: the World compiled into the binary always wins, and the bundled Pack is no longer installed or launched.

## Left for later, deliberately

- **A turn still runs on the window's thread.** In-process it is milliseconds (about 4 ms, plus 19–37 ms for the first look, on the measuring machine), not a round trip; moving it off the thread is a larger change to the window than this release should carry.
- **Onboarding beyond Help.** The gentle pointers exist; a first-session guide that walks the loop is a design task.
- **Props at their own scale.** The bench still reads large beside the people; that is the art direction's scale work.
- **Everything outside the code:** signing, Steam, real devices, strangers, native readers. See [KNOWN_ISSUES](KNOWN_ISSUES.md).
