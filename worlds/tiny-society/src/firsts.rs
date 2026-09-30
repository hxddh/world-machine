//! The harbour's quiet days: at most two letters a week and often fewer, and on the other
//! quiet days a small first. Someone speaks of something they have never
//! mentioned, or shows the player a corner of the harbour they have not
//! seen.

use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use lives::{Corner, QuietDays, Subject};

/// The most letters in any week.
pub(crate) const LETTERS_A_WEEK: usize = 2;

/// How many letters a week may bring, taken week by week in a mixed
/// order: two some weeks, one most, and now and then none (about one and
/// a quarter a week).
const LETTER_WEEKS: &[usize] = &[2, 1, 1, 0, 2, 1, 1, 2];

/// One day in this many is left quiet, with nothing new at all.
const STILL_EVERY: u64 = 40;

const CORNERS: &[Corner] = &[
    Corner {
        place: HARBOR,
        name: "the old net loft",
        said: "Nobody comes up here but me. Best view of the boats.",
    },
    Corner {
        place: HARBOR,
        name: "the steps below the quay",
        said: "Sit here at low tide. You can hear the crabs.",
    },
    Corner {
        place: HARBOR,
        name: "the harbourmaster's hut",
        said: "Nobody's been harbourmaster for years. The kettle still works.",
    },
    Corner {
        place: HARBOR,
        name: "the lobster pots behind the slipway",
        said: "Every one of these has a name. Don't ask.",
    },
    Corner {
        place: HARBOR,
        name: "the bench at the end of the breakwater",
        said: "Best seat on the island. Don't tell anyone.",
    },
    Corner {
        place: HARBOR,
        name: "the tide board",
        said: "My dad painted these numbers. Still right, mostly.",
    },
    Corner {
        place: HARBOR,
        name: "the rock pools by the point",
        said: "Starfish, if you're patient.",
    },
    Corner {
        place: BAKERY,
        name: "the back room of the bakery",
        said: "This is where the bread rises. Mind the draught.",
    },
    Corner {
        place: BAKERY,
        name: "the old bread oven",
        said: "My gran baked in this. It still bakes best.",
    },
    Corner {
        place: BAKERY,
        name: "the flour loft",
        said: "Up the ladder. Sneeze if you must.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's back step",
        said: "Where I sit with a cup before the first batch.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's map room",
        said: "Every map of the island ever drawn. Some of them are even right.",
    },
    Corner {
        place: SCHOOL,
        name: "the school bell tower",
        said: "Ring it once, gently. Everyone will know it was you.",
    },
    Corner {
        place: SCHOOL,
        name: "the bottom of the school garden",
        said: "The children bury treasure here. Don't dig.",
    },
    Corner {
        place: SCHOOL,
        name: "the cloakroom pegs",
        said: "Every child who ever went here has a peg. Mine's the crooked one.",
    },
    Corner {
        place: SCHOOL,
        name: "the reading nook under the stairs",
        said: "Best hiding place on the island. Ask any child.",
    },
    Corner {
        place: PUB,
        name: "the snug at the Anchor",
        said: "Three seats and a fire. The best room in the pub.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's cellar",
        said: "Cool as anything down here, even in August.",
    },
    Corner {
        place: PUB,
        name: "the photographs behind the bar",
        said: "Every regatta since the war. Find me, if you can.",
    },
    Corner {
        place: PUB,
        name: "the pub's back garden",
        said: "One table, one apple tree, and the sea.",
    },
    Corner {
        place: PUB,
        name: "the dartboard with the hole in it",
        said: "Nobody will say who threw that. Everybody knows.",
    },
    Corner {
        place: HARBOR,
        name: "the mooring ring with the brass plate",
        said: "Somebody's great-grandfather fitted that. It's never once pulled out.",
    },
    Corner {
        place: HARBOR,
        name: "the gap in the harbour wall",
        said: "Kids squeeze through here to fish. So did I.",
    },
    Corner {
        place: HARBOR,
        name: "the fish scales on the old weigh-house door",
        said: "Every record catch is nailed up here. Mine's the small one.",
    },
    Corner {
        place: HARBOR,
        name: "the smugglers' cave under the point",
        said: "Nobody's smuggled anything for a hundred years. Probably.",
    },
    Corner {
        place: HARBOR,
        name: "the capstan on the old slip",
        said: "Push it round and it still sings.",
    },
    Corner {
        place: HARBOR,
        name: "the names carved in the quay railing",
        said: "Every couple on the island has had a go at this rail.",
    },
    Corner {
        place: HARBOR,
        name: "the lamp that never lights by the ferry steps",
        said: "It hasn't worked since I was born. We leave it for luck.",
    },
    Corner {
        place: HARBOR,
        name: "the crab line spot on the east wall",
        said: "Bacon on a string. Works every time.",
    },
    Corner {
        place: HARBOR,
        name: "the drying green behind the cottages",
        said: "Monday is wash day. The whole island flaps.",
    },
    Corner {
        place: HARBOR,
        name: "the coil of rope nobody owns",
        said: "It's been there forty years. Nobody dares move it.",
    },
    Corner {
        place: HARBOR,
        name: "the high-tide mark from the great flood",
        said: "The water came up to here. Look how high.",
    },
    Corner {
        place: HARBOR,
        name: "the old customs post",
        said: "They used to check your pockets here. Now it keeps the gulls dry.",
    },
    Corner {
        place: HARBOR,
        name: "the fishermen's chapel on the rocks",
        said: "Just a room and a candle. Everyone stops in before a storm.",
    },
    Corner {
        place: HARBOR,
        name: "the boat with no name",
        said: "She washed up one spring. We never found out whose.",
    },
    Corner {
        place: HARBOR,
        name: "the bollard everyone sits on",
        said: "It's warm by four o'clock. Get there early.",
    },
    Corner {
        place: HARBOR,
        name: "the spot where the seals haul out",
        said: "Stay low and quiet. There, see the pup?",
    },
    Corner {
        place: HARBOR,
        name: "the old ice house",
        said: "Before fridges, the whole catch went in here.",
    },
    Corner {
        place: HARBOR,
        name: "the lookout post on the headland",
        said: "Somebody watched for boats from here every night for a century.",
    },
    Corner {
        place: HARBOR,
        name: "the shell midden at the top of the beach",
        said: "Oyster shells from before anyone remembers. Layers of them.",
    },
    Corner {
        place: HARBOR,
        name: "the bench where the ferry queue waits",
        said: "You hear all the news on this bench. All of it.",
    },
    Corner {
        place: HARBOR,
        name: "the sea glass cove",
        said: "Green, white and one blue piece a year, if you're lucky.",
    },
    Corner {
        place: HARBOR,
        name: "the old winch house",
        said: "Somebody always wants to fix the winch. Nobody ever does.",
    },
    Corner {
        place: HARBOR,
        name: "the fishing flags in the net store",
        said: "Every family had a flag. Ours is the torn one.",
    },
    Corner {
        place: HARBOR,
        name: "the rope walk along the back lane",
        said: "They twisted rope here once, all the way down the lane.",
    },
    Corner {
        place: HARBOR,
        name: "the puffin ledge on the cliff",
        said: "In May it's all beaks and wings. Bring a coat.",
    },
    Corner {
        place: HARBOR,
        name: "the wreck you can see at low spring tide",
        said: "Twice a year her ribs come up. Then she's gone again.",
    },
    Corner {
        place: HARBOR,
        name: "the harbour mouth at dawn",
        said: "Stand here at six. The whole sea turns pink.",
    },
    Corner {
        place: HARBOR,
        name: "the old boat shed with the painted doors",
        said: "Every door a different colour. Nobody agreed.",
    },
    Corner {
        place: HARBOR,
        name: "the stone steps worn smooth by boots",
        said: "Two hundred years of boots made that dip.",
    },
    Corner {
        place: HARBOR,
        name: "the pool where the children learn to swim",
        said: "Cold enough to make you yell. Everyone learns here.",
    },
    Corner {
        place: HARBOR,
        name: "the whalebone arch on the cliff path",
        said: "From a whale that beached when my gran was small.",
    },
    Corner {
        place: HARBOR,
        name: "the gutting table on the quay",
        said: "Not pretty, but it's where the day's money is made.",
    },
    Corner {
        place: HARBOR,
        name: "the old signal flags in the harbour office",
        said: "Each one means something. I know three.",
    },
    Corner {
        place: HARBOR,
        name: "the spring that runs fresh on the beach",
        said: "Fresh water, right beside the salt. Taste it.",
    },
    Corner {
        place: HARBOR,
        name: "the heron's post at the stream mouth",
        said: "Same heron every morning. We call him the Mayor.",
    },
    Corner {
        place: HARBOR,
        name: "the lobster pond",
        said: "Where the lobsters wait for the ferry. Poor things.",
    },
    Corner {
        place: HARBOR,
        name: "the storm shutters on the harbour cottages",
        said: "Up in October, down in April. Like clockwork.",
    },
    Corner {
        place: HARBOR,
        name: "the lantern hooks along the pier",
        said: "On feast nights every hook has a lantern.",
    },
    Corner {
        place: HARBOR,
        name: "the painted stone at the end of the jetty",
        said: "Whoever finds it moves it. It's been round the island twice.",
    },
    Corner {
        place: HARBOR,
        name: "the fog bell on the point",
        said: "When the fog's in, it rings all night. You stop hearing it.",
    },
    Corner {
        place: HARBOR,
        name: "the old salt pans",
        said: "They made salt here once. Now the rain fills them.",
    },
    Corner {
        place: HARBOR,
        name: "the net mending shed",
        said: "Smells of tar. I love it.",
    },
    Corner {
        place: HARBOR,
        name: "the view from the top of the harbour steps",
        said: "Count the boats from here. Every one's home.",
    },
    Corner {
        place: HARBOR,
        name: "the bottle of messages under the quay",
        said: "People leave notes in it for whoever comes next.",
    },
    Corner {
        place: HARBOR,
        name: "the kelp beds off the rocks",
        said: "At slack water you can see right down. Like a forest.",
    },
    Corner {
        place: BAKERY,
        name: "the sourdough crock",
        said: "Older than me. It gets fed before I do.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's recipe tin",
        said: "My mother's hand, my gran's hand, and mine at the bottom.",
    },
    Corner {
        place: BAKERY,
        name: "the window where the loaves cool",
        said: "The whole lane knows when the bread's out.",
    },
    Corner {
        place: BAKERY,
        name: "the flour sacks by the door",
        said: "Mainland flour, island water. That's the secret.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's wood pile",
        said: "Driftwood burns hot. Salt makes it spit blue.",
    },
    Corner {
        place: BAKERY,
        name: "the proving cupboard",
        said: "Warm as a hug. Don't open the door.",
    },
    Corner {
        place: BAKERY,
        name: "the crack in the bakery counter",
        said: "Where my dad dropped the cash box in 1970-something.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's back yard",
        said: "Hens, a bench and a lot of crumbs.",
    },
    Corner {
        place: BAKERY,
        name: "the bread paddle rack",
        said: "Every paddle has a job. This one's for Sundays.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's first coin, framed",
        said: "The first penny ever taken here. Never spent.",
    },
    Corner {
        place: BAKERY,
        name: "the herb pots on the bakery sill",
        said: "Rosemary for the focaccia. Don't pinch it.",
    },
    Corner {
        place: BAKERY,
        name: "the cake stands in the cupboard",
        said: "Wedding cakes, birthday cakes. They've held them all.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery clock that runs fast",
        said: "Ten minutes fast, so I'm never late. It never works.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's delivery basket",
        said: "It goes round the lanes on Saturdays. Half comes back eaten.",
    },
    Corner {
        place: BAKERY,
        name: "the bread stamp with the anchor on it",
        said: "Every loaf gets the anchor. It's how you know it's ours.",
    },
    Corner {
        place: BAKERY,
        name: "the attic window over the bakery",
        said: "You can see the ferry an hour before it comes.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's old scales",
        said: "They're a gram out. The customers win.",
    },
    Corner {
        place: BAKERY,
        name: "the stool by the oven",
        said: "The warmest seat on the island. It's mine.",
    },
    Corner {
        place: BAKERY,
        name: "the jam shelf in the bakery pantry",
        said: "Every jar's labelled with the year. That's 2003's.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's mixing bowl",
        said: "Big enough to bath a baby in. Don't ask.",
    },
    Corner {
        place: BAKERY,
        name: "the rolling pin carved from a mast",
        said: "From a ship's mast. Heavier than it looks.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's chalk board",
        said: "Today's bread, and a joke if I've got one.",
    },
    Corner {
        place: BAKERY,
        name: "the bench outside the bakery",
        said: "The first loaf of the day gets eaten right here.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery cat's basket",
        said: "She doesn't earn her keep. She knows it.",
    },
    Corner {
        place: BAKERY,
        name: "the hatch where the flour comes in",
        said: "Stand back when the sacks drop.",
    },
    Corner {
        place: PUB,
        name: "the ship's bell over the bar",
        said: "Last orders. Ring it and you buy a round.",
    },
    Corner {
        place: PUB,
        name: "the pub's piano",
        said: "Three keys don't work. We play around them.",
    },
    Corner {
        place: PUB,
        name: "the fireplace at the Anchor",
        said: "Burns all winter. We've never let it go out.",
    },
    Corner {
        place: PUB,
        name: "the ceiling covered in postcards",
        said: "Everyone who leaves sends one back. Everyone.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's old sign",
        said: "Painted by a sailor who paid his bill with it.",
    },
    Corner {
        place: PUB,
        name: "the corner table by the window",
        said: "Where the fishermen sit. Nobody else, ever.",
    },
    Corner {
        place: PUB,
        name: "the pub's chalk scores from the quiz",
        said: "The same team's won for eleven years. Mine.",
    },
    Corner {
        place: PUB,
        name: "the tankards on hooks above the bar",
        said: "Everyone has their own. Don't touch Noah's.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's guest book",
        said: "Sixty years of names. Some of them famous. None of them sober.",
    },
    Corner {
        place: PUB,
        name: "the stuffed fish over the door",
        said: "Caught by my grandfather. It gets bigger every time he tells it.",
    },
    Corner {
        place: PUB,
        name: "the pub's back stairs",
        said: "They creak on the third step. Useful for knowing who's coming.",
    },
    Corner {
        place: PUB,
        name: "the barrel room",
        said: "Every barrel has a nickname. That one's Dolores.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's lost property box",
        said: "Three umbrellas, a false tooth and a trumpet.",
    },
    Corner {
        place: PUB,
        name: "the settle by the fire",
        said: "Two people fit. Three if they like each other.",
    },
    Corner {
        place: PUB,
        name: "the pub's bottle-cap wall",
        said: "Every cap from every summer. We're running out of wall.",
    },
    Corner {
        place: PUB,
        name: "the old skittles in the pub cupboard",
        said: "We used to play every Friday. Maybe again.",
    },
    Corner {
        place: PUB,
        name: "the window seat where the storytellers sit",
        said: "If you sit there, you owe the room a story.",
    },
    Corner {
        place: PUB,
        name: "the tide clock at the Anchor",
        said: "More use than a real clock, round here.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's kitchen hatch",
        said: "Chips come through here. Nothing else matters.",
    },
    Corner {
        place: PUB,
        name: "the fiddle hanging on the pub wall",
        said: "Whoever can play it, may. Few can.",
    },
    Corner {
        place: PUB,
        name: "the ship in a bottle behind the bar",
        said: "Nobody knows how it got in. Nobody's getting it out.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's weather glass",
        said: "Tap it twice. If it drops, stay in.",
    },
    Corner {
        place: PUB,
        name: "the pub's garden swing",
        said: "For the children. The grown-ups use it more.",
    },
    Corner {
        place: PUB,
        name: "the list of lifeboat crews on the pub wall",
        said: "Every name who ever went out. Some didn't come back.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's dartboard chart",
        said: "Everyone's best score. Mine's not up there. Yet.",
    },
    Corner {
        place: PUB,
        name: "the pub's old wireless",
        said: "It only gets the shipping forecast. That's all we need.",
    },
    Corner {
        place: PUB,
        name: "the jar of foreign coins by the till",
        said: "From visitors all over. That one's from Peru.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's rooftop",
        said: "Up the ladder. Best fireworks view there is.",
    },
    Corner {
        place: PUB,
        name: "the pub's cosy nook behind the stairs",
        said: "Where people go to say things quietly.",
    },
    Corner {
        place: PUB,
        name: "the painted panel of the first regatta",
        said: "Look closely. That's Noah's grandad, losing.",
    },
    Corner {
        place: SCHOOL,
        name: "the height marks on the school door",
        said: "Every child measured, every September. Some of these are grandparents now.",
    },
    Corner {
        place: SCHOOL,
        name: "the nature table",
        said: "A crab shell, a gull feather and a very old conker.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's weather station",
        said: "The children read it every morning. They're very strict.",
    },
    Corner {
        place: SCHOOL,
        name: "the old school desks in the store",
        said: "Initials in every lid. Look, there's Noah's.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's dressing-up box",
        said: "Three crowns, a pirate hat and a cow.",
    },
    Corner {
        place: SCHOOL,
        name: "the globe with the island drawn on",
        said: "The children drew us on. We're bigger than Ireland.",
    },
    Corner {
        place: SCHOOL,
        name: "the classroom fish tank",
        said: "One goldfish, called Captain. He's older than the teacher.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's piano stool full of music",
        said: "Every song the island ever sang, in one stool.",
    },
    Corner {
        place: SCHOOL,
        name: "the chalk drawings on the playground wall",
        said: "The rain washes them off. They draw them again.",
    },
    Corner {
        place: SCHOOL,
        name: "the lost mittens line",
        said: "One of each pair. Always one of each.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's time capsule stone",
        said: "Open it in fifty years. I'll be ninety.",
    },
    Corner {
        place: SCHOOL,
        name: "the story corner cushions",
        said: "Every cushion sewn by a different mother.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's star chart",
        said: "Stuck on the ceiling. They glow at night, if you stay late.",
    },
    Corner {
        place: SCHOOL,
        name: "the pegs where the lunchboxes hang",
        said: "You can tell whose is whose by the smell.",
    },
    Corner {
        place: SCHOOL,
        name: "the old slates in the school cupboard",
        said: "Before paper was cheap. Still squeak like anything.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's seashell collection",
        said: "A shell from every beach. The children argue about which is best.",
    },
    Corner {
        place: SCHOOL,
        name: "the crooked tree in the playground",
        said: "Every child climbs it. Every child falls out once.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's first register",
        said: "Twelve names, in ink. One of them is my great-gran.",
    },
    Corner {
        place: SCHOOL,
        name: "the paint pots on the art shelf",
        said: "Every colour but blue. Blue always runs out.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's tide table poster",
        said: "The children learn the tides before their times tables.",
    },
    Corner {
        place: SCHOOL,
        name: "the head's old whistle",
        said: "Never been blown. It's the threat that works.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's bean rows",
        said: "Every child plants a bean. One year a bean reached the roof.",
    },
    Corner {
        place: SCHOOL,
        name: "the window the children wave from",
        said: "When the ferry goes by, everyone waves. Every time.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's toy boat pond",
        said: "Just a big puddle, really. Nobody tell them.",
    },
    Corner {
        place: SCHOOL,
        name: "the long bench in the school hall",
        said: "Every school photo ever taken, right on this bench.",
    },
    Corner {
        place: SCHOOL,
        name: "the reading tree",
        said: "In summer we have lessons under it. Nobody listens.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's hand bell",
        said: "The old bell, before the tower. Heavy as a brick.",
    },
    Corner {
        place: SCHOOL,
        name: "the paper cranes over the school stairs",
        said: "A thousand cranes, for a wish. We're on nine hundred.",
    },
    Corner {
        place: SCHOOL,
        name: "the chart of who can swim",
        said: "A gold star for every length. Look how many.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's old map of the sea",
        said: "Sea monsters drawn in the corners. Still accurate, the children say.",
    },
    Corner {
        place: HARBOR,
        name: "the cormorant rock",
        said: "They stand there drying their wings like washing.",
    },
    Corner {
        place: HARBOR,
        name: "the old ferry bell on its post",
        said: "Ring it and the ferry comes. Eventually.",
    },
    Corner {
        place: HARBOR,
        name: "the hollow in the dunes",
        said: "Out of the wind, full of sun. Nobody can see you.",
    },
    Corner {
        place: HARBOR,
        name: "the eel trap in the stream",
        said: "Hasn't caught an eel in years. We check it anyway.",
    },
    Corner {
        place: HARBOR,
        name: "the barnacled anchor on the green",
        said: "Dragged up by a trawler. Too heavy to take anywhere.",
    },
    Corner {
        place: HARBOR,
        name: "the washed-up figurehead in the net store",
        said: "A lady with no nose. We call her Duchess.",
    },
    Corner {
        place: HARBOR,
        name: "the path to the hermit's hut",
        said: "Nobody's lived there in years. Somebody leaves flowers.",
    },
    Corner {
        place: HARBOR,
        name: "the old tarring pot",
        said: "We tarred the boats in this once. You can still smell it.",
    },
    Corner {
        place: HARBOR,
        name: "the rock that looks like a sleeping dog",
        said: "Everybody sees it once they've been told.",
    },
    Corner {
        place: HARBOR,
        name: "the pier's last plank",
        said: "Stand on it and make a wish. Don't fall in.",
    },
    Corner {
        place: HARBOR,
        name: "the gull that steals from the gutting table",
        said: "That's Mr Pike. He's been banned for years.",
    },
    Corner {
        place: HARBOR,
        name: "the pool of still water after the tide",
        said: "Look in and you'll see the whole sky twice.",
    },
    Corner {
        place: HARBOR,
        name: "the chest of spare oars",
        said: "Every oar in there broke on something. Ask about any.",
    },
    Corner {
        place: HARBOR,
        name: "the ferry's old ticket booth",
        said: "The ticket man sold sweets as well. Nobody bought tickets.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's candle drawer",
        said: "For when the power goes. It always goes.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's birthday list",
        said: "Every birthday on the island, pinned up. Nobody gets forgotten.",
    },
    Corner {
        place: BAKERY,
        name: "the flour footprints on the attic stairs",
        said: "The cat's. And mine. And the ghost's, some say.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's little back window",
        said: "I pass the children buns through here. Don't tell.",
    },
    Corner {
        place: BAKERY,
        name: "the pie dishes stacked to the ceiling",
        said: "One for every family. They all come back. Mostly.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's ledger of who owes",
        said: "Nobody's crossed off till they're ready. That's the island way.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery doorbell",
        said: "A ship's bell, small. It rings when the wind wants in.",
    },
    Corner {
        place: PUB,
        name: "the pub's map of every wreck round the island",
        said: "Forty-one. Each one has a song.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's secret shelf",
        said: "The good whisky. For weddings and wakes.",
    },
    Corner {
        place: PUB,
        name: "the arm-wrestling table",
        said: "Worn smooth in the middle. Noah's never lost. So he says.",
    },
    Corner {
        place: PUB,
        name: "the pub's creaky sign in the wind",
        said: "You can tell the weather by how it swings.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's boot rack",
        said: "Everyone's boots, all the same, all muddy. We never mix them up.",
    },
    Corner {
        place: PUB,
        name: "the drawing of the Anchor's first landlady",
        said: "Maud. She threw out a sea captain, they say.",
    },
    Corner {
        place: PUB,
        name: "the pub's lamp in the window",
        said: "Lit every night, for anyone still at sea.",
    },
    Corner {
        place: PUB,
        name: "the singing corner",
        said: "Once the fiddle starts, this corner starts. You'll see.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's bench outside",
        said: "Where people say the things they couldn't say inside.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's lost tooth jar",
        said: "Every tooth that fell out in class. The children insist.",
    },
    Corner {
        place: SCHOOL,
        name: "the chart of every bird seen from the window",
        said: "The children tick them off. The heron's their favourite.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's snow day box",
        said: "Sledges, mittens and hot chocolate. It's been waiting three winters.",
    },
    Corner {
        place: SCHOOL,
        name: "the old school bell rope",
        said: "Pull it gently. It's older than the tower.",
    },
    Corner {
        place: SCHOOL,
        name: "the class tadpole tank",
        said: "Every spring. Every spring they escape.",
    },
    Corner {
        place: SCHOOL,
        name: "the handprints on the school wall",
        said: "One for every child, in every colour.",
    },
    Corner {
        place: SCHOOL,
        name: "the quiet room with the beanbag",
        said: "For children who need a minute. Grown-ups too.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's weather diary",
        said: "Forty years of weather, one line a day, in children's hands.",
    },
    Corner {
        place: SCHOOL,
        name: "the paper boat shelf",
        said: "Every paper boat the children made that didn't sink.",
    },
    Corner {
        place: SCHOOL,
        name: "the gap in the school fence",
        said: "Everyone knows about it. Nobody mends it.",
    },
];

