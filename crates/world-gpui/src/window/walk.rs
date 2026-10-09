//! A walk through the World window in every language the app speaks
//! besides English: every word it lays out, in every state a player
//! reaches (the card, the drawer's leaves, the build card from the hands
//! and from a plot, the person card, a legend, a moment, the almanac),
//! recorded as the text system shapes it. The World here speaks only that
//! language, apart from people's names; so any English word left on
//! screen is the app's own, untranslated.
//!
//! One walk for all of them (v0.27; it was a Chinese and a Japanese walk,
//! nine tenths the same): a language is a [`Words`] table, and every check
//! runs for every language. A new language is a new table.

use super::*;
use gpui::{
    Font, FontId, FontMetrics, FontRun, GlyphId, LineLayout, NoopTextSystem, Pixels,
    PlatformTextSystem, RenderGlyphParams, TestAppContext, TestDispatcher, TextRenderingMode,
    VisualTestContext,
};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use world_projection::{Hand, LegendLine, Question, StoryPage, StoryRequest};

/// A text system that shapes like the test platform's own and keeps every
/// line it is asked to lay out.
struct Recording {
    inner: NoopTextSystem,
    lines: Arc<Mutex<Vec<String>>>,
}

impl PlatformTextSystem for Recording {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> gpui::Result<()> {
        self.inner.add_fonts(fonts)
    }
    fn all_font_names(&self) -> Vec<String> {
        self.inner.all_font_names()
    }
    fn font_id(&self, descriptor: &Font) -> gpui::Result<FontId> {
        self.inner.font_id(descriptor)
    }
    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        self.inner.font_metrics(font_id)
    }
    fn typographic_bounds(
        &self,
        font_id: FontId,
        glyph_id: GlyphId,
    ) -> gpui::Result<gpui::Bounds<f32>> {
        self.inner.typographic_bounds(font_id, glyph_id)
    }
    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> gpui::Result<gpui::Size<f32>> {
        self.inner.advance(font_id, glyph_id)
    }
    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.inner.glyph_for_char(font_id, ch)
    }
    fn glyph_raster_bounds(
        &self,
        params: &RenderGlyphParams,
    ) -> gpui::Result<gpui::Bounds<gpui::DevicePixels>> {
        self.inner.glyph_raster_bounds(params)
    }
    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        raster_bounds: gpui::Bounds<gpui::DevicePixels>,
    ) -> gpui::Result<(gpui::Size<gpui::DevicePixels>, Vec<u8>)> {
        self.inner.rasterize_glyph(params, raster_bounds)
    }
    fn layout_line(&self, text: &str, font_size: Pixels, runs: &[FontRun]) -> LineLayout {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push(text.to_string());
        }
        self.inner.layout_line(text, font_size, runs)
    }
    fn recommended_rendering_mode(&self, font_id: FontId, font_size: Pixels) -> TextRenderingMode {
        self.inner.recommended_rendering_mode(font_id, font_size)
    }
}

/// Words that read the same in any language: the app's name, the Mac, a key.
const ALLOWED: &[&str] = &[
    "World", "Machine", "Mac", "API", "Esc", "OK", "Ctrl", "Alt", "Shift",
];

/// The English words in `line`, but for `names` and what reads the same
/// in any language.
fn english_in(line: &str, names: &[String]) -> Vec<String> {
    line.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| word.len() >= 2)
        .filter(|word| !ALLOWED.contains(word))
        .filter(|word| !names.iter().any(|name| name == word))
        .map(str::to_string)
        .collect()
}

