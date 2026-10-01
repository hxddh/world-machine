//! A walk through the World window in Simplified Chinese: every word it
//! lays out, in every state a player reaches (the card, the drawer's
//! leaves, the build card from the hands and from a plot, a legend, a
//! moment, the almanac), recorded as the text system shapes it. The World
//! here speaks only Chinese, apart from people's names; so any English
//! word left on screen is the app's own, untranslated.

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

/// Words that read the same in Chinese: the app's name, the Mac, a key.
const ALLOWED: &[&str] = &["World", "Machine", "Mac", "API", "Esc", "OK"];

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

/// The harbour on its 358th day, three years of building laid over it,
/// with every word its Pack says turned into Chinese (a placeholder
/// numbered so no two are the same) and only people's names kept.
fn a_chinese_world() -> (ProjectionSnapshot, Vec<String>) {
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
    fn chinese(value: &mut serde_json::Value, key: &str, count: &mut usize, keep: bool) {
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
                *text = format!("港口的话{count}");
            }
            serde_json::Value::Array(list) => {
                for item in list {
                    chinese(item, key, count, keep);
                }
            }
            serde_json::Value::Object(map) => {
                let person = map.get("kind").and_then(|kind| kind.as_str()) == Some("actor");
                for (key, value) in map.iter_mut() {
                    chinese(value, key, count, person && key == "label");
                }
            }
            _ => {}
        }
    }
    chinese(&mut base, "", &mut count, false);
    base["title"] = "小小港口".into();
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
            list.retain(|name| !name.starts_with("港口的话"));
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
            detail: "在码头边".into(),
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
        }
    };
    let mut poor = deed("hands.build.fountain.1", "Build", "喷泉", place, Some("50"));
    poor.unavailable = Some("镇上的钱不够".into());
    snapshot.commands = vec![
        ProjectionCommand {
            id: "ask".into(),
            title: "要不要帮忙修码头？".into(),
            detail: "大家都在等你的答复。".into(),
            effects: Vec::new(),
            scenery: None,
            moves: Vec::new(),
            asker: Some(someone),
            question: Some(Question {
                id: "q".into(),
                prompt: "要不要帮忙修码头？".into(),
            }),
            unavailable: None,
            hand: None,
            preview: None,
        },
        deed("hands.build.bench.1", "Build", "长椅", place, Some("25")),
        poor,
        deed("hands.plant.herbs.1", "Plant", "香草", place, Some("5")),
        deed(
            "hands.decorate.bunting.1",
            "Decorate",
            "彩旗",
            place,
            Some("10"),
        ),
        deed("hands.give.gift.1", "Give", "礼物", someone, Some("10")),
        deed("hands.invite.out.1", "Invite", "邀请", someone, None),
    ];
    snapshot.almanac_years = vec![1, 2, 3, 4, 5, 6, 7, 8];
    // A favour asked, as its note in the drawer.
    snapshot.favour = Some(world_projection::Favour {
        asker: someone,
        whom: someone,
        note: "港口的话：请你去看看她。".into(),
        hint: "去和她聊几句吧。".into(),
        done: false,
    });
    (snapshot, names)
}

/// A World that tells its stories in Chinese.
struct Telling(ProjectionSnapshot);

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
                title: "港口的一生".into(),
                lines: (0..6)
                    .map(|line| LegendLine {
                        day: 700 + line * 40,
                        text: format!("那年的第{line}件事。"),
                        because: (line % 2 == 0).then(|| "因为你说了“好”".into()),
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
    world_i18n::install(crate::i18n::APP_ZH_HANS);
    world_i18n::set_thread_language(Some(world_i18n::Language::SimplifiedChinese));
    let lines = Arc::new(Mutex::new(Vec::new()));
    let text = Arc::new(Recording {
        inner: NoopTextSystem,
        lines: lines.clone(),
    });
    let mut app = TestAppContext::build_with_text_system(TestDispatcher::new(7), None, text);
    let (snapshot, names) = a_chinese_world();
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
        let mut view = ProjectionView::controlled(Telling(snapshot.clone())).with_strip(|_, _| {});
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
            thing: Some("长椅".into()),
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
    look("a legend", cx, &move |view, cx| {
        view.looking.marking.offers = None;
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
    // WORLD_MACHINE_ZH_WALK=<file> keeps what was read, to look over.
    if let Ok(path) = std::env::var("WORLD_MACHINE_ZH_WALK") {
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
        "English in the Chinese window:\n{}",
        english.into_iter().collect::<Vec<_>>().join("\n")
    );
}
