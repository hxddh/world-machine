//! Names a person here could not know, by their shape: someone with a
//! title, a given name, a name in Latin letters inside Chinese or Japanese,
//! a katakana name with an honorific, a Chinese surname and given name; a
//! local place (an inn, a cave, a cannery) or a piece of local history (the
//! Great Fire of some year) the World never had.
//!
//! A name is first asked of the World's own lexicon ([`Grounds::knows_name`]:
//! every name its lines can say, in every language it speaks), then of the
//! names anyone anywhere knows (Jack Frost, Father Christmas) and of the real
//! world's geography and history, which a person of the World's time may know
//! of (the outside-world policy: real places and events of the era are in;
//! brands, celebrities, politicians, media titles and real currency are out,
//! and those are the `outside` checks' to find). Only a name none of them
//! knows, written in a shape that says what it is, is invented. A bare name
//! of no particular shape (a song, a game, a town over the hills) is left to
//! the time and outside checks.

use super::text::{is_han, Text};
use super::Grounds;

/// What an invented name is, by its shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invented {
    Person,
    Place,
    History,
}

/// Titles that go before a person's name, in lower case.
const PERSON_TITLES: &[&str] = &[
    "mr",
    "mrs",
    "ms",
    "miss",
    "mister",
    "missus",
    "master",
    "mistress",
    "madam",
    "madame",
    "dr",
    "doctor",
    "doc",
    "nurse",
    "matron",
    "captain",
    "capt",
    "skipper",
    "bosun",
    "mate",
    "commander",
    "colonel",
    "major",
    "lieutenant",
    "admiral",
    "general",
    "sergeant",
    "sgt",
    "corporal",
    "private",
    "constable",
    "officer",
    "inspector",
    "sheriff",
    "deputy",
    "marshal",
    "mayor",
    "alderman",
    "alderwoman",
    "councillor",
    "councilman",
    "governor",
    "senator",
    "judge",
    "magistrate",
    "professor",
    "prof",
    "reverend",
    "rev",
    "pastor",
    "parson",
    "vicar",
    "rector",
    "deacon",
    "bishop",
    "father",
    "mother",
    "brother",
    "sister",
    "friar",
    "abbot",
    "elder",
    "chief",
    "lady",
    "lord",
    "sir",
    "dame",
    "baron",
    "baroness",
    "duke",
    "duchess",
    "count",
    "countess",
    "king",
    "queen",
    "prince",
    "princess",
    "widow",
    "aunt",
    "auntie",
    "aunty",
    "uncle",
    "cousin",
    "granny",
    "grandma",
    "grandpa",
    "gramps",
    "nana",
    "nan",
    "grandad",
    "grandmother",
    "grandfather",
    "old",
    "young",
    "little",
    "big",
    "farmer",
    "fisherman",
    "teacher",
    "headmaster",
    "headmistress",
    "nanny",
    "saint",
    "st",
    "director",
    "president",
    "foreman",
    "keeper",
    "warden",
];