/// What the World says in one language, for the walk.
struct Words {
    language: world_i18n::Language,
    /// The app's own catalog for it.
    catalog: &'static str,
    /// What the language is called in a failure.
    called: &'static str,
    /// Where `WORLD_MACHINE_<..>_WALK=<file>` keeps what was read.
    keep_in: &'static str,
    /// Every word the Pack says becomes this, numbered.
    said: &'static str,
    title: &'static str,
    by_the_quay: &'static str,
    fountain: &'static str,
    too_poor: &'static str,
    ask: &'static str,
    waiting: &'static str,
    bench: &'static str,
    herbs: &'static str,
    bunting: &'static str,
    gift: &'static str,
    invite: &'static str,
    talk_question: &'static str,
    talk_answer: &'static str,
    /// A favour's note, hint and quick reply, where the walk asks one.
    favour: Option<[&'static str; 3]>,
    legend: &'static str,
    /// A legend's line, given its number.
    legend_line: fn(u32) -> String,
    because: &'static str,
    /// Quotation marks the language does not use.
    not_quotes: &'static [char],
}

const CHINESE: Words = Words {
    language: world_i18n::Language::SimplifiedChinese,
    catalog: crate::i18n::APP_ZH_HANS,
    called: "Chinese",
    keep_in: "WORLD_MACHINE_ZH_WALK",
    said: "港口的话",
    title: "小小港口",
    by_the_quay: "在码头边",
    fountain: "喷泉",
    too_poor: "镇上的钱不够",
    ask: "要不要帮忙修码头？",
    waiting: "大家都在等你的答复。",
    bench: "长椅",
    herbs: "香草",
    bunting: "彩旗",
    gift: "礼物",
    invite: "邀请",
    talk_question: "最近好吗？",
    talk_answer: "还行吧。大家都说“港口今天也很太平”。",
    favour: Some([
        "港口的话：请你去看看她。",
        "去和她聊几句吧。",
        "我来看看你。最近好吗？",
    ]),
    legend: "港口的一生",
    legend_line: |line| format!("那年的第{line}件事。"),
    because: "因为你说了“好”",
    not_quotes: &[],
};

const JAPANESE: Words = Words {
    language: world_i18n::Language::Japanese,
    catalog: crate::i18n::APP_JA,
    called: "Japanese",
    keep_in: "WORLD_MACHINE_JA_WALK",
    said: "港のことば",
    title: "小さな港",
    by_the_quay: "波止場のそば",
    fountain: "噴水",
    too_poor: "町のお金が足りません",
    ask: "波止場の修理、手伝ってくれる？",
    waiting: "みんな、あなたの返事を待っています。",
    bench: "ベンチ",
    herbs: "ハーブ",
    bunting: "旗飾り",
    gift: "贈り物",
    invite: "お誘い",
    talk_question: "元気にしてる？",
    talk_answer: "まあまあだよ。「港は今日も平和だ」って、みんな言ってる。",
    favour: None,
    legend: "港の一生",
    legend_line: |line| format!("その年の{line}番目のできごと。"),
    because: "あなたが「いいよ」と言ったから",
    // Japanese quotes with 「」, never “”.
    not_quotes: &['“', '”'],
};

/// The harbour on its 358th day, three years of building laid over it,
/// with every word its Pack says turned into `words`' language (a
/// placeholder numbered so no two are the same) and only people's names
/// kept.
fn a_world_in(words: &Words) -> (ProjectionSnapshot, Vec<String>) {
    let mut base: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/harbour-day-358.json"))
            .expect("json");
    let later: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/harbour-day-1082.json"))
            .expect("json");
    for key in ["canvas", "goals", "world_time", "calendar"] {
        base[key] = later[key].clone();
    }
    const WORDS: &[&str] = &[
        "title",
        "label",
        "detail",
        "summary",
        "note",
        "what",
        "caption",
        "line",
        "prompt",
        "subtitle",
        "hint",
        "shelf",
        "coming",
        "words",
        "eyebrow",
        "thing",
        "cost",
        "unavailable",
        "asks_for",
        "because",
        "text",
        "answer",
        "question",
        "name",
        "reading",
        "arrived",
        "left",
        "born",
        "died",
        "built",
    ];
    let mut count = 0;
    fn said(value: &mut serde_json::Value, key: &str, count: &mut usize, keep: bool, word: &str) {
        match value {
            serde_json::Value::String(text) if WORDS.contains(&key) && !keep => {
                // A reading that is only a number stays one.
                if key == "reading" && !text.chars().any(|c| c.is_ascii_alphabetic()) {
                    return;
                }
                if key == "cost" {
                    return;
                }
                *count += 1;
                *text = format!("{word}{count}");
            }
            serde_json::Value::Array(list) => {
                for item in list {
                    said(item, key, count, keep, word);
                }
            }
            serde_json::Value::Object(map) => {
                let person = map.get("kind").and_then(|kind| kind.as_str()) == Some("actor");
                for (key, value) in map.iter_mut() {
                    said(value, key, count, person && key == "label", word);
                }
            }
            _ => {}
        }
    }
    said(&mut base, "", &mut count, false, words.said);
    base["title"] = words.title.into();
    base["calendar"]["season"] = "春".into();
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(&base.to_string()).expect("a wire snapshot");
    let mut snapshot = ProjectionSnapshot::try_from(wire).expect("a snapshot");
    // People's names, on the scene and in the almanac's cast.
    let mut names = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .flat_map(|item| {
            item.label
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if let Some(almanac) = &mut snapshot.almanac {
        for named in &almanac.cast {
            names.extend(named.name.split_whitespace().map(str::to_string));
        }
        for list in [
            &mut almanac.arrived,
            &mut almanac.left,
            &mut almanac.born,
            &mut almanac.died,
        ] {
            list.retain(|name| !name.starts_with(words.said));
        }
    }
    let someone = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.kind == CanvasItemKind::Actor)
        .map(|item| item.id)
        .expect("someone");
    let place = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.kind != CanvasItemKind::Actor)
        .map(|item| item.id)
        .expect("somewhere");
    let deed = |id: &str, verb: &str, thing: &str, at: SelectionId, cost: Option<&str>| {
        ProjectionCommand {
            id: id.into(),
            title: thing.into(),
            detail: words.by_the_quay.into(),
            effects: Vec::new(),
            scenery: None,
            moves: Vec::new(),
            asker: None,
            question: None,
            unavailable: None,
            hand: Some(Hand {
                verb: verb.into(),
                thing: thing.into(),
                at: Some(at),
                cost: cost.map(Into::into),
            }),
            preview: None,
            role: None,
        }
    };
    let mut poor = deed(
        "hands.build.fountain.1",
        "Build",
        words.fountain,
        place,
        Some("50"),
    );
    poor.unavailable = Some(words.too_poor.into());
    snapshot.commands = vec![
        ProjectionCommand {
            id: "ask".into(),
            title: words.ask.into(),
            detail: words.waiting.into(),
            effects: Vec::new(),
            scenery: None,
            moves: Vec::new(),
            asker: Some(someone),
            question: Some(Question {
                id: "q".into(),
                prompt: words.ask.into(),
            }),
            unavailable: None,
            hand: None,
            preview: None,
            role: None,
        },
        deed(
            "hands.build.bench.1",
            "Build",
            words.bench,
            place,
            Some("25"),
        ),
        poor,
        deed(
            "hands.plant.herbs.1",
            "Plant",
            words.herbs,
            place,
            Some("5"),
        ),
        deed(
            "hands.decorate.bunting.1",
            "Decorate",
            words.bunting,
            place,
            Some("10"),
        ),
        deed("hands.give.gift.1", "Give", words.gift, someone, Some("10")),
        deed("hands.invite.out.1", "Invite", words.invite, someone, None),
    ];
    snapshot.almanac_years = vec![1, 2, 3, 4, 5, 6, 7, 8];
    // Someone to talk to, as the person card shows them.
    snapshot.capabilities.talk = true;
    snapshot.talks = vec![world_projection::Talk {
        who: someone,
        question: words.talk_question.into(),
        answer: words.talk_answer.into(),
        asks_for: None,
    }];
    // What the player can say to open a talk, as the person card offers
    // it, and something someone gave them, as it is shown.
    snapshot.openers = vec![world_projection::Openers {
        who: someone,
        lines: vec![words.said.to_string(), words.talk_question.into()],
    }];
    snapshot
        .exchanges
        .retain(|exchange| exchange.who != someone);
    snapshot.keepsakes.push(world_projection::Keepsake {
        from: someone,
        what: words.gift.into(),
        note: words.waiting.into(),
        moment: someone,
    });
    // A favour asked, as its note in the drawer.
    snapshot.favour = words
        .favour
        .map(|[note, hint, reply]| world_projection::Favour {
            asker: someone,
            whom: someone,
            note: note.into(),
            hint: hint.into(),
            done: false,
            reply: reply.into(),
            thanks: String::new(),
        });
    (snapshot, names)
}

/// A World that tells its stories in `.1`'s language.
struct Telling(ProjectionSnapshot, &'static Words);

impl crate::ProjectionController for Telling {
    fn snapshot(&self) -> ProjectionSnapshot {
        self.0.clone()
    }

    fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
        Ok(self.0.clone())
    }

    fn story(&mut self, request: StoryRequest) -> Option<StoryPage> {
        match request {
            StoryRequest::Legend(subject) => Some(StoryPage::Legend(world_projection::Legend {
                subject,
                title: self.1.legend.into(),
                lines: (0..6)
                    .map(|line| LegendLine {
                        day: 700 + line * 40,
                        text: (self.1.legend_line)(line),
                        because: (line % 2 == 0).then(|| self.1.because.into()),
                        ..Default::default()
                    })
                    .collect(),
            })),
            StoryRequest::Almanac(year) => {
                let mut almanac = self.0.almanac.clone()?;
                almanac.year = year;
                Some(StoryPage::Almanac(almanac))
            }
            StoryRequest::Moment(id) => self
                .0
                .moments
                .iter()
                .find(|moment| moment.id == id)
                .cloned()
                .map(StoryPage::Moment),
        }
    }
}

/// Every window state a player reaches, in Chinese, says nothing in
/// English but people's names.
#[test]
fn the_world_window_speaks_no_english_in_chinese() {
    walk(&CHINESE);
}

/// Every window state a player reaches, in Japanese, says nothing in
/// English but people's names.
#[test]
fn the_world_window_speaks_no_english_in_japanese() {
    walk(&JAPANESE);
}

/// Walks every window state a player reaches in `words`' language, and
/// checks what was laid out: no English but people's names, no line
/// starting with punctuation, and none of the quotation marks the language
/// does not use.
fn walk(words: &'static Words) {
    world_i18n::install_for(words.language, words.catalog);
    world_i18n::set_thread_language(Some(words.language));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let text = Arc::new(Recording {
        inner: NoopTextSystem,
        lines: lines.clone(),
    });
    let mut app = TestAppContext::build_with_text_system(TestDispatcher::new(7), None, text);
    let (snapshot, names) = a_world_in(words);
    let someone = snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.kind == CanvasItemKind::Actor)
        .map(|item| item.id)
        .expect("someone");
    let plot = snapshot.canvas.plots.first().map(|plot| plot.id.clone());
    let moment = snapshot.moments.last().map(|moment| moment.id.clone());
    let window = app.add_window(move |_, _| {
        let mut view =
            ProjectionView::controlled(Telling(snapshot.clone(), words)).with_strip(|_, _| {});
        view.looking.opening = None;
        view
    });
    let cx = &mut VisualTestContext::from_window(window.into(), &app);
    cx.simulate_resize(gpui::size(px(1100.0), px(820.0)));
    let mut seen = Vec::<(String, String)>::new();
    let mut look =
        |state: &str,
         cx: &mut VisualTestContext,
         change: &dyn Fn(&mut ProjectionView, &mut Context<ProjectionView>)| {
            window
                .update(cx, |view, _, cx| {
                    change(view, cx);
                    cx.notify();
                })
                .expect("the window");
            cx.run_until_parked();
            let mut lines = lines.lock().expect("lines");
            seen.extend(lines.drain(..).map(|line| (state.to_string(), line)));
        };
    look("the card", cx, &|_, _| {});
    for leaf in super::drawer::Leaf::ALL {
        look(leaf.name(), cx, &move |view, _| {
            view.looking.drawer = true;
            view.looking.leaf = leaf;
            view.looking.years.clear();
            view.looking.years.extend([1, 2]);
        });
    }
    look("built, one kind", cx, &|view, _| {
        view.looking.leaf = super::drawer::Leaf::Built;
        view.looking.kind = Some(0);
    });
    for verb in ["Build", "Plant", "Decorate", "Give"] {
        look(verb, cx, &move |view, _| {
            view.looking.drawer = false;
            view.looking.hands = Some(super::world_window::Hands {
                verb: Some(verb.into()),
                thing: super::world_window::to_someone(verb).then(|| "*".into()),
            });
        });
    }
    look("a bench chosen", cx, &|view, _| {
        view.looking.hands = Some(super::world_window::Hands {
            verb: Some("Build".into()),
            thing: Some(words.bench.into()),
        });
    });
    if let Some(plot) = plot {
        look("a plot", cx, &move |view, _| {
            view.looking.hands = None;
            view.looking.marking.offers = Some((plot.clone(), 0));
        });
        look("a plot, planting", cx, &|view, _| {
            view.looking.marking.offers_verb = Some("Plant".into());
        });
    }
    look("the person card", cx, &move |view, cx| {
        view.looking.marking.offers = None;
        view.ask(someone, cx);
        view.looking.answered = Some((0, std::time::Instant::now()));
    });
    look("something given", cx, &|view, _| {
        view.looking.keepsakes_seen = Some(view.snapshot.keepsakes.len());
        view.looking.gift_at = Some(std::time::Instant::now());
    });
    look("a status", cx, &|view, _| {
        view.status = Some(crate::i18n::fill(
            "Couldn't say that: {error}",
            &[("error", "x")],
        ));
    });
    look("a legend", cx, &move |view, cx| {
        view.status = None;
        view.looking.asking = None;
        view.open_legend(someone, cx);
    });
    look("the almanac", cx, &|view, cx| {
        view.close_all_pages(cx);
        view.open_almanac(3, cx);
    });
    if let Some(moment) = moment {
        look("a moment", cx, &move |view, cx| {
            view.close_all_pages(cx);
            view.open_moment(&moment, cx);
        });
    }
    world_i18n::set_thread_language(None);
    // WORLD_MACHINE_ZH_WALK=<file> (or _JA_) keeps what was read, to look over.
    if let Ok(path) = std::env::var(words.keep_in) {
        let all = seen
            .iter()
            .map(|(state, line)| format!("{state}\t{line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(path, all);
    }
    assert!(seen.len() > 200, "{} lines laid out", seen.len());
    let english = seen
        .iter()
        .filter(|(_, line)| !english_in(line, &names).is_empty())
        .map(|(state, line)| format!("{state}: {line}"))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        english.is_empty(),
        "English in the {} window:\n{}",
        words.called,
        english.into_iter().collect::<Vec<_>>().join("\n")
    );
    // No line begins with a mark that closes or ends something.
    let marked = seen
        .iter()
        .filter(|(_, line)| {
            line.trim_start().starts_with([
                '。', '、', '，', '」', '）', '！', '？', '：', '；', '』', '”',
            ])
        })
        .map(|(state, line)| format!("{state}: {line}"))
        .collect::<Vec<_>>();
    assert!(
        marked.is_empty(),
        "lines starting with punctuation:\n{}",
        marked.join("\n")
    );
    // Quotation marks the language does not use (Japanese quotes with 「」).
    let quoted = seen
        .iter()
        .filter(|(_, line)| line.contains(words.not_quotes))
        .map(|(state, line)| format!("{state}: {line}"))
        .collect::<Vec<_>>();
    assert!(
        quoted.is_empty(),
        "{:?} in {}:\n{}",
        words.not_quotes,
        words.called,
        quoted.join("\n")
    );
}

/// A World open as the app's language changes is shown in the new
/// language at once, not when it is next opened.
#[test]
fn an_open_world_is_shown_in_a_new_language_at_once() {
    struct Live(ProjectionSnapshot);
    impl crate::ProjectionController for Live {
        fn snapshot(&self) -> ProjectionSnapshot {
            crate::i18n::localize(self.0.clone())
        }
        fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
            Ok(self.snapshot())
        }
    }
    world_i18n::install_for(world_i18n::Language::Japanese, crate::i18n::APP_JA);
    world_i18n::set_thread_language(Some(world_i18n::Language::English));
    let text = Arc::new(Recording {
        inner: NoopTextSystem,
        lines: Arc::new(Mutex::new(Vec::new())),
    });
    let mut app = TestAppContext::build_with_text_system(TestDispatcher::new(11), None, text);
    let snapshot = ProjectionSnapshot {
        title: "Tiny Society".into(),
        ..a_world_in(&JAPANESE).0
    };
    let window = app.add_window(move |_, _| {
        let mut view = ProjectionView::controlled(Live(snapshot.clone())).with_strip(|_, _| {});
        view.looking.opening = None;
        view
    });
    let cx = &mut VisualTestContext::from_window(window.into(), &app);
    cx.run_until_parked();
    let title = |cx: &mut VisualTestContext| {
        window
            .update(cx, |view, _, _| view.snapshot.title.clone())
            .expect("the window")
    };
    assert_eq!(title(cx), "Tiny Society");
    world_i18n::set_thread_language(Some(world_i18n::Language::Japanese));
    window
        .update(cx, |_, _, cx| cx.notify())
        .expect("the window");
    cx.run_until_parked();
    world_i18n::set_thread_language(None);
    assert_eq!(title(cx), "タイニー・ソサエティ");
}

