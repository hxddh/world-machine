# Art direction: a lit paper-and-paint diorama

This is the art bible for World Machine's places. The code is the brush, and these are the rules it paints by. Every rule is meant to be testable, either by a test or by a contact sheet the art director signs off.

## The picture we are making

A small place seen as a diorama: cut paper and gouache on a table, lit by a window. It should feel calm, legible and warm, and it should change because of the player.
- **Reference points:**
  - Townscaper: one continuous, sculpted place.
  - Tiny Glade: paths and walls that grow out of use.
  - Animal Crossing: every object at human scale, beside a person.
  - The dusk and night of our own v0.25: windows that tell you who is home.
- **A screenshot should read in one second as:**
  - a place;
  - where people are;
  - what changed.

## 1. A place, not a catalogue

**A spine.** Each place has one spine running across the lower-middle band, and people stand and walk on it:
- Tiny Society: the quay road along the water.
- Maple Street: the street.
- Ares: the airlock path.
- Icebridge: the ice causeway.

**Clusters.** Works and homes join the spine in clusters, never one by one on a lawn.
- A cluster holds 3–6 things that share a ground patch: cobbles, a garden, a yard, a plaza or packed snow.
- Each cluster has a name in the code (the Harbour Front, the Green, Chapel Hill…), so the story can say "on the Green".
- Clusters join the spine by paths. A path is a soft painted strip, worn lighter where it is walked.

**Ground between clusters** is intended, never leftover lawn: fields with rows, hedgerows, kitchen gardens, drying greens, rock and tussock at the edges. An empty screen-width of plain meadow is a defect.

**The ground is one painted wash.**
- It varies softly in tone, with no tile edges and no visible joins between bands or panels.
- A seam a player can see is a defect.

## 2. Everything stands where it belongs

The siting zones, from water to sky:

| Zone | What may stand there |
|---|---|
| Water line | Pier, jetty, slipway, boathouse, moorings, boats, the lighthouse on the point |
| Quay | Stalls, carts, crates, nets, benches facing the water, lamp posts |
| Lanes | Homes, shops, the pub, the school, gardens |
| Green | Fair, maypole, bandstand, well, fountain, benches facing each other |
| Edges and hill | Windmill, telescope, chapel, orchard, beehives, lookout |
| Back row | Distant silhouettes only |

- **The back row** is the same drawings as the front, smaller, lighter and cooler: atmospheric perspective toward the sky colour. Never use flat blocks or shapes that read as something else (an arch on a hilltop reads as a bridge to nowhere).
- **Water works are never off the water.** No pier on a hill, no jetty in a meadow.
- **The Pocket Universe places** map the same zones onto their own ground:

| Zone | Ares | Maple Street | Icebridge |
|---|---|---|---|
| Water line | crater rim | kerb | floe edge |
| Quay | airlock apron | pavement | the causeway |
| Lanes | habitat ring | houses | igloo rows |
| Green | dome garden | park | ice plaza |
| Edges | ridge | hill | — |

## 3. One scale for everything

Heights are given in **P**, the height of a standing adult resident.

| Thing | Height |
|---|---|
| Child | 0.65 P |
| Bench seat / back | 0.45 / 0.8 P |
| Handcart | 0.7 P |
| Postbox | 0.75 P |
| Well, fountain | 1.0 P |
| Door | 1.15 P |
| Telescope on its tripod | 1.3 P |
| Market stall | 1.3 P |
| Lamp post | 1.8 P |
| Cottage (eaves / ridge) | 1.8 / 2.75 P |
| Shop | 3.0 P |
| Pub | 3.3 P |
| Maypole | 3.5 P |
| Windmill | 5 P |
| Lighthouse | 6 P |
| Rowing boat (hull) | 0.5 P |

- **Distance shrinks things:** the second band is ×0.75, the third ×0.55 and the back row ×0.4 of the front.
- **Footprints follow heights:** a cottage is about 2.2 P wide and a stall about 1.4 P.

## 4. Value, colour and light

**Three value bands.**
- The sky is lightest, the land is mid, the water is darkest.
- People, doors and windows are the darkest accents on the land, so the eye finds them.
- Nothing on the land is lighter than the sky by day.

**Each place has a palette** of five base colours and two accent hues:
- Tiny Society: sea-green, sand, slate blue, warm stone, harbour-red and lamp-gold.
- Ares: rust, dust, bone, teal and signal-white.
- Maple Street: brick, asphalt, lawn, teal and magenta.
- Icebridge: snow, ice-blue, slate, fish-orange and lantern-gold.

**An accent budget.**
- At most three saturated accent marks per screen-width: flags, the postbox, awnings, a quilt.
- By day the player's designs are shown 15–20% toward the base palette, so they sit in the place. At night they glow in lit windows.

**The key light** comes from the upper left.
- Shadows are cool and always attached: every standing thing has a soft contact shadow (an ellipse, 20–30% opacity, blurred) on the ground beneath it.
- Nothing floats.

**Time is light:**
- **Dawn:** a pink sky and long blue shadows.
- **Noon:** neutral, with short shadows.
- **Dusk:** gold, long warm shadows, the first lamps lit.
- **Night:** warm windows with silhouettes in them, lamps along the spine, and their reflections broken on the water. Night is our best picture; spend detail there.

