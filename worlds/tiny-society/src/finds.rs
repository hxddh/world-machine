//! The book of finds (1.0 P4): what a sitting's morning can turn up, by
//! the place it turns up at and the season it turns up in, each once. A
//! season of mornings never runs short: each of the four places keeps
//! eight things for each season, so a player who comes every day finds
//! something new for the book every morning of the year, and the book is
//! never finished in under a year.

use ritual::Find;
use world_core::WorldState;

const SPRING: usize = 0;
const SUMMER: usize = 1;
const AUTUMN: usize = 2;
const WINTER: usize = 3;

/// The places, by the names the harbour gives them.
const HARBOUR: &str = "the harbour";
const PUB: &str = "Anchor Pub";
const BAKERY: &str = "Harbour Bakery";
const SCHOOL: &str = "Island School";

/// Everything the mornings can turn up, season by season, the places in
/// turn.
pub(crate) const FINDS: &[Find] = &[
    Find::at("a gull's feather", SPRING, HARBOUR),
    Find::at("a brass button", SPRING, PUB),
    Find::at("a chipped blue cup", SPRING, BAKERY),
    Find::at("a child's paper boat", SPRING, SCHOOL),
    Find::at("a piece of green sea glass", SPRING, HARBOUR),
    Find::at("a key on a string", SPRING, PUB),
    Find::at("a copper nail", SPRING, BAKERY),
    Find::at("a tin soldier", SPRING, SCHOOL),
    Find::at("a cork float off a net", SPRING, HARBOUR),
    Find::at("a carved whistle", SPRING, PUB),
    Find::at("a wooden butter stamp", SPRING, BAKERY),
    Find::at("a spool of red thread", SPRING, SCHOOL),
    Find::at("a starfish", SPRING, HARBOUR),
    Find::at("a beer mat with a poem on it", SPRING, PUB),
    Find::at("a paper bag of currants", SPRING, BAKERY),
    Find::at("an empty bird's nest", SPRING, SCHOOL),
    Find::at("a pressed sea pink", SPRING, HARBOUR),
    Find::at("a darts flight", SPRING, PUB),
    Find::at("a robin's feather", SPRING, BAKERY),
    Find::at("a stub of blue chalk", SPRING, SCHOOL),
    Find::at("a skimming stone", SPRING, HARBOUR),
    Find::at("the queen of hearts from a pack of cards", SPRING, PUB),
    Find::at("a sprig of primroses", SPRING, BAKERY),
    Find::at("a frog drawn in crayon", SPRING, SCHOOL),
    Find::at("a tern's eggshell", SPRING, HARBOUR),
    Find::at("a sprig of pussy willow", SPRING, PUB),
    Find::at("a bread tin with a dent in it", SPRING, BAKERY),
    Find::at("a skipping rope", SPRING, SCHOOL),
    Find::at("a tangle of new kelp", SPRING, HARBOUR),
    Find::at("a forgotten umbrella", SPRING, PUB),
    Find::at("a sugar mouse", SPRING, BAKERY),
    Find::at("a jar of frogspawn", SPRING, SCHOOL),
    Find::at("a scallop shell", SUMMER, HARBOUR),
    Find::at("a ribbon from the fair", SUMMER, PUB),
    Find::at("a jar of beach plums", SUMMER, BAKERY),
    Find::at("a fishing fly", SUMMER, SCHOOL),
    Find::at("a crab's claw", SUMMER, HARBOUR),
    Find::at("a lucky horseshoe", SUMMER, PUB),
    Find::at("a strawberry punnet", SUMMER, BAKERY),
    Find::at("a kite with a broken string", SUMMER, SCHOOL),
    Find::at("a mermaid's purse", SUMMER, HARBOUR),
    Find::at("a straw sun hat", SUMMER, PUB),
    Find::at("a bunch of sweet peas", SUMMER, BAKERY),
    Find::at("a butterfly net", SUMMER, SCHOOL),
    Find::at("a bottle with a note in it", SUMMER, HARBOUR),
    Find::at("a fiddle string", SUMMER, PUB),
    Find::at("the lid of a cake tin", SUMMER, BAKERY),
    Find::at("a sports day rosette", SUMMER, SCHOOL),
    Find::at("a sun-bleached sandal", SUMMER, HARBOUR),
    Find::at("a raffle ticket", SUMMER, PUB),
    Find::at("a jar of honeycomb", SUMMER, BAKERY),
    Find::at("a daisy chain", SUMMER, SCHOOL),
    Find::at("a bucket and spade", SUMMER, HARBOUR),
    Find::at("a bottle cap with a sailboat on it", SUMMER, PUB),
    Find::at("a recipe card in faded ink", SUMMER, BAKERY),
    Find::at("a marble with a blue swirl", SUMMER, SCHOOL),
    Find::at("a sand dollar", SUMMER, HARBOUR),
    Find::at("a quiz night answer sheet", SUMMER, PUB),
    Find::at("a cherry stone", SUMMER, BAKERY),
    Find::at("a sunflower head full of seeds", SUMMER, SCHOOL),
    Find::at("a piece of driftwood shaped like a fish", SUMMER, HARBOUR),
    Find::at("a paper fan", SUMMER, PUB),
    Find::at("an empty lemonade bottle", SUMMER, BAKERY),
    Find::at("a class photograph", SUMMER, SCHOOL),
    Find::at("an old coin", AUTUMN, HARBOUR),
    Find::at("a conker on a string", AUTUMN, PUB),
    Find::at("a basket of blackberries", AUTUMN, BAKERY),
    Find::at("a pine cone", AUTUMN, SCHOOL),
    Find::at("a page of an old sea chart", AUTUMN, HARBOUR),
    Find::at("a ticket to the harvest supper", AUTUMN, PUB),
    Find::at("a hazelnut", AUTUMN, BAKERY),
    Find::at("a leaf with a face drawn on it", AUTUMN, SCHOOL),
    Find::at("a ship in a bottle", AUTUMN, HARBOUR),
    Find::at("a pewter tankard lid", AUTUMN, PUB),
    Find::at("a windfall apple", AUTUMN, BAKERY),
    Find::at("an exercise book", AUTUMN, SCHOOL),
    Find::at("a knotted rope bracelet", AUTUMN, HARBOUR),
    Find::at("a chess knight", AUTUMN, PUB),
    Find::at("a jar of bramble jelly", AUTUMN, BAKERY),
    Find::at("a cracked protractor", AUTUMN, SCHOOL),
    Find::at("a smooth white pebble", AUTUMN, HARBOUR),
    Find::at("a candle stub", AUTUMN, PUB),
    Find::at("a wheat sheaf ornament", AUTUMN, BAKERY),
    Find::at("a paper owl", AUTUMN, SCHOOL),
    Find::at("a lobster-pot buoy", AUTUMN, HARBOUR),
    Find::at("a woollen scarf", AUTUMN, PUB),
    Find::at("a pumpkin seed", AUTUMN, BAKERY),
    Find::at("a paper lantern", AUTUMN, SCHOOL),
    Find::at("a piece of amber", AUTUMN, HARBOUR),
    Find::at("a pressed oak leaf", AUTUMN, PUB),
    Find::at("a glove white with flour", AUTUMN, BAKERY),
    Find::at("a ruler with initials carved in it", AUTUMN, SCHOOL),
    Find::at("a heron's feather", AUTUMN, HARBOUR),
    Find::at("a box of matches from the city", AUTUMN, PUB),
    Find::at("the handle of a rolling pin", AUTUMN, BAKERY),
    Find::at("a bag of marbles", AUTUMN, SCHOOL),
    Find::at("a lost mitten", WINTER, HARBOUR),
    Find::at("a sprig of holly", WINTER, PUB),
    Find::at("a gingerbread star", WINTER, BAKERY),
    Find::at("a red woolly hat", WINTER, SCHOOL),
    Find::at("a periwinkle shell", WINTER, HARBOUR),
    Find::at("a paper crown from a cracker", WINTER, PUB),
    Find::at("a cinnamon stick", WINTER, BAKERY),
    Find::at("a snowman's carrot nose", WINTER, SCHOOL),
    Find::at("a seal's whisker", WINTER, HARBOUR),
    Find::at("a carol sheet", WINTER, PUB),
    Find::at("a curl of candied orange peel", WINTER, BAKERY),
    Find::at("a sledge rope", WINTER, SCHOOL),
    Find::at("a piece of glass like an icicle", WINTER, HARBOUR),
    Find::at("a hot-water bottle", WINTER, PUB),
    Find::at("a wooden spoon", WINTER, BAKERY),
    Find::at("a slate with sums on it", WINTER, SCHOOL),
    Find::at("a sailor's woollen hat", WINTER, HARBOUR),
    Find::at("a roast chestnut", WINTER, PUB),
    Find::at("a snow globe", WINTER, BAKERY),
    Find::at("a storybook with a torn cover", WINTER, SCHOOL),
    Find::at("a brass compass", WINTER, HARBOUR),
    Find::at("a bobble off a hat", WINTER, PUB),
    Find::at("an orange stuck with cloves", WINTER, BAKERY),
    Find::at("a glove puppet", WINTER, SCHOOL),
    Find::at("a snow-white feather", WINTER, HARBOUR),
    Find::at("a tin of shortbread", WINTER, PUB),
    Find::at("a paper snowflake", WINTER, BAKERY),
    Find::at("a wooden spinning top", WINTER, SCHOOL),
    Find::at("a whelk shell", WINTER, HARBOUR),
    Find::at("a dart with a bent tip", WINTER, PUB),
    Find::at("a pie dish", WINTER, BAKERY),
    Find::at("a paper star for the window", WINTER, SCHOOL),
];