/// Common given names in Latin letters, in lower case: a single one of
/// them used as a name is a person. Names that are also places (Florence,
/// Victoria, Devon) are left out.
const GIVEN_NAMES: &[&str] = &[
    "aaron",
    "abel",
    "abigail",
    "ada",
    "adam",
    "agatha",
    "agnes",
    "alan",
    "albert",
    "alberto",
    "alec",
    "alex",
    "alexander",
    "alexandra",
    "alfie",
    "alfred",
    "alice",
    "alison",
    "amelia",
    "amos",
    "amy",
    "andrea",
    "andrew",
    "andy",
    "angela",
    "angus",
    "anna",
    "anne",
    "annie",
    "anthony",
    "archie",
    "arnold",
    "arthur",
    "audrey",
    "barbara",
    "barnaby",
    "barney",
    "bart",
    "basil",
    "beatrice",
    "ben",
    "benjamin",
    "bernard",
    "bert",
    "bertha",
    "bertie",
    "betty",
    "beverly",
    "bill",
    "billy",
    "bob",
    "bobby",
    "boris",
    "brian",
    "bridget",
    "bruce",
    "bruno",
    "callum",
    "carl",
    "carlos",
    "carol",
    "caroline",
    "cassius",
    "catherine",
    "cecil",
    "cecily",
    "cedric",
    "charles",
    "charlie",
    "chloe",
    "chris",
    "christine",
    "christopher",
    "clara",
    "clarence",
    "claude",
    "clifford",
    "clive",
    "colin",
    "connor",
    "conrad",
    "cornelius",
    "craig",
    "cyril",
    "daniel",
    "danny",
    "dave",
    "david",
    "deborah",
    "dennis",
    "derek",
    "diana",
    "dick",
    "dolly",
    "dominic",
    "donald",
    "doris",
    "dorothy",
    "doug",
    "duncan",
    "dylan",
    "edgar",
    "edith",
    "edmund",
    "edna",
    "edward",
    "edwin",
    "eileen",
    "elaine",
    "eleanor",
    "eli",
    "elias",
    "elijah",
    "eliza",
    "ella",
    "ellen",
    "elliot",
    "eloise",
    "elsa",
    "emily",
    "enid",
    "eric",
    "ernest",
    "ernie",
    "esther",
    "ethel",
    "eugene",
    "eunice",
    "eva",
    "eve",
    "felix",
    "fiona",
    "fitzgerald",
    "flora",
    "frances",
    "francis",
    "frank",
    "frankie",
    "fred",
    "freddie",
    "frederick",
    "gabriel",
    "gareth",
    "gavin",
    "geoffrey",
    "george",
    "gerald",
    "gertrude",
    "gilbert",
    "giles",
    "gladys",
    "glenn",
    "gloria",
    "gordon",
    "graham",
    "gregory",
    "gwen",
    "gwendolyn",
    "hamish",
    "hank",
    "hannah",
    "harold",
    "harriet",
    "harry",
    "harvey",
    "hector",
    "helen",
    "henrietta",
    "henry",
    "herbert",
    "horace",
    "howard",
    "ian",
    "ida",
    "igor",
    "imogen",
    "irene",
    "isaac",
    "isabel",
    "isabella",
    "isadora",
    "ivan",
    "jack",
    "jacob",
    "james",
    "jamie",
    "jane",
    "janet",
    "jasper",
    "jean",
    "jeff",
    "jennifer",
    "jenny",
    "jeremiah",
    "jeremy",
    "jessica",
    "jessie",
    "jim",
    "joan",
    "joe",
    "joey",
    "john",
    "johnny",
    "jonathan",
    "joseph",
    "josephine",
    "joshua",
    "joyce",
    "judith",
    "judy",
    "julia",
    "julian",
    "julie",
    "june",
    "justine",
    "karen",
    "kate",
    "katherine",
    "kathleen",
    "katie",
    "keith",
    "ken",
    "kenneth",
    "laura",
    "laurence",
    "lawrence",
    "leonard",
    "leslie",
    "lewis",
    "liam",
    "lillian",
    "linda",
    "lionel",
    "lizzie",
    "lloyd",
    "lois",
    "louis",
    "louisa",
    "louise",
    "lucas",
    "lucy",
    "luke",
    "lydia",
    "mabel",
    "maggie",
    "magnus",
    "malcolm",
    "margaret",
    "margery",
    "maria",
    "marian",
    "marianne",
    "marie",
    "marjorie",
    "martha",
    "martin",
    "mary",
    "matilda",
    "matthew",
    "maud",
    "maureen",
    "maurice",
    "maxine",
    "melvin",
    "michael",
    "mildred",
    "millicent",
    "milly",
    "miriam",
    "molly",
    "morris",
    "mortimer",
    "muriel",
    "nancy",
    "nathan",
    "nathaniel",
    "neil",
    "nelly",
    "neville",
    "nicholas",
    "nick",
    "nigel",
    "norman",
    "octavia",
    "olive",
    "oliver",
    "olivia",
    "oscar",
    "oswald",
    "owen",
    "pamela",
    "patricia",
    "patrick",
    "paul",
    "pauline",
    "pearl",
    "peggy",
    "penelope",
    "percy",
    "peter",
    "philip",
    "phoebe",
    "phyllis",
    "pierre",
    "polly",
    "prudence",
    "quentin",
    "quincy",
    "rachel",
    "ralph",
    "randolph",
    "raymond",
    "rebecca",
    "reggie",
    "reginald",
    "rex",
    "richard",
    "rita",
    "robert",
    "roberta",
    "rodney",
    "roger",
    "roland",
    "ronald",
    "ronnie",
    "rosalind",
    "rosie",
    "ross",
    "roy",
    "rupert",
    "russell",
    "ruth",
    "sally",
    "samantha",
    "samuel",
    "sandra",
    "sarah",
    "seamus",
    "sean",
    "sebastian",
    "seraphina",
    "seth",
    "sharon",
    "sidney",
    "silas",
    "simon",
    "sophie",
    "stanley",
    "stella",
    "stephen",
    "steve",
    "stuart",
    "susan",
    "sylvia",
    "ted",
    "teddy",
    "terence",
    "thaddeus",
    "thelma",
    "theodore",
    "theresa",
    "thomas",
    "timothy",
    "toby",
    "tom",
    "tommy",
    "trevor",
    "ursula",
    "valerie",
    "vera",
    "vernon",
    "veronica",
    "victor",
    "vincent",
    "violet",
    "virgil",
    "vivian",
    "wallace",
    "walter",
    "warren",
    "wilbur",
    "wilfred",
    "wilhelmina",
    "william",
    "willie",
    "winifred",
    "winnie",
    "winston",
    "yvonne",
    "zachary",
    "zelda",
    "zoe",
    "ezekiel",
    "obadiah",
    "ebenezer",
    "jebediah",
    "mordecai",
    "bartholomew",
    "barnabas",
    "ignatius",
    "lavinia",
    "euphemia",
    "temperance",
    "hortense",
    "ambrose",
    "augustus",
    "cuthbert",
    "fergus",
    "finlay",
    "ewan",
    "rhys",
    "dafydd",
    "bronwen",
    "siobhan",
    "niamh",
    "padraig",
    "declan",
    "ciaran",
    "eamon",
    "oisin",
    "sven",
    "lars",
    "ingrid",
    "astrid",
    "olaf",
    "nils",
    "karl",
    "hans",
    "fritz",
    "greta",
    "heidi",
    "klaus",
    "dieter",
    "jurgen",
    "giuseppe",
    "giovanni",
    "marco",
    "luca",
    "francesca",
    "sofia",
    "pablo",
    "juan",
    "miguel",
    "carmen",
    "jose",
    "pedro",
    "manuel",
    "rafael",
    "natasha",
    "dmitri",
    "sergei",
    "olga",
    "svetlana",
    "anya",
    "pavel",
    "yuri",
    "vladimir",
    "jacques",
    "marcel",
    "henri",
    "colette",
    "yvette",
    "brigitte",
    "antoine",
    "amelie",
    "ahmed",
    "omar",
    "fatima",
    "aisha",
    "kwame",
    "chidi",
    "wei",
    "ming",
    "akira",
    "kenji",
    "yuki",
    "sakura",
    "haruto",
    "brad",
    "kyle",
    "brittany",
    "tiffany",
    "crystal",
    "dustin",
    "travis",
    "wyatt",
    "clint",
    "dwight",
    "earl",
    "elmer",
    "floyd",
    "homer",
    "jed",
    "jethro",
    "luther",
    "otis",
    "rufus",
    "wade",
    "abner",
    "hiram",
    "josiah",
    "lemuel",
    "zeke",
    "betsy",
    "bessie",
    "cora",
    "dinah",
    "effie",
    "etta",
    "hattie",
    "ivy",
    "lula",
    "mamie",
    "minnie",
    "nettie",
    "ophelia",
    "pansy",
    "tabitha",
    "agatha",
    "beryl",
    "cynthia",
    "daphne",
    "edwina",
    "esmeralda",
    "fanny",
    "florrie",
    "gwyneth",
    "hilda",
    "ivor",
    "jemima",
    "kitty",
    "letitia",
    "mavis",
    "myrtle",
    "nora",
    "petunia",
    "rosamund",
    "sybil",
    "tamsin",
    "una",
    "wilma",
    "ada",
    "alma",
    "gus",
    "mo",
    "ned",
    "sid",
];

/// Names anyone anywhere knows, of no time and no country: who leaves the
/// frost on the window, who comes at midwinter.
const FOLK: &[&str] = &[
    "jack frost",
    "father christmas",
    "santa",
    "santa claus",
    "saint nicholas",
    "st nicholas",
    "mother nature",
    "lady luck",
    "old man winter",
    "davy jones",
    "neptune",
    "king neptune",
    "the sandman",
    "sandman",
    "tooth fairy",
    "the tooth fairy",
    "old nick",
    "mother goose",
    "humpty dumpty",
    "robin hood",
    "king arthur",
    "merlin",
    "cinderella",
    "snow white",
    "goldilocks",
    "jack and jill",
    "old king cole",
    "little bo peep",
    "jonah",
    "noah's ark",
    "saint peter",
    "st peter",
    "saint brendan",
    "st brendan",
    "saint george",
    "st george",
    "saint patrick",
    "st patrick",
    "saint valentine",
    "st valentine",
    "the man in the moon",
    "man in the moon",
    "jack-o'-lantern",
    "union jack",
    "the union jack",
    "peter pan",
    "tom thumb",
    "john barleycorn",
    "father time",
    "old mother hubbard",
    "jesus",
    "baby jesus",
    "christ",
    "jesus christ",
    "the virgin mary",
    "virgin mary",
    "moses",
    "the three kings",
    "three wise men",
    "big ben",
    "the boogeyman",
    "bogeyman",
    "圣诞老人",
    "海神",
    "龙王",
    "月老",
    "财神",
    "灶王爷",
    "嫦娥",
    "サンタ",
    "サンタクロース",
    "サンタさん",
    "浦島太郎",
    "桃太郎",
    "恵比寿",
    "えびす様",
    "竜宮城",
    "雪女",
    "ジャック・フロスト",
];