**Weather** never greys the whole picture.
- Rain is fine, sparse streaks, plus darker wet ground near the water.
- Snow settles on roofs.

## 5. Paper and paint

- Outlines wobble slightly and fills sit a little off their line, as built in v0.25. Keep it subtle: the wobble must never read as a mistake.
- One paper grain lies over everything, at one scale.
- Edges are soft where paint meets paint and crisp where paper is cut (roofs, signs).
- No gradient banding, and no pure black or pure white anywhere.

## 6. Composing each screen

Each screen is composed from front to back:
1. **A foreground framing strip** at the bottom of the land band: grass tufts, a fence, flowers, the quay edge.
2. **One focal cluster** near a third line.
3. **Mid-ground clusters**, smaller.
4. **The back row.**
5. **Sky,** at least 25% of the frame.

**People:**
- People gather in twos and threes, facing each other, on the spine, porches and benches.
- Never more than four people stand in a row.
- Someone is always in view on the first screen.

**Overlays:**
- A card never covers the person it speaks for.
- Name labels appear on hover, or for the person speaking; they never stack.
- Nothing is labelled before it is painted.

## 7. What the player did is visible

- A build shows scaffolding within a second of the answer, and the work appears on its own site, never on top of an existing building.
- A story that happens leaves a mark: a burnt chimney patched, bunting after a party, a new boat after a launch, a closed shutter after a quarrel. Marks last days, not forever.
- The first screen changes often in the first two weeks: something new on at least 8 of the first 14 days.

## 8. Key art

The key art is rendered by the engine itself, from a fixed seed, at 3840×2160.
- **Scene:** the harbour at dusk in its second year.
- **Composition:**
  - the lighthouse on the point on the right third;
  - the quay curving in from the lower left;
  - the pub lit, bunting across the lane, two boats, the first lamps lit;
  - a group of three residents talking on the quay;
  - clear sky in the upper left for the title.
- **Pocket Universe** gets a key art per place, from the same rules.

## Sign-off

Each release that changes the look ships a contact sheet: 12 frames covering each place at noon, dusk and night, day 1 and year three, and the first screen. The art director checks every rule above against it.

From v0.29 the sheet comes from scripted free play, not frames chosen by the team:
- it covers day 1's first 90 seconds, days 5, 13 in rain and 21 at dusk, a return film, Find, Esc, each Pocket Universe place opening, and Chinese and Japanese;
- it includes frames 0.3, 1.0 and 2.5 s after every camera move, not only settled ones;
- the harness picks at least a third of the frames at random;
- every store-page shot must match the frame the player sees at that moment.

(The v0.28 sign-off below was given on settled, harness-picked frames, and was withdrawn in the [v0.28 review](REVIEW_v0.28.md#what-the-art-director-found).)

## v0.27 sign-off

Signed off by the art director on 2026-10-07 after three rounds.
- Key art: [harbour](art/v0.27-keyart-harbour.jpg), [Ares](art/v0.27-keyart-ares.jpg), [Maple Street](art/v0.27-keyart-maple.jpg), [Icebridge](art/v0.27-keyart-icebridge.jpg). Re-render at 3840×2160 with `cargo run -p world-gpui --example key_art -- <dir>`.
- [Contact sheet](art/v0.27-contact-sheet.jpg): the harbour on day 1 and in year three, each at noon, dusk and night; the three Pocket Universe places at noon and at night.
- Accepted deviations:
  - The Point is the east end of the stage, so in the harbour key art the lighthouse sits at about 0.76 of the width. That is inside the right third, not on the line.
  - In year three, three residents by the lifeboat station overlap its front and read as standing on it. Fix in v0.28.

## v0.28 sign-off

Signed off by the art director on 2026-10-08 after three rounds, on real-window screenshots from a release build (`scripts/release-shots.sh`). **Withdrawn on review** (2026-10-08): in free play the rough stand-in stays on the subject of a moment for about 2.5 s, each Pocket Universe place opens on the harbour's meadow, and the lighthouse's shadow box is still there; see the [v0.28 review](REVIEW_v0.28.md#what-the-art-director-found).
- [Contact sheet](art/v0.28-contact-sheet.jpg): the harbour on day 1, in year two and zoomed in at year three, at dusk and at night; Ares at noon and night; Maple Street at noon, dusk and night; Icebridge at noon and night. The v0.27 key art stands.
- Round 1 was not signed off: fog in the first one to two seconds, Maple Street empty at night, a home cut at the edge on day 1, and people overlapping the quay's buildings in year two. Round 2 fixed all four. Round 3 fixed a regression round 2 showed: a hard-edged dark slab at the lighthouse's base and under the Anchor Pub, where the rough painting showed through a fading sharp picture and shadows were drawn twice during a look change. Tests now hold that the rough never shows where a sharp picture is, and that a look change never darkens a frame.
- Accepted deviations, to fix in v0.29:
  - Each Pocket Universe place opens on its one keeper beside a whole building; the four-residents rule is waived there, since the places start with one keeper.
  - The year-two first screen cuts a building at its left edge.
  - A speech bubble can leave one short word alone on a line ("the"); the bubble's width should be balanced.
  - During the welcome the buildings read slightly softer than on day 1, though they are drawn 1:1. Cause not found.
  - The frame bar (8 ms in 10 of 10 runs) is not met locally; this is a measurement, not a look, but a hitch on a pan is seen.