/// The season it is in the harbour, from 0 (spring) to 3 (winter).
pub(crate) fn season(state: &WorldState) -> Option<usize> {
    Some(crate::town::season_of(state).0.index() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// The book of finds holds at least 120, each once, spread over the
    /// four places and the four seasons: never fewer in a season than its
    /// mornings, nor fewer than eight at any place in any season.
    #[test]
    fn the_book_of_finds_is_spread_over_places_and_seasons() {
        assert!(FINDS.len() >= 120, "{}", FINDS.len());
        let names = FINDS.iter().map(|find| find.what).collect::<BTreeSet<_>>();
        assert_eq!(names.len(), FINDS.len(), "a find listed twice");
        let mut cells = BTreeMap::<(usize, &str), usize>::new();
        for find in FINDS {
            let season = find.season.expect("a season");
            let near = find.near.expect("a place");
            assert!(season <= WINTER);
            assert!([HARBOUR, PUB, BAKERY, SCHOOL].contains(&near), "{near}");
            *cells.entry((season, near)).or_default() += 1;
        }
        assert_eq!(cells.len(), 16, "{cells:?}");
        assert!(cells.values().all(|n| *n >= 8), "{cells:?}");
        for season in [SPRING, SUMMER, AUTUMN, WINTER] {
            let in_season = FINDS
                .iter()
                .filter(|find| find.season == Some(season))
                .count();
            assert!(
                in_season as u64 >= crate::story::SEASON_DAYS,
                "{in_season} finds in season {season}"
            );
        }
    }

    /// Every find is in the harbour's Chinese and Japanese.
    #[test]
    fn every_find_is_in_both_catalogs() {
        for catalog in [crate::ZH_HANS, crate::JA] {
            let keys = catalog
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .map(|(english, _)| english)
                .collect::<BTreeSet<_>>();
            for find in FINDS {
                assert!(keys.contains(find.what), "untranslated: {}", find.what);
            }
        }
    }
}