/// The real world's places and history a person of a World's time may
/// know of, in every language the World speaks (lower case): the outside-
/// world policy lets real places and events of the era in. A place or event
/// from after the World's time is the time checks' to find.
pub(super) const GAZETTEER: &[&str] = &[
    // Seas, coasts and places around the northern seas.
    "north sea",
    "irish sea",
    "atlantic",
    "atlantic ocean",
    "pacific",
    "pacific ocean",
    "indian ocean",
    "arctic",
    "arctic ocean",
    "southern ocean",
    "mediterranean",
    "baltic",
    "baltic sea",
    "english channel",
    "the channel",
    "bristol channel",
    "bay of biscay",
    "dogger bank",
    "cornwall",
    "devon",
    "dorset",
    "kent",
    "yorkshire",
    "lancashire",
    "norfolk",
    "suffolk",
    "essex",
    "sussex",
    "cumbria",
    "northumberland",
    "the lake district",
    "lake district",
    "highlands",
    "the highlands",
    "hebrides",
    "the hebrides",
    "orkney",
    "shetland",
    "isle of man",
    "isle of wight",
    "scilly",
    "isles of scilly",
    "land's end",
    "lands end",
    "cape wrath",
    "st ives",
    "penzance",
    "falmouth",
    "plymouth",
    "bristol",
    "hull",
    "grimsby",
    "whitby",
    "scarborough",
    "aberdeen",
    "dundee",
    "glasgow",
    "edinburgh",
    "leith",
    "cardiff",
    "swansea",
    "belfast",
    "cork",
    "galway",
    "dublin",
    "liverpool",
    "manchester",
    "newcastle",
    "southampton",
    "portsmouth",
    "dover",
    "brighton",
    "london",
    "york",
    "birmingham",
    "leeds",
    "sheffield",
    "nottingham",
    "oxford",
    "cambridge",
    "bath",
    "norwich",
    "brittany",
    "normandy",
    "biscay",
    "galicia",
    "lisbon",
    "porto",
    "bergen",
    "oslo",
    "stockholm",
    "copenhagen",
    "helsinki",
    "reykjavik",
    "iceland",
    "greenland",
    "faroe",
    "faroes",
    "the faroes",
    "lofoten",
    "norway",
    "sweden",
    "denmark",
    "finland",
    "scotland",
    "england",
    "wales",
    "ireland",
    "britain",
    "great britain",
    "france",
    "spain",
    "portugal",
    "holland",
    "the netherlands",
    "netherlands",
    "belgium",
    "germany",
    "italy",
    "greece",
    "switzerland",
    "austria",
    "poland",
    "russia",
    "europe",
    "the continent",
    "africa",
    "asia",
    "america",
    "the americas",
    "canada",
    "newfoundland",
    "nova scotia",
    "labrador",
    "boston",
    "new york",
    "nantucket",
    "maine",
    "cape cod",
    "australia",
    "new zealand",
    "india",
    "china",
    "japan",
    "egypt",
    "the nile",
    "nile",
    "amsterdam",
    "rotterdam",
    "antwerp",
    "hamburg",
    "bremen",
    "paris",
    "brest",
    "marseille",
    "berlin",
    "rome",
    "venice",
    "naples",
    "madrid",
    "barcelona",
    "vienna",
    "prague",
    "moscow",
    "st petersburg",
    "istanbul",
    "constantinople",
    "cairo",
    "cape town",
    "cape horn",
    "the horn",
    "good hope",
    "cape of good hope",
    "south georgia",
    "the falklands",
    "falklands",
    "antarctica",
    "antarctic",
    "the antarctic",
    "south pole",
    "the south pole",
    "north pole",
    "the north pole",
    "ross sea",
    "weddell sea",
    "tierra del fuego",
    "patagonia",
    "the thames",
    "thames",
    "severn",
    "the severn",
    "clyde",
    "mersey",
    "tyne",
    "humber",
    "shannon",
    "rhine",
    "danube",
    "seine",
    "ben nevis",
    "snowdon",
    "the alps",
    "alps",
    "everest",
    "mount everest",
    "sahara",
    "himalayas",
    // A street of a city or a town, by its real name.
    "wall street",
    "times square",
    "fifth avenue",
    "broadway",
    "downing street",
    "baker street",
    "oxford street",
    "piccadilly",
    "piccadilly circus",
    "trafalgar square",
    // History anyone of these times may know of.
    "the great war",
    "great war",
    "the war",
    "world war",
    "the first world war",
    "the second world war",
    "the depression",
    "great depression",
    "the great depression",
    "the titanic",
    "titanic",
    "the armistice",
    "armistice",
    "trafalgar",
    "waterloo",
    "great fire of london",
    "the great fire of london",
    "the berlin wall",
    "berlin wall",
    "the wall",
    "the cold war",
    "cold war",
    "the moon landing",
    "moon landing",
    "the olympics",
    "olympics",
    "the world cup",
    "world cup",
    "chernobyl",
    "live aid",
    "the space shuttle",
    "challenger",
    "halley's comet",
    "the iron curtain",
    "iron curtain",
    "east germany",
    "west germany",
    "the soviet union",
    "soviet union",
    "the ussr",
    "ussr",
    // 中文
    "北海",
    "大西洋",
    "太平洋",
    "印度洋",
    "北冰洋",
    "地中海",
    "波罗的海",
    "英吉利海峡",
    "康沃尔",
    "德文郡",
    "约克郡",
    "苏格兰",
    "英格兰",
    "威尔士",
    "爱尔兰",
    "英国",
    "法国",
    "西班牙",
    "葡萄牙",
    "荷兰",
    "比利时",
    "德国",
    "意大利",
    "希腊",
    "瑞士",
    "奥地利",
    "波兰",
    "挪威",
    "瑞典",
    "丹麦",
    "芬兰",
    "冰岛",
    "格陵兰",
    "俄国",
    "俄罗斯",
    "欧洲",
    "非洲",
    "亚洲",
    "美洲",
    "美国",
    "加拿大",
    "纽芬兰",
    "澳大利亚",
    "新西兰",
    "印度",
    "中国",
    "日本",
    "埃及",
    "尼罗河",
    "布列塔尼",
    "诺曼底",
    "里斯本",
    "卑尔根",
    "奥斯陆",
    "斯德哥尔摩",
    "哥本哈根",
    "雷克雅未克",
    "赫尔",
    "格里姆斯比",
    "普利茅斯",
    "布里斯托尔",
    "利物浦",
    "曼彻斯特",
    "伦敦",
    "爱丁堡",
    "格拉斯哥",
    "阿伯丁",
    "都柏林",
    "贝尔法斯特",
    "戈尔韦",
    "巴黎",
    "柏林",
    "罗马",
    "威尼斯",
    "马德里",
    "维也纳",
    "莫斯科",
    "阿姆斯特丹",
    "汉堡",
    "纽约",
    "波士顿",
    "开普敦",
    "合恩角",
    "好望角",
    "南乔治亚",
    "福克兰",
    "南极",
    "南极洲",
    "北极",
    "南极点",
    "北极点",
    "泰晤士河",
    "莱茵河",
    "多瑙河",
    "阿尔卑斯山",
    "珠穆朗玛峰",
    "撒哈拉",
    "喜马拉雅",
    "一战",
    "第一次世界大战",
    "大萧条",
    "泰坦尼克号",
    "滑铁卢",
    "柏林墙",
    "冷战",
    "登月",
    "奥运会",
    "世界杯",
    "切尔诺贝利",
    "苏联",
    "东德",
    "西德",
    "铁幕",
    "哈雷彗星",
    // 日本語
    "北海",
    "大西洋",
    "太平洋",
    "インド洋",
    "北極海",
    "地中海",
    "バルト海",
    "イギリス海峡",
    "コーンウォール",
    "デボン",
    "ヨークシャー",
    "スコットランド",
    "イングランド",
    "ウェールズ",
    "アイルランド",
    "イギリス",
    "フランス",
    "スペイン",
    "ポルトガル",
    "オランダ",
    "ベルギー",
    "ドイツ",
    "イタリア",
    "ギリシャ",
    "スイス",
    "オーストリア",
    "ポーランド",
    "ノルウェー",
    "スウェーデン",
    "デンマーク",
    "フィンランド",
    "アイスランド",
    "グリーンランド",
    "ロシア",
    "ヨーロッパ",
    "アフリカ",
    "アジア",
    "アメリカ",
    "カナダ",
    "ニューファンドランド",
    "オーストラリア",
    "ニュージーランド",
    "インド",
    "中国",
    "日本",
    "エジプト",
    "ナイル",
    "ブルターニュ",
    "ノルマンディー",
    "リスボン",
    "ベルゲン",
    "オスロ",
    "ストックホルム",
    "コペンハーゲン",
    "レイキャビク",
    "ハル",
    "プリマス",
    "ブリストル",
    "リバプール",
    "マンチェスター",
    "ロンドン",
    "エディンバラ",
    "グラスゴー",
    "アバディーン",
    "ダブリン",
    "ベルファスト",
    "ゴールウェイ",
    "パリ",
    "ベルリン",
    "ローマ",
    "ベネチア",
    "ヴェネツィア",
    "マドリード",
    "ウィーン",
    "モスクワ",
    "アムステルダム",
    "ハンブルク",
    "ニューヨーク",
    "ボストン",
    "ケープタウン",
    "ホーン岬",
    "喜望峰",
    "南ジョージア",
    "フォークランド",
    "南極",
    "南極大陸",
    "北極",
    "南極点",
    "北極点",
    "テムズ川",
    "ライン川",
    "ドナウ川",
    "アルプス",
    "エベレスト",
    "サハラ",
    "ヒマラヤ",
    "第一次世界大戦",
    "大恐慌",
    "タイタニック号",
    "ワーテルロー",
    "ベルリンの壁",
    "冷戦",
    "月面着陸",
    "オリンピック",
    "ワールドカップ",
    "チェルノブイリ",
    "ソ連",
    "東ドイツ",
    "西ドイツ",
    "鉄のカーテン",
    "ハレー彗星",
];

