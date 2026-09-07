# Privacy

World Machine runs entirely on your Mac.

## What stays local

- **Your Worlds** are `.world` files in `~/Library/Application Support/World Machine/Worlds`. Installed World Packs live next to them under `Packs`. Nothing is uploaded.
- **The log** at `~/Library/Logs/World Machine/world-machine.log` records what the app observed: startup facts, error messages you also saw on screen, and crashes. It rotates at 1 MiB and is never sent anywhere by the app.
- **Diagnostics** (**Help → Copy Diagnostics**) put the build label, your macOS version, the paths above, the names of included Packs, and the last lines of the log on your clipboard. You decide where to paste it.

## What the app never does

- No account, no sign-in, no telemetry, no crash reporting service, no update check. The only network activity the app initiates is opening the GitHub pages for the install guide and the issue template in your browser when you ask.

## The optional World Analyst

The World Analyst is an experimental feature that talks to an AI model through the Pi runtime. It is off by default and its entry only appears when Node and Pi are installed. When you use it, the World data you ask about is sent to whichever model provider your Pi configuration points at, under that provider's terms. World Machine itself never bundles Pi or a model key. Details are in [PI_ANALYST.md](PI_ANALYST.md).
