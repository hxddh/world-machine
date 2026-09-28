# Changelog

Releases live on the [Releases page](https://github.com/hxddh/world-machine/releases). `0.2.0` is the first release intended to be usable without reading the repository; the `v0.1.0-pre.N` tags below were its pre-releases.

## v0.17.0 (2026-09-28)

**Worlds from `v0.16.0` open and carry on.** For the first time a release keeps every World the last one saved: Tiny Society moves to `0.11.0` and Pocket Universe to `0.28.0`, and each opens its `0.10.0` and `0.27.0` Worlds with their whole history and goes on under the new rules. From now on no release may leave a World behind.

The [v0.16 review](docs/REVIEW_v0.16.md) found `v0.16.0`'s craft in the wrong place and short-lived: a first launch opened the plainest World, more than half of a month's days brought nothing new, and eight of the last ten releases had thrown players' Worlds away. This release turns the showcase into a World you keep. Everything in it is still made by the program.

- **You begin in Tiny Society.** A first launch opens the harbour, the World with the most craft. Pocket Universe's places are shown to choose from as they will first stand, people, buildings and all, drawn the way the window draws them, instead of bare hills.
- **Something new every day.** On any day that brought you nothing to keep and nobody new, someone you have not met comes over to say hello, or, once you know everyone, someone writes to you: their first letter is a keepsake of its own, later ones bring something small, in their own words of the day. While you are away you come back to a letter. Both Packs are tested to bring something new on each of the first 30 days, with at least four keepsakes, to a player who only answers questions. Each Pocket Universe place gains a mid-year festival so a dated day comes at least every two weeks.
- **Never lose a World.** A Pack version names the earlier versions it carries forward; the host, Home and a Pack process all open such a World with it. Its history is kept exactly as recorded, since every Event already holds what it changed, so nothing is decided again. Worlds saved by the real `v0.16.0` are kept as fixtures, and a test opens each, plays 30 more days and checks nothing was lost.
- **A finished surface.** Soft contact shading under every building, thing and person; the foot of each wall darkened; a fine ink rim on each layer of land; a haze over the ridge; a colour grade that eases hour by hour (every hour its own); people standing at three depths, the nearer ones larger; grass, flowers, stones, a fence and a worn path in front of them, and reeds at the water; and scaffolding rising on what the place is still building, instead of a grey box.
- **Portraits drawn as the scene draws them.** A question's card shows the person's own drawing (a bearded Jonas in his sou'wester, not a generic face), with their mood, and a mouth that moves while they ask.
- **Music with memory.** Every World has a tune of its own, made from its colours, which the day plays in phrases (stated, answered, turned about, stated again, ending home) and the evening hums slowly. A festival day brings a quicker step, bells and a soft tap on every beat. Whoever asks you something is announced by three notes of their own.
- **Pocket Universe at parity.** Its six core residents (Nia and Tomas on Mars, Lena and Max on Maple Street, Piko and Miri on the ice) each have a style sheet, 162 to 200 lines of their own, and five friendship moments and a keepsake that are theirs alone. Pocket Universe is shown in Chinese: PU_SHARE of what a year of its places shows is fully in Chinese; a test holds it at 95%.
- **Lighter snapshots.** A snapshot carries only the voices a screen can show, not every one a World has kept: a year-old Tiny Society snapshot went from 19 ms to 14 ms on the machine this was built on.
- **Pack protocol:** a choice may carry a `preview` of the World it starts, a descriptor `carries_forward` earlier versions, and a calendar says whether a festival is `festival_today`; all optional, so older Packs and apps send and ignore none.

## v0.16.0 (2026-09-28)

**Worlds from `v0.15.0` do not open.** Tiny Society moves to `0.10.0` and Pocket Universe to `0.27.0`, because friendships, building and the start of a World all changed. Carrying older Worlds forward is out of scope by decision; start a new one.

`v0.15.0` had a World that was alive underneath and thin on the surface: flat people, 25 frames a second, no music, everyone confiding the same secrets. The [v0.15 review](docs/REVIEW_v0.15.md) scored every part of the product against the best cozy living-world games and planned two craft releases. This release does both, and everything in it that would usually need an artist, a composer or a writer is made by the program itself: people are drawn from code, the music is synthesised from each World's colours, and the lines, scenes and Chinese translation were generated and are held by tests.

- **It moves like a living thing.** In front, a World is drawn at the display's own rate (60, or 120 on ProMotion), and not at all behind other windows; with Reduce Motion on it drops back to a few frames a second. A busy frame's drawing work is under 4 ms in a benchmark, and a lint keeps every move eased.
  - Everyone has idle behaviours between walks: looking around, stretching, sitting, a task at their work.
  - Click someone and they wave and hop; click a building or a thing and it bounces.
- **People you can read.** Each resident is drawn from a generated outline of their own (hair, hat, build, coat; no two share one), with a face that shows how they are: content, happy, sad, cross, or thinking while they ask you something. Three layers of hills and cloud move at different speeds as the camera pans, and a light grade warms the day and cools the night.
- **Music that tells the time.** Each World's music is made from its colours: soft chords always, a plucked day tune that rises with the morning, a slow evening tune as the light goes, each of the day's 24 hours a mix of its own and the weather thinning it. Everyone who speaks babbles in their own voice, pitch and timbre their own, rising at a question. Every small sound comes in three versions played in turn. Settings has separate levels for music, landscape, voices and ticks.
- **The first minute.** Someone comes over and says hello as a World begins. Your first thing made earns a keepsake from whoever saw it, shown for a moment before it goes into the drawer. Speech shows at most two lines at a time, a long line said a page at a time, and no card is longer than two lines. A timed first session checks a greeting within 20 seconds, the first choice within 60 and the first keepsake within 5 minutes, in both Packs.
- **A voice for everyone.** Tiny Society's eight core residents each have a style sheet (the words they reach for, the ones they never use, how they open and close a line) and between 165 and 236 lines of their own. A friendship now opens five moments that are each person's own: a first warmth they share unasked, a secret, an invitation somewhere of theirs, a favour, and a keepsake. Everyone else, in both Packs, takes their five moments and keepsake from their two traits. A moment that has waited grows more pressing, so a close friendship's five come in weeks.
- **World voice keeps to what a person knows.** A model's answer is declined if it speaks as a machine or of its instructions, names something from the world outside (even when you bring it up), names someone or somewhere the person was never told of, or is not plain speech. The World's own answer stands instead, and the record says it was declined. 200 prompts written to pull the voice out of the World, answered by a model that goes along with every one, are all declined; answers that keep to the World are taken.
- **Build anywhere, and it matters.** Twenty things to make in Tiny Society and in each Pocket Universe place, eight of them new shapes (a well, a swing, a fountain, a signpost, a birdhouse, a planter, a statue, a postbox). Click open ground while placing and it stands right there. Take back what you just made or moved the same day (the pill, or ⌘Z), with what it cost returned. Benches get sat on, lamps and wells gather people of an evening, and a grown garden sends you something from it to keep. **Photo** saves the scene to Pictures.
- **A book of everything.** The drawer keeps a book of keepsakes, people met, things made and festival days: at least sixty entries in Tiny Society and in every Pocket Universe place, found ones drawn in colour, the rest a silhouette with a hint.
- **Goals you can finish.** The pier and the lamp are civic works the harbour fund pays for, so a player who says yes to what the harbour can afford finishes both within a year. Pocket Universe's goals are finished by such a player in every place.
- **In Chinese.** The app, the Systems and Tiny Society can be shown in Simplified Chinese: about 4,600 catalog lines plus every line the core residents can say, matched a sentence at a time with names and things filled in. Over a year of play, 97% of what Tiny Society shows is fully in Chinese; a test holds it at 95%. **Settings → Display** chooses the language (or follows the Mac), text size from 100% to 200%, and more contrast (or follows the Mac's Increase Contrast).
- **Keyboard and VoiceOver.** H opens your hands, P takes a photo, ⌘Z takes back, ⌘I opens the drawer, arrows and Return work the cards; the controls on the scene, and every person and place, carry VoiceOver labels.
- **Fixed:** a birthday question that came round again said "{name}" instead of whose birthday it was.
- **Pack protocol:** a snapshot may carry a `book`, a canvas item a `spot` along the ground and a `mood`, and a drawing part the `moods` it shows in; all optional, so an older Pack sends none. Eight new mark shapes cross the wire; an older app draws them as a house.

## v0.15.0 (2026-09-27)

**Worlds from `v0.14.0` do not open.** Tiny Society moves to `0.9.0` and Pocket Universe to `0.26.0`, because friendships now open doors, people remember what you did, and a new World begins differently. Carrying older Worlds forward is out of scope by decision; start a new one.

In `v0.14.0` people remembered what you said, but nothing you built up with them paid off. Standing was a mark in a card that nothing read, nobody remembered what you had done for them, and a question came round again in exactly the same words. This release learns from *Stardew Valley*, *Animal Crossing*, *Crusader Kings III*, *Dwarf Fortress*, *Wildermyth*, *RimWorld*, *Neko Atsume* and *Townscaper*; the review is in [docs/REVIEW_v0.14.md](docs/REVIEW_v0.14.md).

- **A friendship opens doors.** Once someone likes you, they tell you something they have told nobody. Once they think the world of you, they offer you a favour (a good word with a friend, or help for whoever is struggling), and a little later give you something to keep. Each happens once per person in a World.
- **A grudge closes them.** Someone who has turned against you stops asking you for help, and lets it show: "Oh. It's you." Saying sorry mends some of it.
- **People remember what you did.** The first time someone sees you on a day, they bring up what you did for them, not only what you said:
  - an answer you gave them: "When I asked you yesterday, you said “Pay Evan to mend it”. The children can hear themselves think now.";
  - something you made that they saw: "The bench you built by Harbor yesterday. I use it most days.";
  - a present, an evening out, a secret you kept, a day off you gave them.
- **Someone says what they make of what you make.** Build, plant or put something up, and whoever is nearby says so on the spot.
- **What you made is yours.** Festivals are told with it ("…, around the bench you made"), and a chapter's ending says what you built that chapter.
- **A question never comes round in the same words.** The second time, and every time after, it opens differently and says how it went last time: "Here we are again. Last time: Evan mended the school roof. The roof's dripping on the desks again." Questions that have never come up are favoured, so nothing in the deck waits behind the rest for long.
- **Every return brings a keepsake.** Come back after a period or more and someone who thinks well of you has left you something (a note under the door, a sketch, a photograph) with a line about what happened while you were away. The return ends on it, and the drawer keeps every keepsake you have been given.
- **A new World opens on the place.** No question waits on top of the scene. Build something first; someone says what they make of it, and the first question comes straight after. If you do nothing, the first question comes when the first period passes.
- **Measured.** Tests play a friendship up and a grudge down with one person in each Pack and every seed, and check:
  - each door opening once, in turn, and the keepsake reaching the drawer;
  - nobody with a grudge asking for help, and the cold shoulder showing;
  - an answer and a deed brought up within three periods;
  - what you made named in a festival and a chapter's ending;
  - every return of one to seven periods ending on a keepsake;
  - no question asked in the same words twice running, over a year, three ways of playing;
  - for a player who answers, nothing that could come up waiting more than 60 days (the calendar's days aside).
- **Pack protocol:** a snapshot may carry `keepsakes`. It is optional, so a Pack that sends none shows an empty drawer.

## v0.14.0 (2026-09-27)

**Worlds from `v0.13.0` do not open.** Tiny Society moves to `0.8.0` and Pocket Universe to `0.25.0`, because their people now hear and remember far more and their festivals remember earlier years. Carrying older Worlds forward is out of scope by decision; start a new one.

In `v0.13.0` you could talk to anyone, but they understood only about half of what people ordinarily type, and remembered none of it the next day. With World voice on, the window froze while the model thought. This release is about being understood and remembered.

- **People understand far more.** The conversation System hears 28 kinds of thing instead of 16:
  - how someone's day has been, and who they are;
  - their family, what worries them, and the weather;
  - what is coming up, by name ("Are you going to Lantern Night?");
  - a present, who their friends are, and what is wrong between them and someone else;
  - an invitation for a drink or a walk, what they think of you, and small talk ("ok", "haha").

  Names are recognised in Chinese as well as English (诺亚, 玛拉, 面包店), and places by other names ("harbour" for Harbor).
- **What someone says about a person comes from what happened between them.** Ask Mara about Leo and she says what she thinks and when they last fell out or made up. Ask why she is angry with him and she tells you in her own words.
- **People remember what you said.** The first time someone sees you on a day, they bring up what you said before:
  - "Thank you again for the flowers yesterday."
  - "You asked after Leo the other day. Leo's worried about money."
  - "I did talk to Noah, like you said."
  - "I haven't forgotten what you said yesterday."

  A present warms someone, and an invitation eases loneliness if they accept it.
- **See where you stand with someone.** Their card shows five marks and a few words: "Getting to know you", "Likes you", "Thinks the world of you", "Hurt by what you said". Ask "What do you think of me?" and they tell you.
- **"How are you?" is said differently from day to day.** The same state of things is put several ways, and an answer no longer repeats the same person twice.
- **World voice never freezes the window.** With the voice on, the app asks the model itself, off the window's thread: the person's card shows your words and a thinking "…" while the scene keeps moving. After 12 seconds, or if the model has nothing usable, the World answers in its own words. If you take a turn or branch while they think, the answer is dropped rather than recorded in a World that has moved on. The model's answer is still only a proposal that the World's rules check.
- **Festivals remember last year.** The second time a festival comes round, it is told against the first: "The whole harbour turned out for Spring Clean, bigger than last year, with 2 new faces among them."
- **Drawn with each place's own pictures.** Flags, bunting, lanterns, stalls and tents put up for a festival are drawn by each Pack in its own colours: harbour red and navy, Mars orange, Maple Street neon, Icebridge ice blue. Home's covers draw a World's people and buildings with its own drawings.
- **Measured.** A corpus of 428 everyday phrases in English and Chinese is heard without a miss by the System and by both Packs, in every seed. On three sets of new phrases written before any tuning for them, 27%, 25% and 45% were not understood and up to 13% misheard, before they were added to the corpus. Other tests check:
  - people remembering what they were told;
  - the standing moving with what is said;
  - no clause repeating within an answer, and "How are you?" not opening the same way on more than half of 30 days;
  - a model that takes too long not being waited for;
  - an app asking the model itself while the World still decides;
  - festivals told differently in their second year.
- **Pack protocol v3:**
  - `say` may carry `ears`: the World's own hearing, a model's response the app already has, or the World's own rules only.
  - A new request, `hear`, returns the prompt a model should be asked, and changes nothing.
  - The app sends `hear` and `ears` only to a Pack whose manifest says v3; a v1 or v2 Pack hears as before.
  - Canvas items may carry a `standing`.

  Everything new is optional. A World file keeps its cast's drawings for its cover.

## v0.13.0 (2026-09-27)

**Worlds from `v0.12.0` need exporting first.** Tiny Society moves to `0.7.0` and Pocket Universe to `0.24.0`, because their people can now be spoken to, their year is longer and has festivals, and they draw with their own pictures. A World made with the older Packs reports that it needs a Pack this build does not have; export it before updating to keep its history.

`v0.12.0` gave people lives and the player hands, but you could still only ask three fixed questions, a year was forty days of four ten-day seasons, and everyone was drawn from the same few shapes. This release lets you talk to the place and gives it a year and a look of its own.

- **Talk to people in your own words.** Click someone and type whatever you like: "How's the bakery?", "What do you think of Leo?", "You should make up with Noah", "Your bread is wonderful", "对不起". They answer in their own voice from how their life stands: their day, their friends and grudges, their worries, the news, what they need. What you say can move people: a kindness warms them once a day, a sharp word is remembered until you apologise, company eases someone lonely, and advice to make up with someone is taken by someone who trusts you, or refused. The conversation shows in their card and over their head, a need can be granted with one click, and no time passes. Every exchange is recorded like any other change, so a World replays without hearing anything again.
- **World voice speaks for people.** With the World voice switched on in Settings, people answer in a language model's words, through the local program or the API key already set up there. The model is told how the person's life stands and only proposes what they meant and what they say: a meaning outside the closed set, a name nobody has or anything that is not plain words is replaced by the World's own answer, and the rules still decide what the words do. Without the voice, people answer in the Pack's own words and nothing leaves the Mac.
- **A year with a shape.** Seasons last a month, so a year is 120 days. Each place has about fifteen days on its calendar: festivals people get ready for days ahead, and days that simply arrive. The harbour has its Spring Clean, Blossom Walk, Boat Blessing, Midsummer Fair, Lantern Night, Harvest Home, Apple Pressing, Bonfire Night, school play, Midwinter Feast and Year's End Swim, and the swallows, mackerel, geese and first frost. Mars, Maple Street and Icebridge each have a year of their own, from Landing Day to Halloween to the Aurora Festival. How a festival goes depends on the place: its spirits and what you have put up. Something goes up for the day, people mix, and the harvest is as good as what was planted. The drawer shows the season and what is coming up, and people mention it when asked for news.
- **Every place drawn as itself.** Packs now ship their own drawings. The harbour has a striped lighthouse, a bakery with an awning and loaves in the window, a timbered pub with a hanging sign, and a school with a bell and a clock. Jonas wears a sou'wester and a beard, Mara a baker's hat and apron, Noah a flat cap, Emma glasses and a bun. Mars has a dome habitat and a hydroponics greenhouse, Maple Street a neon arcade and a radio station with its mast, and Icebridge an arch of ice, a Fish Vault igloo, a council hall and penguins in scarves. People stand, walk, work with their hands, gesture while they talk and throw their arms up at a festival.
- **Light and a camera.** Buildings and people cast shadows away from the sun, long at dawn and dusk and short at noon, and lit windows glow after dark. The camera moves in on whoever you talk to, the scroll wheel zooms in on the place around the pointer, and Escape pulls back out.
- **Instant at any age, now for snapshots too.** A year-old World's snapshot takes about 9 ms in a release build, down from 20. Detail panels read an index kept as events are recorded, and History is told from the latest 1,200 events.
- **Measured:** new tests check each Pack:
  - People answer seven kinds of question differently in both Packs and every seed, and an exchange survives a reopen without passing time.
  - In a year, something is on the calendar in every fortnight and every season has at least three festivals.
  - Planting changes the harvest.
  - Every shipped drawing is drawable.

  The conversation System has its own tests for hearing, what words do, and what a model's proposal can and cannot change. The year-long, consequence and story-density tests from earlier releases still hold.
- **Pack protocol:**
  - A new intent, `say`, carries what the player says to someone.
  - Snapshots may carry `exchanges`, `drawings` and a calendar `season` and `coming`.
  - Canvas items may name a `drawing` and a `stance`.
  - Capabilities gain `talk`.

  Everything new is optional, and a Pack that sends none of it reads as before. Two new Systems sit outside `world-core`, `conversation` and `calendar`, and a new crate, `world-voice`, reaches a model for Packs that are given one.

## v0.12.0 (2026-09-27)

**Worlds from `v0.11.0` need exporting first.** Tiny Society moves to `0.6.0` and Pocket Universe to `0.23.0`, because their people now live their own lives and the player can build in them. A World made with the older Packs reports that it needs a Pack this build does not have; export it before updating to keep its history.

In `v0.11.0`, a harbour asked nothing new after about 75 days, and every chapter went round the same three climaxes. The player could only answer, and a year-old World took a quarter of a second a turn. This release turns a deck of cards into a world.

- **People have lives of their own.** Everyone has four needs (money, rest, company, purpose), two traits and an opinion of everyone else. Every period each person does something about the need pressing hardest, alone or with someone: a pint with a friend at the Anchor, a shift at the bakery, a walk on the cliff path, supper under the dome, a paper round before six, belly-sliding on the ice. What they do together moves how they feel about each other, and friendships, quarrels, couples and making up come of it. What people say over their heads is what they did that day, and nobody's words come round again for a month.
- **Situations come from how people stand.** Two people who have fallen out over a torn net, someone sweet on someone, someone who wants to learn a trade, is short of money, lonely or worn out, a couple to celebrate or in a rough patch, a stranger asking for a room, someone thinking of leaving. Each is made of who, what and over what, so a year keeps bringing new ones. Answers move people on the scene: sent home, brought to the pub, walking out together.
- **The place grows and changes.** Strangers come by and can stay: a fiddler from Galway, a geologist from Phobos, a guitarist off the Greyhound, a fisher from the far floe. People left alone long enough can leave. Pocket Universe's places hold up to eight, and Pocket Universe's own story of the pair stays theirs.
- **Your own hands.** The plus beside the drawer handle opens six things to do besides answering: build, decorate, plant, move, give and invite. Pick a bench, a lamp post, bunting or an apple tree, and the scene lights up where it can go; click a place to put it there. Plants grow through their stages over the seasons, decorations come down after a while, and a present or an invitation reaches the person it is for. Two deeds a period; the harbour's come out of its fund.
- **Weather and life on the scene.** A storm arrives as a storm: the sky and land darken, rain drives in and lightning flashes. Winter brings snow that lies, spring and autumn rain and fog, Mars its dust storms and Icebridge its blizzards, all from each World's own state. Chimneys smoke and gulls wheel over on a fair day. The scene also draws two new things, a bench and seedlings.
- **Chapters that never repeat.** A chapter's ending tells what changed between people ("Jonas and Noah fell out. Emma and Sofia became firm friends."), no two chapters of a World share a title, and only someone let down within the chapter is said to remember it. Everyone in the harbour has a birthday in their own words.
- **Instant at any age.** On a World that has lived a year, a turn with its previews takes 33 to 37 ms instead of about a second. Finding an event no longer reads the whole history, a snapshot describes its latest 400 events in full, previews run on a copy with only recent history, and a period passes on the World itself with a checkpoint to go back to.
- **Different branches become different places.** The World's mood colours everyone's days, and who turns up depends on how the place is regarded, so one different answer at day 10 leads to a different town 90 days later.
- **Measured:** each Pack now has year-long tests. Playing 365 periods three ways, each month from the fourth brings at least eight never-seen situations, no everyday line comes more than three times in 30 periods, how people stand keeps changing, no chapter title repeats, and everyone lives every period. Other tests check the six verbs, and that after a month a third of what stands is the player's. They also check that branches diverge and that weather follows storms and seasons. The story-density and consequence tests from `v0.10.0` and `v0.11.0` still hold.
- **Pack protocol:** commands may carry a `hand` (a deed: verb, thing, where, cost), and snapshots a `weather`. Both are optional, and a Pack that sends neither reads as before. The scene knows two new shapes, `bench` and `sprouts`. Two new Systems sit beside `storylets`, outside `world-core`: `lives` and `hands`.

## v0.11.0 (2026-09-26)

**Worlds from `v0.10.0` need exporting first.** Tiny Society moves to `0.5.0` and Pocket Universe to `0.22.0`, because what you choose now builds, opens and brings people into them. A World made with the older Packs reports that it needs a Pack this build does not have; export it before updating to keep its history.

In `v0.10.0`, whatever you chose, the harbour ended a month with the same fourteen things on its scene, and no question ever followed from an earlier answer. This release is about consequence.

- **A question is one card.** Whoever asks, what they ask ("The biggest storm in years is coming. What do we save?") and its answers are on the same card, like *Reigns*. ← and → lean toward an answer, ↑ and ↓ turn to another question, ⏎ chooses; pointing at an answer leans to it and clicking chooses it. An answer you cannot afford is shown greyed with the reason ("Noah hasn't 60 to spare"). The Pack protocol gains an optional `question` on a command, and `unavailable` with its reason; a Pack that sends neither reads as before.
- **What you choose appears.** Answers stand things up on the scene through each World's own state: Sofia's stall by the pub, bunting for the fête, the first sections of the pier, the lamp on the point, the school's garden, crates, nets, benches on the quay; on Mars the second dome, a beacon, a flag at the edge of the map, a keeper's garden, a tent. Some last, some come down after a while. The scene draws them because the recorded state says so.
- **Questions follow from answers.** Wants have second and third acts: a stall granted leads to "Sofia's stall is thriving", then to Leo finding the pub quiet; a stall refused brings an offer from the mainland. On Mars a seal fitted, a solo trip allowed or a signal answered each come back later. Over 30 periods a third of the questions follow from something you answered.
- **People arrive and leave.** A traveller can make the harbour her home, a fishing family can move in, and on Mars, Maple Street and Icebridge a newcomer arrives when invited. A thread can also send someone away: Sofia, refused her stall, can leave for the mainland.
- **Chapters with a shape.** Each chapter opens on a named pressure (the storm season, hard times, the inspector; the long dark, a breaking point, the call) and builds to a decisive question, and its ending tells how that went and which threads it closed ("The storm we boarded up against. Hammers all night, and it held. Sofia opened a stall of her own.").
- **A first minute and a fair return.** A new World opens on its first question instead of "Let the day pass". While you are away, wants wait for you instead of lapsing into grudges; only what cannot wait happens without you, at most three questions in a week.
- **A deeper deck.** Pocket Universe's deck grows from 17 storylets to the harbour's size, with its own threads and three climaxes, and nothing but the calendar is asked more than twice in 30 periods.
- **The pair and the harbour's spirits never pin.** Two people in Pocket Universe who are as easy with each other as they can be start to grate a little, and the harbour's spirits settle back from either end over a few days.
- **Measured:** each Pack now has consequence tests that play 30 periods always saying yes and always taking the last answer: at least half of all answers change the scene (Tiny Society 38 of 60, Pocket Universe 31 of 60), a third of the questions follow from an earlier answer, no question besides the calendar's comes up more than twice, the two ways of playing end with at least three different things on the scene, a new World opens on a question and a week away lapses at most three questions.

## v0.10.0 (2026-09-26)

**Worlds from `v0.9.0` need exporting first.** Tiny Society moves to `0.4.0` and Pocket Universe to `0.21.0`, because both now have a storyteller that changes what happens in them. A World made with the older Packs reports that it needs a Pack this build does not have; export it before updating to keep its history.

In `v0.9.0`, a harbour played for 30 days asked the player something on one of them, and on Mars trust and tension sat pinned at the ends for the last 15 sols. This release is about having a story every day.

- **A storyteller.** Each World has a director, in the style of RimWorld's. Every period it lets lapse what nobody answered, sees how the World stands, and sets up what comes next: a storm, a traveller wanting a room, a sick child, a hotel's bread order, or on Mars a dust front, a failing seal or a signal from the old lander. When a gauge sits at one end it reaches first for something that eases it. It runs as rules inside each Pack, is the same every time from the World's own state, and what it does is recorded, so a World replays without it. The pacing logic that belongs to no World (what is open, what has rested, chapters, goals) is a shared System, `systems/storylets`, outside `world-core`.
- **Everyone wants something.** Emma wants the school roof mended, Leo a music night, Sofia a stall of her own, Nia a spare seal, Tomas one trip past the edge of the map. Asking *What do you need?* gives their want, and granting it is a choice with a cost. They remember: granted wants make them glad, and a want turned down or left to lapse becomes a grudge they mention when asked how they are.
- **Every period brings a card.** Each day or sol offers something more than waiting, asked by someone and drawn with their face, and letting time pass is one card, said one way ("Let the day pass", "Let the sol pass"). Tiny Society can now be played turn by turn; it used to only move while you were away.
- **People remember, and don't repeat themselves.** Everyday lines come in fives and take turns, so nobody says the same thing twice in a working week. The day after something goes their way, people say so ("Dry desks at last!", "Still thinking about that traveller's stories.").
- **A calendar.** Four seasons of ten days each change the colours of the place: summer warmer, autumn's hills gold, winter pale and frosted. The harbour has market day every week, a regatta in summer, a harvest supper in autumn and coal for the school in winter; everyone has a birthday; Mars has its supply drop and launch window, Maple Street its county fair and Icebridge its great migration.
- **Chapters that end.** A World's story comes in chapters. Each ends on a card of its own that says how it went ("A bitter winter. The harbour kept its bakery. Jonas found his feet ashore. Mara hasn't forgotten being let down."), and the next begins from where it left the World. A gauge that reaches its very end turns the chapter. Every chapter so far is in a chapter book at the top of the drawer.
- **Goals you can see.** The new pier and a lamp on the point in the harbour; a second dome, a relay mast and the map past the edge on Mars. Each stands on the horizon as a pale outline with a pip for every part, filling in as parts are built, and in the drawer under *Building*. Pocket Universe's ridge no longer fills with an unnamed shape every sol: one period in three leaves a mark.
- **The pair never sit at an end.** In Pocket Universe, two people who are as easy with each other as they can be start to grate on each other a little, and a feud wears itself out. A new chapter starts nearer the middle than the last one ended.
- **Fixed:** a Pocket Universe World left alone long enough could fail to move on at all, when the World tried to choose its direction by itself before the pair's relationship had resolved.
- **Measured:** each Pack now has story-density tests that play 60 periods three ways (always saying yes, always taking the last answer, never answering) and check the bar the v0.9 review set: something to decide every period, no line said more than three times in any ten periods, no gauge at an end for more than five periods in a row, the first chapter closed within 30 periods, and the World replaying to the same place without the storyteller.
- **Pack protocol:** snapshots may carry `goals` and `chapters`. Both are optional on the wire, and a Pack that sends neither reads exactly as before.

## v0.9.0 (2026-09-25)

Every World from `v0.8.0` opens as it is: no Pack's rules moved. What people look like, what they say and what they answer is new, written by each Pack from what its World already records.

- **The World fills the window.** The scene is the whole window, and what you act on floats over it. The gauges are a strip over the sky, the moment ("Sol 5") sits beside them, and your turn is a card at the bottom. At rest a World window shows at most 40 words; v0.8 showed about 170.
- **People are drawn, and places are buildings.** Everyone is a small figure in the clothes of their work, carrying what they work with: Jonas in a yellow oilskin with a fish, Mara in her apron with a loaf. On Icebridge they are penguins in scarves. Places stand on the ground as what they are: a dome, a greenhouse, a lighthouse, a bakery with a striped awning. The rover and the boat are drawn as themselves. Names show when something is about someone, or when you point at them.
- **People talk.** Whoever something happened to says it in a speech bubble ("Could you spare something till I'm back at sea?"), and at the end of a day each person says a word about their work.
- **Life goes on between turns.** People walk over to another place and come home, the rover's lights and the windows glow after dusk, clouds drift and the harbour's water moves. Nobody wanders at night, or while they are speaking, asking or in the news. None of it is recorded.
- **Ask anyone.** Click someone to ask how they are, what they think of the other person or how the harbour is, and what they need. They answer in a bubble. When they ask for something, *Do it* makes that choice through the World's own rules.
- **A decision is a card.** One choice at a time, whoever asks it drawn large, and the gauges showing what it would move. ← and → leaf through the choices, ⏎ chooses, Space or *More* turns the card over for the whole account. Choices somebody asks come before letting time pass.
- **Coming back is a short film.** Each beat of a return moves the camera to where it happened, the people in it say their line, and it goes on by itself; *Skip* and *Next* are there if you want them. It ends by pulling back to your turn.
- **Home is covers.** Each World's cover is drawn from its own stage, people and buildings included, so branches no longer look alike. Rename, Export, Remove, What if… and the World a branch came from wait behind a ⋯ that appears when you point at a cover, and behind a right-click. Search and filters appear past nine Worlds.
- **The rest is one ⌘I away.** A closer look at whoever you picked, what happened, how things stand, who lives there and the whole history are in a drawer over the scene.
- **Motion and sound.** Gauges slide to where a turn leaves them, cards rise, people walk, and what gets built grows up out of the ground. With Sound on in Settings, turning a card ticks, a turn passing rings a soft bell, and something new being built plays two rising notes.

## v0.8.0 (2026-09-25)

Every World from `v0.7.0` opens as it is: no Pack moved. The new scene, gauges and choices are drawn from what each World already records.

- **The scene is a place, not a diagram.** People stand at the place they are at, a pair side by side, and walk over when a turn moves them. A relationship is a small bubble (a heart, a crack, three dots) between two faces, not a labelled line; a dotted line joins a pair only when they are apart. The rover, the night bus and the boat stand on the ground. Places never cover each other and no name is cut off. Partners go together; a pair who fell out keep apart.
- **Coming back is a moment.** A return tells what happened one thing at a time, big, with the face of whoever it happened to and the people it is about lit up on the scene ("While you were away · 1 of 2"), then hands over to your turn. Every beat is a recorded event.
- **The stakes are always on screen.** A row of gauges sits above the scene: trust and tension between the pair and how safe their home is in Pocket Universe; people in work, money in town and the bakery in Tiny Society. Hovering a choice lights the gauges it moves and shows where each would end up (▲, ▲▲, ▼). The marks are measured by making the choice on a copy of the World, never guessed.
- **A choice is someone asking.** Each choice shows the face of whoever it concerns (both, for the pair), one line, and the gauges it moves; the rest opens while you consider it. Letting time pass shows a clock.
- **Home is a shelf of living Worlds.** Each World is a large cover in its own landscape with what it has built on its ridge, a badge for how long it has been living without you ("5 sols have passed"), its age in its own unit ("Sol 5", not "time 50"), and its latest line cut at a word. Branches keep their colours. A first launch opens one window, and a World nothing has happened in yet is not listed.
- **Time you can see.** The sky follows your clock: warm at dawn and dusk, dark blue with stars at night. Optional ambient sound (Settings → Sound, off by default) plays a quiet loop made from the World's own landscape.
- **Nothing a player reads is in engine words.** Detail panels, why chains, headings and choices lost "Care Count", "Universe Grew", "latest at World time 105", "A larger choice is here" and the like, and a test now fails if any comes back.
- Opening a World from Home's cover opened it twice; moving the pointer from one choice straight to the next dropped the preview.

## v0.7.0 (2026-09-25)

**Pocket Universe Worlds created before this release will not open.** Pocket Universe moves to `0.20.0` because a turn now moves the World's clock. Export anything you want to keep before updating; Tiny Society, Future Archaeologist and Micro Company Worlds are unaffected.

- **A new World starts with a picture.** The first screen is three illustrated places to begin (a Mars colony on red dust, a 1987 street at dusk, an ice bridge under the aurora) instead of a form with three lines of text and two cards of instructions.
- **Every World looks like where it is.** A Pack can give a World its own colours: sky, ridges and sun. Pocket Universe gives each of its three places theirs and Tiny Society its harbour; the scene stands on that ground, and Home draws each World's cover in the same colours. Each place is drawn as what it is: a dome for the Mars habitat, a shopfront for the arcade, a bridge for Icebridge, a lighthouse for the harbour, where every place used to be the same green house.
- **A World is called by its name.** A new World is "A new World" until it begins and then takes its place's name ("Ares Pocket Colony", "Maple Street · 1987") in its window and on Home; it used to be "Pocket Universe · Empty World" forever. A World you rename keeps your name; the next turn used to overwrite it. The window no longer shows the file's id beside the name.
- **A World grows on screen.** Every time a World builds something (a water-recovery loop, a tournament bracket at the arcade, a span of the ice bridge) a small shape joins its horizon and stays: domes and masts on Mars, houses and street lamps on Maple Street, igloos and ice trees on Icebridge. After a week a colony looks lived in. Clicking one opens the moment it was built.
- **A World says it keeps going.** Beside its name, every World window says so and says when: "Keeps going without you · next sol in 6 h". Nothing in the app used to mention that a World moves on while it is closed.
- **Home offers only Worlds a person would play.** *Future Archaeologist* and *Micro Company* exercise the engine and read like it ("recover fragments … without exposing the hidden ground truth"); Home no longer offers them to start. Worlds already made with them still open, and `WORLD_MACHINE_DEVELOPER=1` offers them again.
- **The window's title bar keeps up with the page.** After a few turns it could still show the actions of an earlier moment, because it only redrew when something else made it.
- **Time is counted the World's own way.** A Mars colony counts sols, Maple Street nights, Icebridge auroras and the harbour days: the page, History, the activity strip and What if… say "Sol 3" and "Sol 1–5" instead of "Time 30" and "time 10–50".
- **A turn is a day passing.** "Let the first sol unfold" now moves the clock by one sol, exactly as a sol passing while you are away does, so History files each turn under its own sol instead of piling every turn of a session under *The beginning*. Pocket Universe moves to `0.20.0` for it; Worlds from `0.19.0` do not open.
- **Choices speak plainly.** "Create a goal that neither actor can complete alone; future interactions will lean toward trust" is now "Give them something neither can finish alone. From here on they lean toward trusting each other", and no choice asks to "let one more persistent change happen".
- **Branch and What if… appear when they mean something**: once a World has a history to branch, and once it has two choices to compare.
- **The Pack install review shows the Pack's picture.** It was drawn with no size and came out blank in `v0.6.0`.

## v0.6.0 (2026-09-25)

**Worlds created before this release will not open.** Tiny Society moves to `0.3.0` because it now records things it used to leave unrecorded, and Pocket Universe moves to `0.19.0` because what it writes into a World's history is now in plain words. A `.world` file records the Pack version it was created with. Export anything you want to keep before updating; Micro Company Worlds are unaffected.

- **Settings is a setting, not an essay.** World voice is a switch, and where the voice comes from is two tiles you pick between, each with the one fact that matters ("Stays on this Mac", "Each return is sent to your provider"). What is true now is one line with a coloured dot. It used to be five paragraphs around two small unlabeled toggles.
- **Installing a Pack reads like an install sheet.** The Pack's landscape, name and description, then version, what it runs as, a short fingerprint and the file it came from, one line on what trusting it means, and Cancel beside Install & Trust. It used to be a list of "Identity ·", "Format ·" and a full SHA-256 on a yellow background.
- **History tells what happened, not what the engine did.** Each line is the thing that happened, with the face of whoever it happened to ("Jonas started eating into his savings"), instead of "Agent Decision Recorded" or "Work Shift Completed". The everyday round (shifts, bread, each day's tending) folds into one line per moment, "Everyday life · 6 things", that opens on a click, and a quiet stretch reads as one line ("Time 60–80") instead of a column of identical ones.
- **Pocket Universe speaks in its own words.** What a World records no longer says "Legacy cycle 13 is now a durable adaptive pattern", "(13 care / 13 explore)", "Trust is 7; tension is 0", "care cycle 106" or "ridge trace 106". It says "The habitat commons grew stronger through coordinated upkeep and expansion", "Care and exploring have carried it in equal measure" and "Trust between them grew", and where two people stand reads "They trust each other, with some friction". Choices no longer say "durable", "seed" or "one more cycle", and a return no longer counts the routine ("· 7 times", "· 4 updates").

- **You see the World before you read about it.** A World opens on a drawn scene: places as tiles, people as faces, the relationship between them as a line that says what it is (Partners, Rivals), and a halo on whoever the latest news is about. A strip next to the title shows when things happened over the World's life. A crowded World such as Tiny Society's harbour lays itself out so nothing covers anything else.
- **Branches are a picture.** The lineage window, now called Branches, draws every World as a lane on one time axis. Each branch leaves its parent at the moment it split, and the selected World's ancestry lights up. It opens from the badge in a branched World's title bar, which now names the World it came from ("Branched from Ares · Held on") instead of a clipped file id. Home says where a World came from ("Branched from Ares · Held on by choosing 'Let time pass'") instead of listing file ids and "+10".
- **The World moves.** When time passes or a choice lands, what happened eases in, in the order it happened. A place or person in trouble has a slow ring breathing around it. The strip of the World's history grows in. Motion follows the system's Reduce Motion setting.
- **Coming back shows what moved, on the people it moved on.** A World you return to marks each person and place with what changed while you were away: "cash ↓24" under Jonas, "→ repaired" on a boat, coloured by whether it is good news. The values are replayed from the World's own history, not remembered by the app.
- **A choice shows what it changes before you make it.** Each choice lists its consequences as small coloured chips ("↑ Trust", "Sea Finch → repaired", "Against the World's direction"), and pointing at it rings what it would change on the World's scene. A test holds every choice to the consequences it shows.
- **News is coloured by what it means.** Trouble rising is amber, a loss is red, and good news is green: on the faces beside the story, the standing cards, and the places and people on the scene.
- **What if… shows two futures, not two identical cards.** Each future leads with the thing that happened only there ("Something was lost"), draws its World with what differs from the other future picked out, and says how it ends. Where the two split reads as the last moment they share, the first thing that went differently, and what followed, instead of "FIRST RECORDED DIFFERENCE" and "1 supporting record folded".
- **Every window uses the same palette.** What if…, lineage, Settings, About, the Analyst panel and Pack review now take their colours from the same named roles as the World window, so dark mode is designed rather than computed there too.
- **Every World on Home has its own small landscape**, so Worlds can be told apart before reading their names.
- **A World window reads like a page, not a log.** The choice in front of you is the one highlighted panel, beside what happened, each beat with the face of whoever it happened to; how things stand follows. People, places, and a history grouped by moment sit in a sidebar that folds under the page in a narrow window. The World's name appears once, in one title bar with Branch and What if….
- **Choices speak the World's language.** "Choice signal: one full cycle resolves under current rules…" and "Verified by this Event: World direction = Rooted at generation 6" are gone. A choice you made now reads "You chose · Rooted — This World now leans rooted", and why anything happened is still one click away under *Why it happened*.
- **Home lists your Worlds by name, with one Open button each.** What if…, Rename, Export… and Remove are quiet actions under the name; event counts and file ids are gone from the card. Start a new World is a grid led by the World to try first.
- **Home no longer offers "Start your first World" to someone who has one**, no longer shifts under the pointer when a message appears, and names the World you opened.
- **One palette, chosen for both appearances.** Colours, type sizes, weights, and hover states come from one set of named roles, with dark values picked by hand and contrast checked against WCAG AA, instead of a formula inverting light colours.
- **Coming back to a World stops reading like a dashboard.** A return briefing mixed two different things in one list: what happened to somebody, and how much of the routine ran. Measured over fourteen returns to a Tiny Society World, eight of them led with "Harbor Bakery had customers · 40 purchases · 400 revenue" — a counter, presented exactly like news. The two are now marked apart where the World produces them, so a counter can never again be handed to you as the answer to "what happened while I was away?".
- **Tiny Society 0.3.0: running out of money is something you are told about.** Jonas leaves the opening story unemployed with savings to burn, and burning them took twenty periods during which the World recorded nothing at all — his cash fell from 85 towards nothing and every briefing in between said only that the bakery had customers. The World now records the crossings: when he starts spending savings he cannot replace, when he can no longer cover a day, and when he is covering his own days again. Each is recorded once per spell, so none of them becomes another daily counter.
- **A day nobody could pay for no longer passes in silence.** When Jonas could not cover his living costs the simulation simply skipped him — no event, no trace, and a World that appeared to freeze with him at 5 cash forever. That day is now recorded like anything else that happens.
- **The same thing happening every day is told once.** Once Sea Finch is back in the water Jonas sells a catch daily, and a briefing would print three identical lines about it. The most recent stands for the rest; how many there were is what the counters are for.
- **A World stops waiting for you forever.** Measured over twenty returns to a Tiny Society World, sixteen had nothing to report, and from the thirteenth onward the World offered no choice at all — it held out one button, and if nobody pressed it, it held it out for the rest of time. Every offer now has a deadline as well as a default, the way Pocket Universe's choices have since `v0.4.0`. Leo's backing for the boat repair stands for a while and then he puts the money elsewhere. What drifts is exactly as durable as what you choose, and the briefing says which it was: "while nobody was watching".
- **Losing an offer opens a harder question rather than ending the story.** A man with a broken boat and no backing can sell her for far less than the repair would have cost — that is the shape of having not answered — and a man with no boat asks the bakery for work. Whether Mara takes him back is the next thing you can decide, and it drifts to yes, because a small town does not leave a destitute neighbour standing outside. Saying yes is not free: the bakery carries a second daily wage against the same island trade, the till drains, and the payroll chain that has been in this Pack since the beginning closes Harbor Bakery — which is when the reopening choices come back. The same twenty returns now have nine with nothing to report, and a choice on the table at every one of the last seven.
- **A long absence no longer loses the beginning of its own story.** A briefing told the four most recent things and dropped the rest in silence, so a fortnight holding a household budget cut, the demand loss it caused, a payroll shortfall and two closures reported the closures and never mentioned the cut. It now tells up to eight and says how many it left out.
- **A return briefing reads forwards.** Keeping the newest beats is how you choose what to tell; telling them newest-first is how you turn a story back into a log. A window reported "Harbor Bakery closed its doors" above "The bakery could not cover payroll" — the consequence printed above its own cause. It now reads the way it happened: the payroll failed, so the bakery shut, so the school's income went with it.
- **A line filed under somebody says who it is about.** "The bakery could not cover payroll" is recorded against the worker whose wage failed, so it appeared beside Jonas's name while never mentioning him. It now reads "The bakery could not pay Jonas".
- **A boat somebody sold is not still something they own.** Jonas's card read "Owns · Sea Finch" for the rest of the World after she went for scrap.
- **Being out of work is a condition, not a moment.** Asking the bakery for work happened once, on the day the boat sold, so a man who lost the counter job when the bakery closed spent the rest of the World unemployed — with a status that still said he had been taken on. He asks whenever asking makes sense, including after a reopening, which pressing the reopen button now actually reaches.
- **Climbing out of trouble is a higher bar than falling into it.** A counter wage that hovers around a week of living costs would otherwise cross the same line in both directions every few days, and every crossing was reported as news.

## v0.5.2 (2026-09-10)

- **A new app icon.** The first one was a node joined to two smaller nodes, which is the share glyph every platform already uses and said nothing about what this app does. It is now a single stroke that spirals outward from the centre, thin and pale where a World's history begins and broad and warm at the present — the one shape that means "this kept going while you were away".
- **Full screen.** The Window menu had Minimize and Zoom but no **Enter Full Screen**, and ⌃⌘F — which works in every other Mac app — did nothing. AppKit only adds that item for apps whose menus come from a nib, and this app builds its menus in code, so it had to be added by hand.

## v0.5.1 (2026-09-10)

**Every release before this one shipped with no text on screen.** World Machine drew its windows, its cards, its buttons and its borders, and every word inside them was invisible — the app was unusable, and had been since the first release. `gpui_platform`, the crate the app opens its window through, ships with an empty default feature set; without `font-kit` it quietly installs a text system that measures text but draws nothing, logs no error, and never crashes. Enabling that feature is the whole fix. A check now fails the build if any desktop app in this repository asks for a window without also asking to be able to render text.

- **The app has an icon.** The bundle never carried one, so macOS drew the blank generic-application page in the Dock, the Finder and the Cmd-Tab switcher. It is now built from a single source image at every size macOS asks for.
- **Home no longer opens with an error on it.** `v0.5.0` started shipping a World as a bundled Pack that the app also carries compiled into itself. The Host refuses to register one Pack twice, so the install failed, the Pack never became reachable, and Home reported it on every launch. An installed Pack now wins over the copy inside the app.
- **The Create button on Home stays on screen.** The first-run card laid its text out at full width and pushed its own button past the window edge at the size Home opens at.
- The About window no longer says everything stays on your Mac without qualification — the same correction the privacy page took in `v0.5.0`, in the one place a user actually reads it.

## v0.5.0 (2026-09-10)

**Every World you already have still opens.** No World Pack changed the rules its Worlds run under, so nothing saved by `v0.4.0` — or by anything older that still opens — is closed by this release. That is the first time a content release has been able to say so, and it is what the additive-content discipline is for.

- **Your Worlds can write in their own words.** Until now a World told you what happened using sentences written into the app, and there were only so many of them: over twenty week-long returns, the lines describing trouble repeated 82% of the time because each one had exactly three phrasings. Give a World a voice and what it tells you is written about your World instead — the same measurement drops those lines to no repetition at all, and two Worlds grown from the same seed stop sharing their prose.
- **World Machine has a Settings window.** ⌘, opens it, and it is where a World's voice lives: on or off, whether it reaches a model through a program already on your Mac or through an API key, and the key itself. It says in plain words what each choice means. ⌘, used to open About, which is not what that key does anywhere else on macOS; About is still in the menu.
- **A key belongs to you, not to the app.** It is kept in your login keychain rather than in any file World Machine writes, so a backup of your Worlds folder never carries one, and you can see, inspect, or delete it yourself in Keychain Access under "World Machine · World voice".
- **Nothing is worse for having a voice.** A model that is unreachable, slow, refuses, or answers with something unusable leaves a World reading exactly as it does with no voice at all, line by line. A voice can only put an already-recorded fact into words: it cannot decide what happens in a World, so a bad answer costs you a clumsy sentence and never a broken World. Replaying a World never asks a model anything.
- **Tiny Society ships in the app.** It has been advertised in the README since before `v0.2.0` and its second consequence chain shipped in `v0.3.0`, but the packaged app only ever carried Pocket Universe and Micro Company, so nobody who downloaded World Machine could open it. It is now one of the included Worlds on Home, alongside the other two.
- **What leaves your Mac, and what does not.** An API key is the only setting in the app that sends anything anywhere: one request each time you return to a World, carrying what that World has already recorded and nothing else — not your other Worlds, not your library, not your file names, not the log. One request per return, never one per period. A program you choose is between you and that program. Both are off until you switch them on, and the [privacy page](docs/PRIVACY.md) now says all of this rather than claiming everything stays on your Mac without qualification.
- The Packs the app ships now have to agree about who they are. The Pack's own id and version, what the app expects to find, and what the build actually writes into the app are three lists in three places that cannot see each other; a drift between them makes a bundled World refuse to install, and only on a real Mac. A check now fails the build instead.

## v0.4.0 (2026-09-10)

**Worlds created before this release will not open.** Pocket Universe moves to `0.18` because the era engine changes the rules its Worlds run under, and a `.world` file records the Pack version it was created with. Export anything you want to keep before updating; Tiny Society and Micro Company Worlds are unaffected.

- Pocket Universe 0.18: **a World's story no longer ends.** The four chapters were a ladder — measured, a World completed all of them by its thirteenth period and then offered one button that changed nothing for as long as you kept it. They are now the stages of one era, and eras loop: when a succession settles, the successor becomes the keeper of a new era and the World faces a different trouble. No era faces the trouble the one before it just survived, and how an era ended decides what the next one inherits — a legacy handed on unchanged carries over, one let go has to form again.
- Each seed now has a second trouble to face: Ares can lose its air to a dust season as well as its water to the reclaimer, Maple Street can lose the night bus as well as the lease, and Icebridge can empty its fish vault as well as crack a span.
- A third trouble per seed, which is what lets a World's history start to matter: with only two, the never-repeat rule already decided everything. Ares can now also lose contact with the relay, Maple Street can lose its crowd to a highway mall, and Icebridge can lose its ice to an early thaw.
- **How you have been answering changes what comes next.** Two Worlds at the same era facing the same trouble diverge if one has been handing its legacy on and the other rewriting it. A World that has answered the same way several eras running is told so: "3 eras running have handed their habits on unchanged; nobody keeping them now chose them."
- **A World left alone keeps living.** Measured before this: a World seeded and then never answered walked to its eighteenth period and stopped there forever, holding buttons nobody was going to press; one never given its opening choices never got started at all. Every choice now has a deadline as well as a default, so leaving is itself an answer — the World reaches the default on its own, one choice per period. Coming back, the briefing leads with **Decided without you** and says what it settled and why. What it decided is exactly as durable as what you decide; a trouble left unanswered still costs the World its anchor rather than being quietly held for you, and a lost anchor is never rebuilt without you.
- **A week away is now a week of World time.** Background catch-up was capped at seven periods — a day and three quarters — from when a World held a fixed amount of story and running through it unattended was the risk. Eras removed that ceiling, so the cap now says the simpler thing: a World lives through at most a week of its own time while you are away. A day away moves it four periods; a week away moves it a week; a month away still moves it a week, because a return should be readable rather than exhaustive.
- **A long absence reads as the eras it crossed**, not as a list of periods. Coming back to a World that turned over while you were gone now opens with *"2 eras turned — you left during era 1; this is era 3"* and what the current era began on, above everything that happened inside it.
- An era opens on a quiet stretch before its trouble arrives, and says so instead of falling back to first-visit language. A World past its first era says which era it is and what it inherited.

## v0.3.0 (2026-09-08)

**Worlds created before this release will not open.** Pocket Universe moves to `0.17` and Tiny Society to `0.2` because both change the rules their Worlds run under, and a `.world` file records the exact Pack version it was created with. This release ships only the new versions, so a Pocket Universe or Tiny Society World saved by `v0.2.2` or earlier reports that it needs a Pack version this build does not have. Export anything you want to keep before updating; Micro Company Worlds are unaffected.

- Pocket Universe 0.17: chapter four. Once the pressure has resolved, two generations later someone who did not live through the World's beginning steps forward. This chapter has no deadline — the successor waits as long as you leave them waiting, but every generation of waiting deepens habits of their own, so the briefing moves from "New hands" through "Own habits" to "Already theirs". Entrust the legacy unchanged or release it to be rewritten; the answer is durable, and releasing resets the legacy's reinforcement cycles the way recovering a lost anchor does. The same choice reads differently depending on how chapter three ended.
- Tiny Society: a second consequence chain. Reopening the bakery lean leaves Mara working the counter alone. Only the demand that came back — Jonas buying bread again, which is the end of the first chain — is counted, and once the counter has carried that trade and the till holds a wage in reserve, Mara takes Mia on and the job the closure cost the island comes back. A World whose harbour never recovered keeps a one-person bakery for good.
- My Worlds: once you have six or more Worlds, the list gains **Find a World** and an **Order** switch — most recently played, or A to Z. Typing matches a World's name, its World Pack's title, or its file id, and the heading says how many of your Worlds are showing.
- Windows open where you left them. Home and World windows remember their size and position between launches; a window saved on a display you no longer have opens centred instead of off-screen.

## v0.2.2 (2026-09-08)

- My Worlds: **Rename** on any World card gives it the name you type, and clearing the name lists it under its World Pack's title again. Branches no longer all read "Pocket Universe" with only a file id to tell them apart.
- My Worlds: **Remove** takes a World off Home after a second click on the card. Its file moves to a `Removed` folder inside your Worlds folder, so a removal by mistake is a drag back in the Finder.
- A World window is titled by the World's name, and its header shows the name with the file id beside it. A World named on Home is called by that name in its own window, in its save and reload lines, and in the file name Save As suggests.
- One damaged or foreign file in the Worlds folder no longer hides every other World. The Worlds that can be read are listed as usual, and Home names the files it could not read.

## v0.2.1 (2026-09-08)

- Home shows a banner when a newer stable release exists (one request to the GitHub Releases API per launch; `WORLD_MACHINE_NO_UPDATE_CHECK=1` turns it off). Download opens the release page.
- Help → Report a Problem… opens the issue template with the build label and macOS version already filled in.
- Dark mode: every window follows the macOS appearance. The light palette is adapted automatically (backgrounds dark, cards slightly raised, text and accents light) and windows re-render when the system switches.
- Standard Mac behaviour: Cmd-W closes a window, Cmd-M minimizes, Cmd-H hides, a Window menu, Hide Others / Show All in the app menu, and clicking the Dock icon after the last window is closed brings Home back.

## v0.2.0 (2026-09-07)

The "usable" release: install by drag and drop, a World that opens by itself, two buttons to live in it, and a way to report problems. Not yet notarized: the first launch asks once until the Developer ID pipeline in docs/RELEASE_SIGNING.md is switched on.

- Diagnostics: a local log under `~/Library/Logs/World Machine`, an About window with the build label and signing status, and Help menu items to copy diagnostics, show the log, open the install guide, and report a problem. Cmd-Q quits.
- Home: **What if…** on every World card opens the World and runs the comparison immediately (its first two choices, 20 periods); "Change choices…" in the result window opens the full setup.
- Fewer controls: Home's header buttons move to a File menu (Import World, Install World Pack, Refresh); installed-Pack management is folded behind one line; the World window keeps only Branch and What if… as buttons, with Save As, Reload, lineage, comparisons, and the Analyst in a World menu.
- Layout: Home header, World cards, document chrome, and the projection center pane no longer lay out wider than their window.
- Copy: the first-launch path speaks of Worlds being prepared and ready instead of Packs, probes, and registries.
- Tiny Society: every briefing opens with a "Harbor today" state line, so a fresh visit and a return share the same now / changed / can-do shape.
- Install: releases now ship a DMG with an Applications shortcut; no Terminal in the install path. The pipeline signs with a Developer ID, notarizes, and staples as soon as the five secrets in docs/RELEASE_SIGNING.md exist. Help → Check for Updates… opens the Releases page. A one-line Terminal installer remains as an optional extra.
- First launch on a fresh install opens straight into the first Pocket Universe World instead of stopping at a Create card.
- Release: the zip unpacks to a folder with the app and a `Read Me First.txt` explaining the one-time "Open Anyway" step.
- Docs: user-first README, this changelog, known issues, privacy note.

## v0.1.0-pre.5 (2026-09-07)

- Pocket Universe 0.16: chapter three. After a legacy has reinforced itself, a seed-specific pressure rises against the World's anchor, warns, peaks, and can durably cost the anchor; the observer holds or reaches, and a lost anchor can be recovered.

## v0.1.0-pre.4 (2026-09-07)

- Universal build: one zip runs on Apple Silicon and Intel Macs, including the bundled Packs and the analyst host.

## v0.1.0-pre.3 (2026-09-07)

- Included Packs activate on first launch without a review dialog; the review flow remains for user-supplied `.worldpack` files.

## v0.1.0-pre.2 (2026-09-07)

- The experimental World Analyst entry is hidden unless Node and Pi are installed, so a fresh install needs nothing beyond the app.

## v0.1.0-pre.1 (2026-09-07)

- First automated pre-release: a tag or a workflow dispatch builds the app, validates the package, and publishes the zip, SHA-256, manifest, and release notes. Install guide for macOS 14 and 15.