/// Words in Latin letters that Chinese and Japanese speech takes in as
/// they are and that name nobody: greetings, a word of Spanish from a
/// night class, a few letters for a thing.
const EVERYDAY_LATIN: &[&str] = &[
    "ok",
    "okay",
    "bye",
    "hi",
    "hello",
    "yes",
    "no",
    "sorry",
    "thanks",
    "thank",
    "wow",
    "hey",
    "oh",
    "sir",
    "mister",
    "miss",
    "mr",
    "mrs",
    "ms",
    "dj",
    "tv",
    "cd",
    "lp",
    "ufo",
    "sos",
    "bbq",
    "ng",
    "diy",
    "vip",
    "pk",
    "kg",
    "km",
    "cm",
    "mm",
    "am",
    "pm",
    "adios",
    "adiós",
    "gracias",
    "hola",
    "amigo",
    "amiga",
    "señor",
    "señora",
    "senor",
    "senora",
    "bonjour",
    "merci",
    "oui",
    "non",
    "salut",
    "voila",
    "voilà",
    "ciao",
    "grazie",
    "prego",
    "bravo",
    "danke",
    "bitte",
    "nein",
    "ja",
    "guten",
    "tag",
    "olé",
    "ole",
    "sayonara",
    "aloha",
    "bingo",
    "encore",
    "cheers",
    "hooray",
    "yay",
    "boss",
    "baby",
    "honey",
    "darling",
    "party",
    "happy",
    "birthday",
    "merry",
    "christmas",
    "new",
    "year",
];

/// The word after a name that makes it a local place.
const LOCAL_HEADS: &[&str] = &[
    "inn",
    "tavern",
    "pub",
    "arms",
    "alehouse",
    "cafe",
    "café",
    "diner",
    "tearoom",
    "tearooms",
    "bakery",
    "shop",
    "store",
    "stores",
    "emporium",
    "mill",
    "farm",
    "farmstead",
    "hall",
    "cottage",
    "manor",
    "works",
    "cannery",
    "brewery",
    "distillery",
    "smokehouse",
    "yard",
    "boatyard",
    "forge",
    "smithy",
    "chapel",
    "church",
    "school",
    "academy",
    "cave",
    "caves",
    "cavern",
    "caverns",
    "grotto",
    "cove",
    "beach",
    "strand",
    "rock",
    "rocks",
    "reef",
    "skerry",
    "mine",
    "quarry",
    "orchard",
    "warehouse",
    "hotel",
    "motel",
    "lodge",
    "saloon",
    "parlour",
    "parlor",
    "salon",
    "arcade",
    "rink",
    "lanes",
    "cinema",
    "theatre",
    "theater",
    "outpost",
    "station",
    "depot",
    "dome",
    "module",
    "hab",
    "colony",
    "rookery",
    "burrow",
    "den",
    "hollow",
    "landing",
    "wharf",
    "jetty",
    "pier",
];