/// No English is written into the window's code where it is shown: every
/// word goes through the catalogs (`ui::t`, `i18n::fill` and the rest),
/// never straight onto the screen as `.child("Skip to your turn")` or
/// `.child(format!("{from} gave you"))` were.
#[test]
fn no_words_go_on_screen_from_the_code_untranslated() {
    let sources = [
        ("window.rs", include_str!("../window.rs")),
        ("world_window.rs", include_str!("world_window.rs")),
        ("drawer.rs", include_str!("drawer.rs")),
        ("marking.rs", include_str!("marking.rs")),
        ("stories.rs", include_str!("stories.rs")),
        ("arrival.rs", include_str!("arrival.rs")),
        ("farewell.rs", include_str!("farewell.rs")),
        ("panels.rs", include_str!("../panels.rs")),
        ("ui.rs", include_str!("../ui.rs")),
        ("strip.rs", include_str!("../strip.rs")),
    ];
    let mut found = Vec::new();
    for (file, source) in sources {
        // The code, not its tests.
        let code = source
            .split("\n#[cfg(test)]\nmod ")
            .next()
            .unwrap_or(source);
        for (number, line) in code.lines().enumerate() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            for opening in [".child(\"", ".child(format!(\""] {
                for (at, _) in line.match_indices(opening) {
                    let said = &line[at + opening.len()..];
                    let said = said.split('"').next().unwrap_or("");
                    // What is outside the slots ("{from}") is what is read.
                    let mut read = String::new();
                    let mut depth = 0;
                    for c in said.chars() {
                        match c {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ if depth == 0 => read.push(c),
                            _ => {}
                        }
                    }
                    if !english_in(&read, &[]).is_empty() {
                        found.push(format!("{file}:{}: {line}", number + 1));
                    }
                }
            }
        }
    }
    assert!(found.is_empty(), "English on screen:\n{}", found.join("\n"));
}
