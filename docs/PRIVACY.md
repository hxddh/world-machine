# Privacy

World Machine runs entirely on your Mac.

## What stays local

- **Your Worlds** are `.world` files in `~/Library/Application Support/World Machine/Worlds`. Installed World Packs live next to them under `Packs`. Nothing is uploaded.
- **The log** at `~/Library/Logs/World Machine/world-machine.log` records what the app observed: startup facts, error messages you also saw on screen, and crashes. It rotates at 1 MiB and is never sent anywhere by the app.
- **Diagnostics** (**Help → Copy Diagnostics**) put the build label, your macOS version, the paths above, the names of included Packs, and the last lines of the log on your clipboard. You decide where to paste it.

## The update check

Once per launch the app asks `api.github.com` for the latest release, the same request a browser makes when it opens the Releases page. GitHub sees your IP address and the app's version string, nothing else, and the app only reads back the version and the download page. A newer stable release shows as a banner on Home with Download and Later. To turn the check off, launch with the environment variable `WORLD_MACHINE_NO_UPDATE_CHECK=1`.

## What the app never does

- No account, no sign-in, no telemetry, no crash reporting service, no automatic download. Beyond the update check above and the optional voice described below — off until you turn it on — the only network activity the app initiates is opening the GitHub pages for the install guide, the Releases page, and the issue template in your browser when you ask. The issue template arrives with the build label and macOS version filled in; you see and can edit both before submitting.

## The optional World voice

A World reads from copy written into the app. Turning **World voice** on in
**Settings** changes that: when you come back to a World, what it tells you
happened is written by a model instead. It is off by default, and there are two
ways to switch it on, which differ in exactly the way that matters here.

- **A local program you already have.** The app starts the program you chose
  in **Settings**. Where that program sends anything is
  between you and it; World Machine bundles no model and no key.
- **An API key you give the app**, entered in **Settings** and kept in your login keychain rather than in any file this app writes — a backup of your Worlds folder never carries one, and you can inspect or delete it yourself in Keychain Access under "World Machine · World voice". This is the only case where World Machine
  itself sends your World's contents anywhere. Requests go to `api.anthropic.com`:
  - when you come back to a World, one request carrying the facts that World has
    already recorded (its seed, which era it is, what just happened, and the
    built-in sentence describing it);
  - when you talk to someone with the voice on, one request carrying what you
    said and what that person knows from the World (how they are, the people and
    places there, recent news), and, if the second opinion is on (it is by
    default when a key is stored), one more carrying the same facts and the
    answer, to check it.

  Nothing else is sent: not your other Worlds, not your library, not your file
  names, not the log. Nothing is sent per period. Anthropic sees these requests
  under their terms. Turn the switch off and no request is ever made. The
  second opinion can also use this Mac's own model, which sends nothing, or be
  turned off in Settings.

Whichever you choose, the model is only ever asked to put what the World
already holds into words, and a resident's answer is only a proposal. It cannot decide what happens in a World, and a reply that
does not arrive, or does not make sense, leaves the World reading exactly as it
does with no voice at all.