/// The words that make a name a piece of history: a disaster, a war.
const HISTORY_HEADS: &[&str] = &[
    "fire",
    "flood",
    "floods",
    "storm",
    "gale",
    "blizzard",
    "famine",
    "plague",
    "wreck",
    "drought",
    "freeze",
    "frost",
    "war",
    "siege",
    "riot",
    "riots",
    "strike",
    "battle",
    "massacre",
    "uprising",
    "rebellion",
    "disaster",
    "tragedy",
    "year",
    "winter",
    "summer",
    "crash",
    "collapse",
    "eruption",
    "quake",
    "earthquake",
];

/// Chinese surnames, one or two characters: a name begun with one and a
/// given name after it is a person.
const HAN_SURNAMES: &[&str] = &[
    "王",
    "李",
    "张",
    "刘",
    "陈",
    "杨",
    "黄",
    "赵",
    "吴",
    "周",
    "徐",
    "孙",
    "马",
    "朱",
    "胡",
    "郭",
    "何",
    "高",
    "林",
    "罗",
    "郑",
    "梁",
    "谢",
    "宋",
    "唐",
    "许",
    "韩",
    "冯",
    "邓",
    "曹",
    "彭",
    "曾",
    "肖",
    "田",
    "董",
    "袁",
    "潘",
    "于",
    "蒋",
    "蔡",
    "余",
    "杜",
    "叶",
    "程",
    "苏",
    "魏",
    "吕",
    "丁",
    "任",
    "沈",
    "姚",
    "卢",
    "姜",
    "崔",
    "钟",
    "谭",
    "陆",
    "汪",
    "范",
    "金",
    "石",
    "廖",
    "贾",
    "夏",
    "韦",
    "付",
    "方",
    "白",
    "邹",
    "孟",
    "熊",
    "秦",
    "邱",
    "江",
    "尹",
    "薛",
    "闫",
    "段",
    "雷",
    "侯",
    "龙",
    "史",
    "陶",
    "黎",
    "贺",
    "顾",
    "毛",
    "郝",
    "龚",
    "邵",
    "万",
    "钱",
    "严",
    "覃",
    "武",
    "戴",
    "莫",
    "孔",
    "向",
    "汤",
    "欧阳",
    "司马",
    "上官",
    "诸葛",
    "佐藤",
    "鈴木",
    "高橋",
    "田中",
    "伊藤",
    "渡辺",
    "山本",
    "中村",
    "小林",
    "加藤",
    "吉田",
    "山田",
    "佐々木",
    "山口",
    "松本",
    "井上",
    "木村",
    "斎藤",
    "清水",
    "山崎",
    "池田",
    "橋本",
    "阿部",
    "石川",
    "山下",
    "中島",
    "石井",
    "小川",
    "前田",
    "岡田",
    "長谷川",
    "藤田",
    "後藤",
    "近藤",
    "村上",
    "遠藤",
    "青木",
    "坂本",
    "福田",
    "太田",
    "西村",
    "藤井",
    "金子",
    "岡本",
    "藤原",
    "中野",
    "三浦",
    "原田",
    "中川",
    "松田",
    "竹内",
    "小野",
    "田村",
    "中山",
    "和田",
];

/// Who someone is, after a Chinese or Japanese name: a title, an
/// honorific, kin.
const HAN_PERSON_AFTER: &[&str] = &[
    "先生",
    "太太",
    "小姐",
    "女士",
    "夫人",
    "大爷",
    "大妈",
    "大叔",
    "大婶",
    "阿姨",
    "师傅",
    "老板娘",
    "老板",
    "掌柜",
    "镇长",
    "村长",
    "市长",
    "校长",
    "警长",
    "警官",
    "船长",
    "医生",
    "大夫",
    "牧师",
    "神父",
    "修女",
    "爷爷",
    "奶奶",
    "姥姥",
    "婆婆",
    "叔叔",
    "伯伯",
    "舅舅",
    "姑姑",
    "长老",
    "议员",
    "局长",
    "主任",
    "经理",
    "队长",
    "老爷",
    "老师",
    "教授",
    "博士",
    "院长",
    "司令",
    "将军",
    "巡査",
    "牧師",
    "長老",
    "船長",
    "医師",
    "神父",
    "博士",
    "先輩",
    "親方",
    "隊長",
    "村長",
    "町長",
    "署長",
    "校長",
    "師匠",
    "院長",
    "店長",
    "教授",
    "夫人",
    "老人",
    "翁",
    "婆",
];

/// Honorifics written in kana after a name.
const KANA_PERSON_AFTER: &[&str] = &[
    "じいさん",
    "ばあさん",
    "おじいさん",
    "おばあさん",
    "さん",
    "ちゃん",
    "くん",
    "君",
    "さま",
    "様",
    "氏",
    "どの",
    "殿",
    "爺さん",
    "婆さん",
    "おじさん",
    "おばさん",
    "じい",
    "ばあ",
];

/// What a Japanese name is by the word after it: an inn, an eating house,
/// a ship, a farm. Not 屋 or 店, which after a common word only say what a
/// shop sells (パン屋, チーズ屋).
const JA_PLACE_AFTER: &[&str] = &[
    "亭", "食堂", "商店", "酒場", "号", "丸", "農場", "牧場", "工場", "洞窟", "洞穴", "浜", "岬",
    "湾", "通り",
];

/// Chinese for a local place, after its name.
const HAN_PLACE_AFTER: &[&str] = &[
    "罐头厂",
    "加工厂",
    "造船厂",
    "工厂",
    "厂",
    "酒馆",
    "酒吧",
    "饭店",
    "餐馆",
    "食堂",
    "客栈",
    "旅馆",
    "旅店",
    "商店",
    "杂货铺",
    "铺",
    "店",
    "馆",
    "坊",
    "磨坊",
    "农场",
    "牧场",
    "渔场",
    "滩",
    "礁",
    "洞",
    "岬",
    "灯塔",
    "号",
];

/// What real geography ends with: a Chinese or Japanese name ending so is
/// a river or a mountain before it is anyone.
const GEOGRAPHY_TAILS: &str = "河江山湖海洋岛州县市镇村城港湾峰岭原谷洲省国宫寺庙塔门川島県";

/// Words Chinese writes before a family name, as people call each other.
const HAN_PERSON_BEFORE: &[&str] = &["老", "小", "阿"];

/// Shops named by what they sell, as anyone says them: not a name.
const GENERIC_HAN: &str = "面包糕点鱼肉酒茶布鞋药书花杂货五金修理理发洗衣早餐小吃咖啡冰淇淋水果蔬菜粮油铁匠木匠裁缝渔具船具海产干货烟草糖果玩具文具照相钟表眼镜百货日用渔面馆大小老新旧这那一家个间街角镇上岛港村东西南北头边门前后的";