const SUBJECTS: &[Subject] = &[
    Subject {
        about: "the lighthouse",
        said: &[
            "My grandad used to wind the lamp by hand.",
            "On a clear night I count the flashes till I fall asleep.",
        ],
    },
    Subject {
        about: "the ferry",
        said: &[
            "It's never once been on time, and I'd miss it if it were.",
            "I nearly left on it once. Glad I didn't.",
        ],
    },
    Subject {
        about: "the mainland",
        said: &[
            "I go twice a year and come back tired.",
            "Too many cars. Too few boats.",
        ],
    },
    Subject {
        about: "the winter storms",
        said: &[
            "The whole island holds its breath.",
            "I like them, secretly. Everyone's indoors together.",
        ],
    },
    Subject {
        about: "the seals",
        said: &[
            "There's one that follows my boat. I call her Maud.",
            "They watch us like we're the odd ones.",
        ],
    },
    Subject {
        about: "the old chapel",
        said: &[
            "Nobody's married there in years. It's still lovely.",
            "The roof lets in more light than the windows.",
        ],
    },
    Subject {
        about: "the tides",
        said: &[
            "You live by them here, whether you like it or not.",
            "Spring tides come right up to my door.",
        ],
    },
    Subject {
        about: "fishing",
        said: &[
            "I'm no good at it, and I love it anyway.",
            "It's all waiting, really. I'm good at waiting.",
        ],
    },
    Subject {
        about: "the gulls",
        said: &[
            "They've stolen three of my pasties this year.",
            "Noisy things. I'd miss them.",
        ],
    },
    Subject {
        about: "the Saturday market",
        said: &[
            "I go for the gossip, not the cabbages.",
            "Same stalls every week, and I never tire of it.",
        ],
    },
    Subject {
        about: "baking day",
        said: &[
            "The whole lane smells of it.",
            "Fresh bread is the only reason I get up some days.",
        ],
    },
    Subject {
        about: "the school",
        said: &[
            "I learned to read in that room.",
            "It's small, but it's ours.",
        ],
    },
    Subject {
        about: "the harbour fund",
        said: &[
            "Every coin in that tin has a story.",
            "I put in what I can. Never enough.",
        ],
    },
    Subject {
        about: "the old days",
        said: &[
            "It was quieter. Not better, mind.",
            "People say it was better. I'm not so sure.",
        ],
    },
    Subject {
        about: "the weather",
        said: &[
            "You can smell rain coming here, you know.",
            "Four seasons in a day, most days.",
        ],
    },
    Subject {
        about: "the island",
        said: &[
            "I've lived here all my life, or near enough.",
            "It took me years to call it home.",
        ],
    },
    Subject {
        about: "the stars",
        said: &[
            "No streetlights here. You see all of them.",
            "I learned their names from my mother.",
        ],
    },
    Subject {
        about: "the cliffs",
        said: &[
            "I walk them every Sunday, rain or shine.",
            "Don't go near the edge in a wind.",
        ],
    },
    Subject {
        about: "the island's cats",
        said: &[
            "There's more of them than of us.",
            "They all belong to everyone.",
        ],
    },
    Subject {
        about: "the boats",
        said: &[
            "Every boat here has a name and a temper.",
            "You can tell who's out by the colour of the sails.",
        ],
    },
];