/// Japanese for whoever someone is by their work or kin, before an
/// honorific: パン屋さん, お医者さん, おじいさん.
const JA_COMMON_BEFORE_SAN: &[&str] = &[
    "屋",
    "医者",
    "大工",
    "漁師",
    "先生",
    "巡り",
    "父",
    "母",
    "兄",
    "姉",
    "坊",
    "嬢",
    "奥",
    "皆",
    "店員",
    "船長",
    "隣",
    "お客",
    "客",
    "職人",
    "主人",
    "女将",
    "親方",
    "神",
    "仏",
    "お日",
    "お月",
    "お天道",
];

fn lower(name: &str) -> String {
    name.trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || "「」『』“”‘’。、・".contains(c))
        .to_lowercase()
}

fn without_the(name: &str) -> &str {
    name.strip_prefix("the ").unwrap_or(name)
}

/// Whether someone with these grounds might know the name though the World
/// never gave it: a folk figure, or the real world's geography or history.
pub(super) fn known_to(grounds: &Grounds, name: &str) -> bool {
    // A World between worlds names its planets in its own era lexicon
    // (`Hearing::era_has`), which the grounds already know.
    let _ = grounds;
    known_anywhere(name)
}

/// Whether anyone of the World's time might know the name: a folk figure,
/// or the real world's geography or history.
pub(super) fn known_anywhere(name: &str) -> bool {
    let name = lower(name);
    let bare = without_the(&name);
    FOLK.contains(&name.as_str())
        || FOLK.contains(&bare)
        || GAZETTEER.contains(&name.as_str())
        || GAZETTEER.contains(&bare)
}

fn is_katakana(c: char) -> bool {
    matches!(c as u32, 0x30A1..=0x30FA)
}

fn has_letters(name: &str) -> bool {
    name.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 2
}

/// Whether `text` has two or more Chinese characters or kana: it is written
/// in Chinese or Japanese.
fn cjk(text: &str) -> bool {
    text.chars()
        .filter(|c| is_han(*c) || matches!(*c as u32, 0x3040..=0x30FF))
        .count()
        >= 2
}

/// What follows `name` where `answer` says it, folded.
fn after_in<'a>(answer: &'a str, name: &str) -> Option<&'a str> {
    answer.find(name).map(|at| &answer[at + name.len()..])
}

/// What comes before `name` where `answer` says it.
fn before_in<'a>(answer: &'a str, name: &str) -> Option<&'a str> {
    answer.find(name).map(|at| &answer[..at])
}

/// Whether a name, as a judge lists it from `answer`, is one nobody gave
/// this person and its shape says it is someone, somewhere or something
/// that happened here: what it is, if so.
pub fn invented(name: &str, answer: &str, grounds: &Grounds) -> Option<Invented> {
    let name = name.trim();
    if name.is_empty() || grounds.knows_name(name) || known_to(grounds, name) {
        return None;
    }
    let folded = Text::read(answer).norm;
    let lowered = lower(name);
    if name.chars().any(is_katakana) {
        return katakana(name, &folded, grounds);
    }
    if name.chars().any(is_han) {
        return han(name, &folded, grounds);
    }
    if !has_letters(name) {
        return None;
    }
    // A name in Latin letters inside Chinese or Japanese is someone the
    // World would have written its own way, or never heard of.
    if cjk(answer)
        && name.chars().next().is_some_and(char::is_uppercase)
        && name.len() >= 3
        && !lowered
            .split_whitespace()
            .all(|word| EVERYDAY_LATIN.contains(&word))
    {
        return Some(Invented::Person);
    }
    latin(&lowered, &folded, grounds)
}

/// A spaced name: a title before a name, a given name, a local place by
/// its head word, a piece of history.
fn latin(name: &str, answer: &str, grounds: &Grounds) -> Option<Invented> {
    let words = name
        .split(|c: char| c.is_whitespace() || c == '.')
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric() && c != '\''))
        .map(super::text::base)
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let little = [
        "the", "of", "a", "an", "and", "de", "von", "van", "la", "le", "del",
    ];
    let names = words
        .iter()
        .filter(|word| !little.contains(word))
        .copied()
        .collect::<Vec<_>>();
    if names.is_empty() {
        return None;
    }
    let last = *names.last()?;
    // Brighton Pier, the Great Fire of London: real places, by the real
    // place in them.
    if names.iter().any(|word| GAZETTEER.contains(word)) {
        return None;
    }
    if names.len() >= 2 && HISTORY_HEADS.iter().any(|head| names.contains(head)) {
        let rest = names.iter().filter(|word| !HISTORY_HEADS.contains(word));
        if rest.clone().any(|word| !grounds.knows_word(word)) {
            return Some(Invented::History);
        }
    }
    if names.len() >= 2 && LOCAL_HEADS.contains(&last) {
        let rest = &names[..names.len() - 1];
        if rest.iter().any(|word| !grounds.knows_word(word)) {
            return Some(Invented::Place);
        }
    }
    let titled = names.len() >= 2
        && PERSON_TITLES.contains(&names[0])
        && names[1..].iter().any(|word| !grounds.knows_word(word));
    // "Captain Harvey" as the judge lists only "Harvey".
    let titled_in_answer = names.len() == 1
        && before_in(answer, names[0]).is_some_and(|before| {
            before
                .split_whitespace()
                .next_back()
                .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
                .is_some_and(|word| PERSON_TITLES.contains(&word))
        });
    let given =
        GIVEN_NAMES.contains(&names[0]) && names.iter().any(|word| !grounds.knows_word(word));
    (titled || titled_in_answer || given).then_some(Invented::Person)
}

/// A katakana name: an honorific or a title after it, a shop or a ship's
/// word after it, or a full name with its middle dot.
fn katakana(name: &str, answer: &str, grounds: &Grounds) -> Option<Invented> {
    let core = name
        .chars()
        .skip_while(|c| !is_katakana(*c))
        .take_while(|c| is_katakana(*c) || *c == 'ー' || *c == '・')
        .collect::<String>();
    if core.chars().count() < 2 || super::japanese::knows_katakana(grounds, &core) {
        return None;
    }
    let tail = name[name.find(&core).map_or(0, |at| at + core.len())..].to_string();
    let after = format!("{tail}{}", after_in(answer, &core).unwrap_or_default());
    let head = &name[..name.find(&core).unwrap_or(0)];
    let before = format!("{}{head}", before_in(answer, &core).unwrap_or_default());
    let person = KANA_PERSON_AFTER
        .iter()
        .chain(HAN_PERSON_AFTER)
        .any(|word| after.starts_with(word))
        || ["ミスター", "ミセス", "ミス", "ドクター", "キャプテン", "老"]
            .iter()
            .any(|word| before.ends_with(word))
        || (core.contains('・') && core.chars().count() >= 5);
    if person {
        return Some(Invented::Person);
    }
    let place =
        core.chars().count() >= 3 && JA_PLACE_AFTER.iter().any(|word| after.starts_with(word));
    place.then_some(Invented::Place)
}