/// What people speak of in the harbour's later days: kept after the
/// festivals and works, so a World from before they were written still
/// knows what everyone has already spoken of.
const LATER_SUBJECTS: &[Subject] = &[
    Subject {
        about: "the fog",
        said: &[
            "When it comes in, the island shrinks to the end of your nose.",
            "I find my way home by the smell of the bakery.",
        ],
    },
    Subject {
        about: "the first swim of the year",
        said: &[
            "Always on the first of May. Always a mistake.",
            "You don't swim, exactly. You scream and get out.",
        ],
    },
    Subject {
        about: "the puffins",
        said: &[
            "They come back the same week every year. How do they know?",
            "Clowns of the sea, my gran called them.",
        ],
    },
    Subject {
        about: "the shipping forecast",
        said: &[
            "I fall asleep to it. Dogger, Fisher, German Bight.",
            "My dad wouldn't let anyone speak while it was on.",
        ],
    },
    Subject {
        about: "the island's wells",
        said: &[
            "Every well tastes different. I could tell you which is which blindfolded.",
            "Some say the old well by the chapel is lucky.",
        ],
    },
    Subject {
        about: "the blackberry lanes",
        said: &[
            "September, purple fingers, scratched arms. Best month.",
            "Everyone has a secret bush. Mine's the best.",
        ],
    },
    Subject {
        about: "the lifeboat",
        said: &[
            "When the maroons go up, every man on the island runs.",
            "I've never been out in her. I hope I never need to.",
        ],
    },
    Subject {
        about: "the old school",
        said: &[
            "There were only six of us. We had the teacher to ourselves.",
            "The roof leaked then, too. Some things don't change.",
        ],
    },
    Subject {
        about: "the harvest moon",
        said: &[
            "It sits on the sea like an orange. Makes you want to row to it.",
            "We used to stay up for it, all of us on the quay.",
        ],
    },
    Subject {
        about: "the mackerel",
        said: &[
            "When they come in, the whole sea boils silver.",
            "Fried with a bit of oat. Nothing better.",
        ],
    },
    Subject {
        about: "the heather on the hill",
        said: &[
            "In August the hill goes purple overnight.",
            "The bees go mad for it. So do I.",
        ],
    },
    Subject {
        about: "the fishermen's songs",
        said: &[
            "My grandad knew forty. I know four.",
            "You sing them hauling nets. They make the rope lighter.",
        ],
    },
    Subject {
        about: "the old wreck",
        said: &[
            "Every child dares every other child to swim out to it.",
            "She was carrying oranges, they say. The island ate well that winter.",
        ],
    },
    Subject {
        about: "the winter nights",
        said: &[
            "Dark by four. You learn to love a lamp.",
            "Cards, soup and stories. That's winter.",
        ],
    },
    Subject {
        about: "the spring tides",
        said: &[
            "Twice a month the sea comes right up to the doors.",
            "You can walk out to the point at low spring tide. Just.",
        ],
    },
    Subject {
        about: "the island's ghosts",
        said: &[
            "Nobody believes in them. Nobody walks the cliff path at night either.",
            "The grey lady at the chapel? My aunt saw her. Twice.",
        ],
    },
    Subject {
        about: "the ferryman",
        said: &[
            "He's done the crossing forty years. Never once seasick.",
            "He knows everybody's business by the parcels.",
        ],
    },
    Subject {
        about: "the dances",
        said: &[
            "They used to clear the net loft and dance till dawn.",
            "My parents met at one. Everyone's did.",
        ],
    },
    Subject {
        about: "the rain",
        said: &[
            "Sideways, mostly. Island rain doesn't fall, it arrives.",
            "The smell after rain on hot stone. Nothing like it.",
        ],
    },
    Subject {
        about: "the sea in summer",
        said: &[
            "Still as glass some mornings. You could walk on it.",
            "Blue like nowhere else. Don't tell the mainland.",
        ],
    },
    Subject {
        about: "the chapel bell",
        said: &[
            "It hasn't rung in fifty years. I still listen for it.",
            "They say it rang by itself the night of the great storm.",
        ],
    },
    Subject {
        about: "the island's dogs",
        said: &[
            "Every dog here thinks it runs the harbour.",
            "Old Bess walks the quay at six every morning. Nobody owns her.",
        ],
    },
    Subject {
        about: "the cliffs in spring",
        said: &[
            "Thrift everywhere, pink as sugar.",
            "The birds come back and the cliffs get noisy again.",
        ],
    },
    Subject {
        about: "the kelp harvest",
        said: &[
            "We used to burn it for soda. The smoke hung for days.",
            "It's good for the gardens. Smells awful, works wonders.",
        ],
    },
    Subject {
        about: "the tea",
        said: &[
            "Strong enough to stand a spoon in. That's island tea.",
            "Everything here stops for tea. Even storms, nearly.",
        ],
    },
    Subject {
        about: "the old ways",
        said: &[
            "We used to know the weather by the birds. Now we ask the wireless.",
            "Some things were better. Most weren't.",
        ],
    },
    Subject {
        about: "the harbour at night",
        said: &[
            "The lights on the water, and the boats knocking together.",
            "I walk it sometimes, when I can't sleep.",
        ],
    },
    Subject {
        about: "the mainland shops",
        said: &[
            "Twelve kinds of cheese. Who needs twelve?",
            "I came back with nothing I went for. Every time.",
        ],
    },
    Subject {
        about: "frost on the quay",
        said: &[
            "The quay goes white and everyone slides to work.",
            "It means the kippers are ready. That's how my dad told it.",
        ],
    },
    Subject {
        about: "the swallows leaving",
        said: &[
            "One day they're on the wires, the next they're gone.",
            "I always feel it, the day they go.",
        ],
    },
    Subject {
        about: "the fish auctions",
        said: &[
            "Fast as anything. Blink and you've bought a crate of cod.",
            "The auctioneer's voice carries to the point.",
        ],
    },
    Subject {
        about: "the island's roads",
        said: &[
            "One road, three names, and it goes nowhere much.",
            "Two cars on the whole island. They still manage to meet.",
        ],
    },
    Subject {
        about: "the wind",
        said: &[
            "It never stops. When it does, everyone looks up.",
            "You learn to lean into it. Walk straight on the mainland and you fall over.",
        ],
    },
    Subject {
        about: "the lighthouse keepers",
        said: &[
            "The last one left when it went automatic. He cried, they say.",
            "They kept a log of every ship. I've read some. Lovely writing.",
        ],
    },
    Subject {
        about: "the midsummer bonfire",
        said: &[
            "Everyone jumps it once. For luck.",
            "The sun barely sets. We stay out all night.",
        ],
    },
    Subject {
        about: "the island's bees",
        said: &[
            "The heather honey here is dark as treacle.",
            "You tell the bees your news, or they leave. Old island rule.",
        ],
    },
    Subject {
        about: "the post",
        said: &[
            "Twice a week, if the ferry comes. We read it on the quay.",
            "A letter here still feels like a gift.",
        ],
    },
    Subject {
        about: "the tidy harbour prize",
        said: &[
            "We came second in 1987. Nobody's forgotten who won.",
            "Somebody still has the rosette. Second place.",
        ],
    },
    Subject {
        about: "the island's children",
        said: &[
            "There used to be forty. Now there's a handful, and they run the place.",
            "They know every rock and every rule. And break them all.",
        ],
    },
    Subject {
        about: "the old boats",
        said: &[
            "Wood breathes. Fibreglass doesn't. That's all I'll say.",
            "My first boat was older than my dad.",
        ],
    },
    Subject {
        about: "the long summer evenings",
        said: &[
            "Light till eleven. You forget to go to bed.",
            "The best talk happens after nine, on the quay.",
        ],
    },
    Subject {
        about: "the island's stones",
        said: &[
            "Every stone wall here was built by hand. No mortar.",
            "There's a stone on the hill that hums when the wind's right.",
        ],
    },
    Subject {
        about: "the seaweed",
        said: &[
            "Some of it you can eat. Ask before you try.",
            "After a gale the beach is knee-deep in it.",
        ],
    },
    Subject {
        about: "the otters",
        said: &[
            "If you see one, don't tell anyone. They like their peace.",
            "They play in the harbour at dusk. I swear it.",
        ],
    },
    Subject {
        about: "the island's recipes",
        said: &[
            "Every family has a stew. Every family says theirs is right.",
            "My gran's fish pie. Nobody gets the recipe.",
        ],
    },
    Subject {
        about: "the old maps",
        said: &[
            "The old maps put the island twice the size. We liked that.",
            "There's a map in the chapel with a dragon on the point.",
        ],
    },
    Subject {
        about: "the lambing",
        said: &[
            "Spring on the hill farm means nobody sleeps.",
            "I helped once. Once.",
        ],
    },
    Subject {
        about: "the northern lights",
        said: &[
            "Twice in my life. Green and pink over the sea.",
            "The children want to see them more than anything.",
        ],
    },
    Subject {
        about: "the market bell",
        said: &[
            "When it rings, run, or the best fish goes.",
            "It's cracked. It still does the job.",
        ],
    },
    Subject {
        about: "the gales",
        said: &[
            "Force nine and the whole harbour groans.",
            "You tie everything down and hope.",
        ],
    },
    Subject {
        about: "the island's accent",
        said: &[
            "The mainland thinks we sing when we talk.",
            "You lose it if you leave. It comes back the minute you're home.",
        ],
    },
    Subject {
        about: "the rock pools",
        said: &[
            "Anemones like jam. Don't poke them.",
            "I could sit by one for hours. I have.",
        ],
    },
    Subject {
        about: "the old postmistress",
        said: &[
            "She knew every letter before you opened it.",
            "She kept sweets under the counter for the children.",
        ],
    },
    Subject {
        about: "the Christmas lights",
        said: &[
            "One string, round the harbour. It's enough.",
            "Every year somebody falls off the ladder. Every year.",
        ],
    },
    Subject {
        about: "the island's sheep",
        said: &[
            "They walk on the road like they own it. They do.",
            "One ate my washing. Wool for wool, I suppose.",
        ],
    },
    Subject {
        about: "the storm of '53",
        said: &[
            "My gran talked about it till the day she died.",
            "The sea came right into the chapel.",
        ],
    },
    Subject {
        about: "the quiet",
        said: &[
            "On the mainland it's never quiet. Here you hear your own heart.",
            "Some folk can't stand it. I can't live without it.",
        ],
    },
    Subject {
        about: "the old fiddler",
        said: &[
            "He played at every wedding for sixty years.",
            "When he died the whole island went quiet for a week.",
        ],
    },
    Subject {
        about: "the tide clock",
        said: &[
            "Round here you tell time by the water, not the hour.",
            "Everything waits on the tide. Even weddings.",
        ],
    },
    Subject {
        about: "the ginger cat",
        said: &[
            "They know when the boats come in before we do.",
            "He has a different name in every house.",
        ],
    },
    Subject {
        about: "the ferry timetable",
        said: &[
            "It's more of a hope than a timetable.",
            "I know it by heart. Much good it does me.",
        ],
    },
    Subject {
        about: "the old lifeboat crews",
        said: &[
            "Every family on the island gave someone to the lifeboat.",
            "They went out in weather I wouldn't open a door to.",
        ],
    },
    Subject {
        about: "the winter feasts",
        said: &[
            "Everyone brings what they've got. It's always enough.",
            "The longest night deserves the biggest pie.",
        ],
    },
    Subject {
        about: "the harbour wall",
        said: &[
            "My grandad helped build it. He talked about every stone.",
            "Stand on it in a gale and you feel it shake.",
        ],
    },
    Subject {
        about: "the lobsters",
        said: &[
            "Clever things. They know a pot when they see one.",
            "Blue when they come up, red when they're cooked. Magic.",
        ],
    },
    Subject {
        about: "the first ferry of spring",
        said: &[
            "Everyone comes down to the quay to see it in.",
            "It brings oranges, post and news. Mostly oranges.",
        ],
    },
    Subject {
        about: "the old songs",
        said: &[
            "Half of them are about drowning. We sing them anyway.",
            "My mother sang one to me every night. I've forgotten the end.",
        ],
    },
    Subject {
        about: "the tides in winter",
        said: &[
            "They come right up to the pub door. The pub hates it.",
            "At the big tides you can hear the stones moving.",
        ],
    },
    Subject {
        about: "the island's paths",
        said: &[
            "Every path goes to the sea, sooner or later.",
            "Some paths only the sheep know.",
        ],
    },
    Subject {
        about: "the bread",
        said: &[
            "The bakery's bread is why half of us stay.",
            "Warm bread on a cold morning. That's the island.",
        ],
    },
    Subject {
        about: "the mainland news",
        said: &[
            "It's always a week late here. It never matters.",
            "Big things happen over there. Small things happen here. I prefer small.",
        ],
    },
    Subject {
        about: "the moon on the water",
        said: &[
            "A road of light, right to the door.",
            "My gran said you could walk it, if you were brave enough.",
        ],
    },
    Subject {
        about: "the island's first settlers",
        said: &[
            "They came in open boats. I can't imagine it.",
            "Three families, one cow and a lot of stubbornness.",
        ],
    },
    Subject {
        about: "the kittiwakes",
        said: &[
            "Their cry sounds like their name. Listen.",
            "They nest on the cliffs where nothing else can.",
        ],
    },
    Subject {
        about: "the island's weddings",
        said: &[
            "Everyone's invited. Everyone comes. Every time.",
            "The whole island dances on the quay till the tide comes in.",
        ],
    },
    Subject {
        about: "the sea mist",
        said: &[
            "It comes in off the water like a slow wave.",
            "You can hear the boats but you can't see them. Eerie.",
        ],
    },
    Subject {
        about: "the old ferryman's hut",
        said: &[
            "He used to give children a boiled sweet for the crossing.",
            "It still smells of pipe smoke, after all these years.",
        ],
    },
    Subject {
        about: "the island's birthdays",
        said: &[
            "Nobody's birthday goes unnoticed here. Almost nobody.",
            "A cake, a song and the whole pub joining in.",
        ],
    },
    Subject {
        about: "the shells on the beach",
        said: &[
            "Every storm brings new ones. The children count them.",
            "The cowries are lucky. Keep one in your pocket.",
        ],
    },
    Subject {
        about: "the harbour at dawn",
        said: &[
            "The boats go out before the gulls wake.",
            "Everything's grey, then gold, then blue. Every morning.",
        ],
    },
    Subject {
        about: "the winter swim",
        said: &[
            "Only the brave or the daft. I'm both.",
            "You come out pink and shouting. Best feeling there is.",
        ],
    },
    Subject {
        about: "the island's fiddles",
        said: &[
            "There's a fiddle in every other house. Half of them in tune.",
            "When the fiddles come out, nobody sits still.",
        ],
    },
    Subject {
        about: "the crab races",
        said: &[
            "Every summer on the quay. Mine always goes sideways.",
            "The children train them. It doesn't help.",
        ],
    },
    Subject {
        about: "the tide pools at night",
        said: &[
            "Take a torch. The whole pool lights up with eyes.",
            "Things come out at night you'd never see by day.",
        ],
    },
    Subject {
        about: "the island's weather sayings",
        said: &[
            "Red sky at night, the fishers' delight. Mostly true.",
            "When the gulls come inland, tie down your washing.",
        ],
    },
    Subject {
        about: "the old mill",
        said: &[
            "It ground the island's flour for a hundred years.",
            "The wheel still turns when the stream's high. Nobody knows why.",
        ],
    },
    Subject {
        about: "the island's patience",
        said: &[
            "Everything takes longer here. It's better for it.",
            "You wait for the ferry, the tide, the weather. You learn.",
        ],
    },
    Subject {
        about: "the lighthouse beam",
        said: &[
            "It sweeps across my ceiling all night. I count it like sheep.",
            "Every ship for miles knows where we are because of it.",
        ],
    },
    Subject {
        about: "the dolphins",
        said: &[
            "Twice a year they come right into the harbour.",
            "Everyone stops what they're doing. Everyone.",
        ],
    },
    Subject {
        about: "the harbour fund's old ledger",
        said: &[
            "Every penny since 1910. Some of the entries are very funny.",
            "Somebody once spent the whole fund on a brass band.",
        ],
    },
    Subject {
        about: "the island's quiet Sundays",
        said: &[
            "No boats, no bells. Just the sea.",
            "Sunday is for walking the cliffs and saying nothing.",
        ],
    },
    Subject {
        about: "the island's lost things",
        said: &[
            "Everything lost turns up on the beach eventually.",
            "I found a ring once. Gave it back. Got a pie for it.",
        ],
    },
    Subject {
        about: "the first snow",
        said: &[
            "It never lasts. Everyone rushes out anyway.",
            "The children build one snowman. It's gone by noon.",
        ],
    },
    Subject {
        about: "the harbour's colours",
        said: &[
            "Every boat is painted a family colour. Ours is green.",
            "Blue doors, red boats and white walls. That's us.",
        ],
    },
    Subject {
        about: "the old net knots",
        said: &[
            "My grandmother could tie them in the dark.",
            "There's a knot for every job. I know six.",
        ],
    },
    Subject {
        about: "the island's storytellers",
        said: &[
            "Every pub needs someone who can tell a story. We have three.",
            "The best stories get better every time.",
        ],
    },
    Subject {
        about: "the view from the chapel",
        said: &[
            "You can see the whole island and half the sea.",
            "On a clear day, the mainland. On a better day, not.",
        ],
    },
    Subject {
        about: "the summer visitors",
        said: &[
            "They come for a week and talk about it for a year.",
            "Some of them never leave. Look at half of us.",
        ],
    },
    Subject {
        about: "the old boat names",
        said: &[
            "Every boat is named for someone. Some for someone's mother-in-law.",
            "Sea Finch, Kittiwake, Brave Molly. Each one a story.",
        ],
    },
];

/// Everything people can speak of for the first time: the harbour's own
/// subjects, then its days of the year and its works, which are spoken of
/// in a few words that fit anything.
fn subjects() -> &'static [Subject] {
    static ALL: std::sync::OnceLock<Vec<Subject>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut all = SUBJECTS.to_vec();
        let festivals = crate::almanac::FESTIVALS
            .iter()
            .chain(crate::years::NEW_FESTIVALS)
            .map(|festival| festival.name);
        let works = crate::story::WORKS
            .iter()
            .map(|work| crate::story::leak(crate::story::the(work.label)));
        for about in festivals.chain(works) {
            if !all.iter().any(|subject| subject.about == about) {
                all.push(Subject { about, said: &[] });
            }
        }
        for subject in LATER_SUBJECTS {
            if !all.iter().any(|known| known.about == subject.about) {
                all.push(*subject);
            }
        }
        all
    })
}

/// How the harbour keeps its quiet days.
pub(crate) fn quiet_days() -> QuietDays {
    QuietDays {
        letters_a_week: Some(LETTERS_A_WEEK),
        letter_weeks: LETTER_WEEKS,
        still_every: Some(STILL_EVERY),
        corners: CORNERS,
        subjects: subjects(),
    }
}