/// A name in Chinese characters (or Japanese kanji): a title after it, a
/// family name and a given name, a local place by its last word.
fn han(name: &str, answer: &str, grounds: &Grounds) -> Option<Invented> {
    let name = lower(name);
    // An honorific in kana after kanji: 佐藤さん, 源じいさん.
    for honorific in KANA_PERSON_AFTER {
        if let Some(core) = name.strip_suffix(honorific) {
            let generic = JA_COMMON_BEFORE_SAN
                .iter()
                .any(|word| core.ends_with(word) || core.starts_with('お'));
            return (core.chars().filter(|c| is_han(*c)).count() >= 1
                && !generic
                && !grounds.knows_han(core))
            .then_some(Invented::Person);
        }
    }
    let unknown = |part: &str| !part.is_empty() && !grounds.knows_han(part);
    for title in HAN_PERSON_AFTER {
        if let Some(core) = name.strip_suffix(title) {
            if core.chars().count() >= 1
                && (core.chars().count() == 1 || unknown(core))
                && !core.chars().all(|c| GENERIC_HAN.contains(c))
            {
                return Some(Invented::Person);
            }
            return None;
        }
    }
    let chars = name.chars().count();
    if name.ends_with(|c| GEOGRAPHY_TAILS.contains(c)) {
        return None;
    }
    for tail in HAN_PLACE_AFTER {
        if let Some(core) = name.strip_suffix(tail) {
            let generic = core.chars().all(|c| GENERIC_HAN.contains(c));
            return (core.chars().count() >= 2 && !generic && unknown(core))
                .then_some(Invented::Place);
        }
    }
    let surname = HAN_SURNAMES
        .iter()
        .find(|surname| name.starts_with(*surname));
    if let Some(surname) = surname {
        let given = surname.chars().count();
        if (given + 1..=given + 2).contains(&chars) && unknown(&name) {
            return Some(Invented::Person);
        }
    }
    // 老王, 小李: what neighbours call each other.
    if chars == 2
        && HAN_PERSON_BEFORE.iter().any(|word| name.starts_with(word))
        && HAN_SURNAMES.iter().any(|surname| name.ends_with(surname))
        && unknown(&name)
        && !answer.is_empty()
    {
        return Some(Invented::Person);
    }
    None
}

/// Whether an answer, by itself, names someone or somewhere invented in a
/// shape that leaves no doubt what it is: a title and a name, a given name
/// mid-sentence, a name in Latin letters inside Chinese or Japanese, a
/// katakana name with an honorific or a shop's word after it. What no judge
/// is needed to read; the judge's own list of names reads the rest.
pub(super) fn invented_in(raw: &str, text: &Text, grounds: &Grounds) -> bool {
    let words = &text.words;
    let in_cjk = cjk(raw);
    let spaced = words.iter().enumerate().any(|(index, word)| {
        if !word.capital
            || word.text.chars().count() < 2
            || !word
                .text
                .chars()
                .all(|c| c.is_alphabetic() || c == '-' || c == '\'')
            || grounds.knows_word(&word.text)
            || super::lexicon::PRONOUNS.contains(&word.text.as_str())
        {
            return false;
        }
        let same = |at: usize| {
            words
                .get(at)
                .filter(|other| other.sentence == word.sentence)
        };
        let before = index.checked_sub(1).and_then(same);
        let after = same(index + 1);
        let pair = |a: &str, b: &str| format!("{a} {b}");
        // Jack Frost, Mother Nature, Land's End, Cape Town.
        if after
            .is_some_and(|next| next.capital && known_to(grounds, &pair(&word.text, &next.text)))
            || before.is_some_and(|prior| {
                prior.capital && known_to(grounds, &pair(&prior.text, &word.text))
            })
            || known_to(grounds, &word.text)
        {
            return false;
        }
        if in_cjk {
            return word.text.chars().count() >= 3 && !EVERYDAY_LATIN.contains(&word.text.as_str());
        }
        let titled = before.is_some_and(|prior| {
            prior.capital && PERSON_TITLES.contains(&prior.text.as_str()) && !prior.pause
        });
        let given = GIVEN_NAMES.contains(&word.text.as_str()) && !word.first;
        let full = GIVEN_NAMES.contains(&word.text.as_str())
            && after
                .is_some_and(|next| next.capital && !word.pause && !grounds.knows_word(&next.text));
        titled || given || full
    });
    if spaced {
        return true;
    }
    // A run of capitalised words that is a local place or a piece of
    // local history by its shape: the Gullstone Caves, the Great Gale of
    // the Mackerel Year.
    if !in_cjk {
        let mut run: Vec<&str> = Vec::new();
        let mut sentence = usize::MAX;
        let mut runs = Vec::new();
        for word in words
            .iter()
            .chain(std::iter::once(&super::text::Word::default()))
        {
            let joins = !run.is_empty()
                && word.sentence == sentence
                && ["of", "the"].contains(&word.text.as_str());
            if word.capital && !word.first && (run.is_empty() || word.sentence == sentence) {
                run.push(word.text.as_str());
                sentence = word.sentence;
            } else if joins {
                run.push(word.text.as_str());
            } else {
                if run.len() >= 2 {
                    runs.push(run.join(" "));
                }
                run.clear();
                if word.capital && !word.first {
                    run.push(word.text.as_str());
                    sentence = word.sentence;
                }
            }
        }
        let shaped = runs.iter().any(|name| {
            let name = name.trim_end_matches(" of").trim_end_matches(" the");
            !grounds.knows_name(name)
                && !known_to(grounds, name)
                && matches!(
                    latin(name, &text.norm, grounds),
                    Some(Invented::Place | Invented::History)
                )
        });
        if shaped {
            return true;
        }
    }
    // A Chinese family name with a title after it: 邓警长, 张医生, 王老板;
    // a Japanese one with an honorific: 田中さん.
    // Only honorifics that are never the end of another word ("高さまで"
    // is "as high as", not "Lord Gao").
    let titled_han = HAN_PERSON_AFTER
        .iter()
        .chain(&["さん", "ちゃん", "くん"])
        .any(|title| {
            text.find_han(title).into_iter().any(|at| {
                let before = text.norm[..at]
                    .chars()
                    .rev()
                    .take_while(|c| is_han(*c))
                    .take(4)
                    .collect::<Vec<_>>();
                let before = before.into_iter().rev().collect::<String>();
                HAN_SURNAMES.iter().any(|surname| {
                    before.ends_with(surname) && {
                        let name = format!("{surname}{title}");
                        // Only the family name itself, begun where a name
                        // may begin.
                        let start = before.len() - surname.len();
                        let prior = before[..start].chars().next_back();
                        prior.is_none_or(|c| {
                            "的在是了和与跟给对把被让叫说到从去来请问找见位".contains(c)
                        }) && !grounds.knows_name(&name)
                    }
                })
            })
        });
    if titled_han {
        return true;
    }
    if text.kana == 0 {
        return false;
    }
    super::japanese::katakana_runs(&text.norm)
        .into_iter()
        .any(|(start, end)| {
            let core = &text.norm[start..end];
            if core.chars().count() < 2
                || super::japanese::knows_katakana(grounds, core)
                || known_to(grounds, core)
            {
                return false;
            }
            let after = &text.norm[end..];
            KANA_PERSON_AFTER
                .iter()
                .chain(HAN_PERSON_AFTER)
                .any(|word| after.starts_with(word))
                || (core.chars().count() >= 3
                    && JA_PLACE_AFTER.iter().any(|word| after.starts_with(word)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Era;

    fn grounds(said: &str) -> Grounds {
        Grounds::new(
            [
                "Leo",
                "Jonas",
                "Mara",
                "Anchor Pub",
                "锚酒馆",
                "酒場「アンカー」",
                "ジョナス",
                "乔纳斯",
                "Old Tam",
                "老Tam",
                "タムじいさん",
                "Sea Finch",
                "海雀号",
                "シーフィンチ号",
                "Rosa",
                "ローザ",
                "罗莎",
            ],
            said,
            Era::Radio,
        )
    }

    #[test]
    fn invented_people_are_read_by_their_shape_in_every_script() {
        let en = grounds("Who's new around here?");
        for (name, answer) in [
            ("Captain Harvey", "Captain Harvey runs the ferry now."),
            ("Harvey", "Captain Harvey runs the ferry now."),
            (
                "Bartholomew Quill",
                "Bartholomew Quill mends the nets these days.",
            ),
            ("Widow Pennock", "The Widow Pennock keeps hens up the lane."),
            ("Agatha", "Agatha from up the hill brought jam."),
        ] {
            assert_eq!(
                invented(name, answer, &en),
                Some(Invented::Person),
                "{name}"
            );
        }
        assert_eq!(
            invented(
                "Gullstone Caves",
                "We used to hide in the Gullstone Caves.",
                &en
            ),
            Some(Invented::Place)
        );
        assert_eq!(
            invented(
                "Great Gale of the Mackerel Year",
                "Since the Great Gale of the Mackerel Year.",
                &en
            ),
            Some(Invented::History)
        );
        let zh = grounds("最近有什么新鲜事？");
        assert_eq!(
            invented("Bertram", "Bertram昨天来过。", &zh),
            Some(Invented::Person)
        );
        assert_eq!(
            invented("周大福", "周大福说要来。", &zh),
            Some(Invented::Person)
        );
        assert_eq!(
            invented("海豹罐头厂", "海豹罐头厂招人呢。", &zh),
            Some(Invented::Place)
        );
        assert_eq!(
            invented("胡警长", "胡警长来过了。", &zh),
            Some(Invented::Person)
        );
        let ja = grounds("最近どう？");
        assert_eq!(
            invented("ブランドン巡査", "ブランドン巡査が来たよ。", &ja),
            Some(Invented::Person)
        );
        assert_eq!(
            invented("ブランドン", "ブランドンさんが来たよ。", &ja),
            Some(Invented::Person)
        );
        assert_eq!(
            invented("カモメ亭", "カモメ亭で会おう。", &ja),
            Some(Invented::Place)
        );
        assert_eq!(
            invented("佐藤さん", "佐藤さんに聞いて。", &ja),
            Some(Invented::Person)
        );
    }

    #[test]
    fn the_worlds_own_names_and_the_real_worlds_places_are_never_invented() {
        let en = grounds("Where have you been?");
        for (name, answer) in [
            ("Leo", "Leo's at the Anchor Pub."),
            ("Anchor Pub", "Leo's at the Anchor Pub."),
            ("Old Tam", "Old Tam was in."),
            ("Rosa", "Rosa might come by."),
            ("Cornwall", "My cousin went to Cornwall."),
            ("Hull", "The trawler came up from Hull."),
            ("Jack Frost", "Jack Frost's been at the windows."),
            ("North Sea", "The North Sea was wild."),
            ("Berlin Wall", "They say the Berlin Wall came down."),
            ("Livin' on a Prayer", "They played Livin' on a Prayer."),
        ] {
            assert_eq!(invented(name, answer, &en), None, "{name}");
        }
        let zh = grounds("你去哪儿了？");
        for (name, answer) in [
            ("锚酒馆", "我在锚酒馆。"),
            ("老Tam", "老Tam来过。"),
            ("海雀号", "海雀号回来了。"),
            ("康沃尔", "我表哥去了康沃尔。"),
            ("黄河", "黄河很远。"),
            ("面包店", "面包店开着。"),
            ("Rosa", "Rosa也许会来。"),
        ] {
            assert_eq!(invented(name, answer, &zh), None, "{name}");
        }
        let ja = grounds("どこ行ってたの？");
        for (name, answer) in [
            ("アンカー", "アンカーにいたよ。"),
            ("ジョナスさん", "ジョナスさんに聞いて。"),
            ("タムじいさん", "タムじいさんが来た。"),
            ("シーフィンチ号", "シーフィンチ号が戻った。"),
            ("ローザ", "ローザが来るかも。"),
            ("コーンウォール", "コーンウォールに行ったんだ。"),
            ("パン屋さん", "パン屋さんに行こう。"),
            ("サンタさん", "サンタさんが来るよ。"),
        ] {
            assert_eq!(invented(name, answer, &ja), None, "{name}");
        }
    }

    #[test]
    fn an_answer_alone_shows_an_invented_name_when_its_shape_leaves_no_doubt() {
        let found = |answer: &str, said: &str| {
            let grounds = grounds(said);
            invented_in(answer, &Text::read(answer), &grounds)
        };
        assert!(found("I heard it from Nurse Winterbottom.", "Any news?"));
        assert!(found("Ask Agatha, she knows.", "Any news?"));
        assert!(found("Bertram昨天来过。", "有什么新闻？"));
        assert!(found("ブランドンさんが来たよ。", "ニュースある？"));
        assert!(found("昨日、カモメ亭で会ったよ。", "ニュースある？"));
        assert!(!found("Leo's at the Anchor Pub.", "Any news?"));
        assert!(!found("Jack Frost's been at the windows.", "Cold?"));
        assert!(!found("Rosa might come by.", "Any news?"));
        assert!(!found("We went to Cornwall in May.", "Any news?"));
        assert!(!found("Agatha? Never heard of her.", "Who's Agatha?"));
        assert!(!found("ジョナスさんが来たよ。", "ニュースある？"));
        assert!(!found("パン屋さんに行こう。", "どこ行く？"));
        assert!(!found("我在锚酒馆。", "你在哪？"));
    }
}
