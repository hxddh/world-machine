//! A World shown in the player's language: its snapshot's words run
//! through the catalogs once as it arrives. Presentation only; the World
//! and everything it records keep their own words.

use world_i18n::{language, tr_owned, Language};
use world_projection::{Almanac, Moment, ProjectionSnapshot, StoryPage};

/// The app's own words in Simplified Chinese.
pub const APP_ZH_HANS: &str = include_str!("../locales/zh-Hans.tsv");

/// The app's own words in Japanese.
pub const APP_JA: &str = include_str!("../locales/ja.tsv");

fn put(text: &mut String) {
    if !text.is_empty() {
        *text = tr_owned(text);
    }
}

fn put_option(text: &mut Option<String>) {
    if let Some(text) = text {
        put(text);
    }
}

/// `snapshot` with every word the player reads in the app's language.
pub fn localize(mut snapshot: ProjectionSnapshot) -> ProjectionSnapshot {
    if language() == Language::English {
        return snapshot;
    }
    put(&mut snapshot.title);
    if let Some(briefing) = &mut snapshot.briefing {
        for item in &mut briefing.items {
            put(&mut item.title);
            put(&mut item.detail);
        }
    }
    for command in &mut snapshot.commands {
        put(&mut command.title);
        put(&mut command.detail);
        put_option(&mut command.unavailable);
        if let Some(question) = &mut command.question {
            put(&mut question.prompt);
        }
        if let Some(hand) = &mut command.hand {
            put(&mut hand.thing);
            put_option(&mut hand.cost);
        }
    }
    for item in &mut snapshot.timeline.items {
        put(&mut item.title);
        put(&mut item.subtitle);
    }
    // The plots and what could stand on them, and the districts they lie
    // in, as the build card names them.
    for plot in &mut snapshot.canvas.plots {
        for offer in &mut plot.offers {
            put(&mut offer.label);
            put_option(&mut offer.cost);
            put_option(&mut offer.unavailable);
        }
    }
    for district in &mut snapshot.canvas.districts {
        put(&mut district.label);
    }
    for item in &mut snapshot.canvas.items {
        put(&mut item.label);
        put(&mut item.detail);
        if let Some(standing) = &mut item.standing {
            put(&mut standing.words);
        }
    }
    // The unit stays in English: it is a key, and every screen says a
    // count of it through `day_label` and its kin below, which put the
    // whole phrase in order ("Day 12" is 第12天, not 日 12).
    if let Some(calendar) = &mut snapshot.calendar {
        put_option(&mut calendar.season);
        put_option(&mut calendar.coming);
    }
    for gauge in &mut snapshot.gauges {
        put(&mut gauge.label);
        // "14 of 14" is a count, said the same way whatever it counts.
        match gauge.reading.split_once(" of ") {
            Some((some, all))
                if !some.is_empty()
                    && !all.is_empty()
                    && some
                        .chars()
                        .chain(all.chars())
                        .all(|c| c.is_ascii_digit() || c == ',') =>
            {
                gauge.reading = format!("{some} / {all}");
            }
            _ => put(&mut gauge.reading),
        }
    }
    for talk in &mut snapshot.talks {
        put(&mut talk.question);
        put(&mut talk.answer);
        put_option(&mut talk.asks_for);
    }
    for exchange in &mut snapshot.exchanges {
        put(&mut exchange.answer);
    }
    for voice in &mut snapshot.voices {
        put(&mut voice.line);
    }
    for chapter in &mut snapshot.chapters {
        put(&mut chapter.title);
        put(&mut chapter.summary);
    }
    for goal in &mut snapshot.goals {
        put(&mut goal.label);
    }
    for keepsake in &mut snapshot.keepsakes {
        put(&mut keepsake.what);
        put(&mut keepsake.note);
    }
    for letter in &mut snapshot.letters {
        put(&mut letter.note);
    }
    if let Some(favour) = &mut snapshot.favour {
        put(&mut favour.note);
        put(&mut favour.hint);
    }
    for entry in &mut snapshot.book {
        put(&mut entry.shelf);
        put(&mut entry.name);
        put(&mut entry.hint);
    }
    for moment in &mut snapshot.moments {
        put_moment(moment);
    }
    if let Some(almanac) = &mut snapshot.almanac {
        put_almanac(almanac);
    }
    snapshot
}

fn put_moment(moment: &mut Moment) {
    put(&mut moment.title);
    for panel in &mut moment.panels {
        put(&mut panel.caption);
    }
}

fn put_almanac(almanac: &mut Almanac) {
    put(&mut almanac.title);
    for list in [
        &mut almanac.arrived,
        &mut almanac.left,
        &mut almanac.born,
        &mut almanac.died,
        &mut almanac.built,
    ] {
        for name in list {
            put(name);
        }
    }
    for named in &mut almanac.cast {
        put(&mut named.name);
    }
}

/// How a unit of a World's time is said in Chinese or Japanese: the nth
/// one, the next one, and a count of them. `None` for a unit this table
/// does not know.
struct ZhUnit {
    nth: &'static str,
    next: &'static str,
    count: &'static str,
}

fn zh_unit(unit: &str) -> Option<ZhUnit> {
    let unit = unit.trim().to_lowercase();
    let unit = unit.strip_suffix('s').unwrap_or(&unit);
    if japanese() {
        return ja_unit(unit);
    }
    Some(match unit {
        "day" | "日" | "天" => ZhUnit {
            nth: "第{n}天",
            next: "新的一天",
            count: "{n} 天",
        },
        "sol" | "火星日" => ZhUnit {
            nth: "第{n}个火星日",
            next: "下一个火星日",
            count: "{n} 个火星日",
        },
        "night" | "夜" | "夜晚" => ZhUnit {
            nth: "第{n}夜",
            next: "下一个夜晚",
            count: "{n} 个夜晚",
        },
        "aurora" | "极光" => ZhUnit {
            nth: "第{n}次极光",
            next: "下一次极光",
            count: "{n} 次极光",
        },
        "week" | "周" => ZhUnit {
            nth: "第{n}周",
            next: "新的一周",
            count: "{n} 周",
        },
        "time" => ZhUnit {
            nth: "时间 {n}",
            next: "下一刻",
            count: "{n} 刻",
        },
        _ => return None,
    })
}

fn ja_unit(unit: &str) -> Option<ZhUnit> {
    Some(match unit {
        "day" | "日" => ZhUnit {
            nth: "{n}日目",
            next: "新しい一日",
            count: "{n}日",
        },
        "sol" | "ソル" => ZhUnit {
            nth: "{n}ソル目",
            next: "次のソル",
            count: "{n}ソル",
        },
        "night" | "夜" => ZhUnit {
            nth: "{n}夜目",
            next: "次の夜",
            count: "{n}夜",
        },
        "aurora" | "オーロラ" => ZhUnit {
            nth: "{n}度目のオーロラ",
            next: "次のオーロラ",
            count: "オーロラ{n}回",
        },
        "week" | "週" => ZhUnit {
            nth: "{n}週目",
            next: "新しい一週間",
            count: "{n}週間",
        },
        "time" => ZhUnit {
            nth: "時刻 {n}",
            next: "次のひととき",
            count: "{n}刻",
        },
        _ => return None,
    })
}

fn chinese() -> bool {
    language() == Language::SimplifiedChinese
}

fn japanese() -> bool {
    language() == Language::Japanese
}

/// Whether the app is shown in Chinese now.
pub fn is_chinese() -> bool {
    chinese()
}

/// Whether the app is shown in Japanese now.
pub fn is_japanese() -> bool {
    japanese()
}

/// "Year 2", 第 2 年, 2年目.
pub fn year_label(year: u32) -> String {
    if chinese() {
        format!("第 {year} 年")
    } else if japanese() {
        format!("{year}年目")
    } else {
        format!("Year {year}")
    }
}

/// "Chapter 12", 第 12 章, 第12章.
pub fn chapter_label(number: u32) -> String {
    if chinese() {
        format!("第 {number} 章")
    } else if japanese() {
        format!("第{number}章")
    } else {
        format!("Chapter {number}")
    }
}

/// "12 chapters", 12 章, 全12章.
pub fn chapters_count(count: usize) -> String {
    match (chinese(), japanese(), count) {
        (true, _, _) => format!("{count} 章"),
        (_, true, _) => format!("全{count}章"),
        (_, _, 1) => "1 chapter".into(),
        _ => format!("{count} chapters"),
    }
}

/// "2 of 3 done", 已完成 2/3, 2/3 完了.
pub fn parts_done(done: u32, parts: u32) -> String {
    if chinese() {
        format!("已完成 {done}/{parts}")
    } else if japanese() {
        format!("{done}/{parts} 完了")
    } else {
        format!("{done} of {parts} done")
    }
}

/// "3 of 10", 3 / 10.
pub fn of_count(some: usize, all: usize) -> String {
    if chinese() || japanese() {
        format!("{some} / {all}")
    } else {
        format!("{some} of {all}")
    }
}

/// "All 18", 全部 18, すべて 18.
pub fn all_count(count: usize) -> String {
    if chinese() {
        format!("全部 {count}")
    } else if japanese() {
        format!("すべて {count}")
    } else {
        format!("All {count}")
    }
}

/// "Built · 18", 已建成 · 18, 完成 · 18.
pub fn built_count(count: usize) -> String {
    if chinese() {
        format!("已建成 · {count}")
    } else if japanese() {
        format!("完成 · {count}")
    } else {
        format!("Built · {count}")
    }
}

/// "Spring, Year 2", 第 2 年 · 春, 2年目 · 春.
pub fn season_in_year(season: &str, year: u32) -> String {
    if !chinese() && !japanese() {
        return format!("{season}, Year {year}");
    }
    let season = match season {
        "Spring" => "春".to_string(),
        "Summer" => "夏".to_string(),
        "Autumn" => "秋".to_string(),
        "Winter" => "冬".to_string(),
        other => tr_owned(other),
    };
    format!("{} · {season}", year_label(year))
}

/// "and 12 more", 还有 12 个, ほか 12 件.
pub fn more_kept(count: usize) -> String {
    if chinese() {
        format!("还有 {count} 个")
    } else if japanese() {
        format!("ほか {count} 件")
    } else {
        format!("and {count} more")
    }
}

/// "Choose where the bench goes.", 选择长椅放在哪里。, ベンチを置く場所を選んでください。
pub fn choose_where(thing: &str, now: bool) -> String {
    match (chinese(), japanese(), now) {
        (true, _, false) => format!("选择{thing}放在哪里。"),
        (true, _, true) => format!("选择把{thing}挪到哪里。"),
        (_, true, false) => format!("{thing}を置く場所を選んでください。"),
        (_, true, true) => format!("{thing}の移動先を選んでください。"),
        (_, _, false) => format!("Choose where the {} goes.", thing.to_lowercase()),
        (_, _, true) => format!("Choose where the {} goes now.", thing.to_lowercase()),
    }
}

/// "From Mara", 来自 Mara, マーラから.
pub fn from_whom(names: &str) -> String {
    if chinese() {
        format!("来自{names}")
    } else if japanese() {
        format!("{names}から")
    } else {
        format!("From {names}")
    }
}

/// "Mara and Leo", 2 names joined as the language joins them.
pub fn two_names(one: &str, two: &str) -> String {
    if chinese() {
        format!("{one} 和 {two}")
    } else if japanese() {
        format!("{one}と{two}")
    } else {
        format!("{one} and {two}")
    }
}

/// "Mara, Leo and 2 others".
pub fn names_and_others(one: &str, two: &str, others: usize) -> String {
    if chinese() {
        format!("{one}、{two}等 {} 人", others + 2)
    } else if japanese() {
        format!("{one}、{two}ほか{others}人")
    } else {
        format!("{one}, {two} and {others} others")
    }
}

/// The `n`th of a World's `unit`: "Day 12", "Sol 5"; in Chinese and
/// Japanese the whole phrase in its own order, 第12天, 12日目.
pub fn day_label(unit: &str, n: impl std::fmt::Display) -> String {
    let wide = chinese() || japanese();
    match zh_unit(unit).filter(|_| wide) {
        Some(zh) => zh.nth.replace("{n}", &n.to_string()),
        None if wide => format!("{} {n}", tr_owned(unit)),
        None => format!("{unit} {n}"),
    }
}

/// A moment in a World's own words, as the window shows it: "Sol 3",
/// "The beginning", in the app's language.
pub fn moment_label(snapshot: &ProjectionSnapshot, world_time: u64) -> String {
    match &snapshot.calendar {
        _ if world_time == 0 => tr_owned("The beginning"),
        Some(calendar) if calendar.length > 0 => {
            day_label(&calendar.unit, world_time.div_ceil(calendar.length))
        }
        _ => day_label("Time", world_time),
    }
}

/// A stretch of a World's time: "Sol 2–5", or one moment when both ends
/// fall on the same one.
pub fn span_label(snapshot: &ProjectionSnapshot, from: u64, to: u64) -> String {
    let (from, to) = (from.min(to), from.max(to));
    match &snapshot.calendar {
        Some(calendar) if calendar.length > 0 => {
            let first = from.div_ceil(calendar.length).max(1);
            let last = to.div_ceil(calendar.length).max(1);
            if first == last {
                moment_label(snapshot, to)
            } else {
                day_label(&calendar.unit, format!("{first}–{last}"))
            }
        }
        _ if from == to => moment_label(snapshot, to),
        _ => day_label("Time", format!("{from}–{to}")),
    }
}

/// "3 days", "1 sol": a count of a World's unit, lower case in English.
pub fn count_of(count: u64, unit: &str) -> String {
    let unit = unit.to_lowercase();
    match zh_unit(&unit).filter(|_| chinese() || japanese()) {
        Some(zh) => zh.count.replace("{n}", &count.to_string()),
        None if count == 1 => format!("1 {unit}"),
        None => format!("{count} {unit}s"),
    }
}

/// "3 days have passed": how long a World lived while the player was
/// away.
pub fn time_passed(periods: u64, unit: &str) -> String {
    if chinese() {
        return format!("已过去 {}", count_of(periods, unit));
    }
    if japanese() {
        return format!("{}が過ぎました", count_of(periods, unit));
    }
    let verb = if periods == 1 { "has" } else { "have" };
    format!("{} {verb} passed", count_of(periods, unit))
}

/// "Keeps going without you · next sol in 5 h": what the app is about, in
/// one line, with the one number that makes it true. In Chinese and
/// Japanese, whole sentences in their own order.
pub fn keeps_going(remaining_seconds: u64, unit: &str) -> String {
    let unit = unit.to_lowercase();
    let zh = zh_unit(&unit).filter(|_| chinese() || japanese());
    if remaining_seconds == 0 {
        return match zh {
            Some(zh) if japanese() => {
                format!(
                    "留守のあいだも暮らしは続きます · 次に来たときには{}",
                    zh.next
                )
            }
            Some(zh) => format!("你不在时，这里照常继续 · {}在等你下次到访", zh.next),
            None => format!("Keeps going without you · a new {unit} waits for your next visit"),
        };
    }
    let (count, english, chinese_unit, japanese_unit) = if remaining_seconds >= 3600 {
        (remaining_seconds.div_ceil(3600), "h", "小时", "時間")
    } else {
        (remaining_seconds.div_ceil(60).max(1), "min", "分钟", "分")
    };
    match zh {
        Some(zh) if japanese() => format!(
            "留守のあいだも暮らしは続きます · あと{count}{japanese_unit}で{}",
            zh.next
        ),
        Some(zh) => format!(
            "你不在时，这里照常继续 · {count} {chinese_unit}后便是{}",
            zh.next
        ),
        None => format!("Keeps going without you · next {unit} in {count} {english}"),
    }
}

/// "Here since Day 703": when someone came, in the World's own count.
pub fn here_since(unit: &str, day: u32) -> String {
    if chinese() {
        return format!("自{}起在这里", day_label(unit, day));
    }
    if japanese() {
        return format!("{}からここに", day_label(unit, day));
    }
    format!("Here since {}", day_label(unit, day))
}

/// A story the World told, with every word the player reads in the app's
/// language.
pub fn localize_story(mut page: StoryPage) -> StoryPage {
    if language() == Language::English {
        return page;
    }
    match &mut page {
        StoryPage::Legend(legend) => {
            put(&mut legend.title);
            for line in &mut legend.lines {
                put(&mut line.text);
                put_option(&mut line.because);
            }
        }
        StoryPage::Moment(moment) => put_moment(moment),
        StoryPage::Almanac(almanac) => put_almanac(almanac),
    }
    page
}

/// The app's own catalogs, one for each language it is shown in but
/// English, for tests that every word is in each of them.
#[cfg(test)]
pub(crate) fn app_catalogs() -> [world_i18n::Catalog; 2] {
    [
        world_i18n::Catalog::parse(APP_ZH_HANS),
        world_i18n::Catalog::parse(APP_JA),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_i18n::set_thread_language;
    use world_projection::{Calendar, District, Gauge, Plot, PlotOffer, Tone};

    fn calendar(unit: &str) -> ProjectionSnapshot {
        ProjectionSnapshot {
            calendar: Some(Calendar {
                unit: unit.into(),
                length: 10,
                season: None,
                coming: None,
                festival_today: false,
                year: Some(120),
            }),
            ..ProjectionSnapshot::default()
        }
    }

    /// A World's own count reads as a whole phrase in Chinese, never as
    /// "日 1082"; English is as it was.
    #[test]
    fn a_day_is_counted_in_the_language_shown() {
        set_thread_language(Some(Language::English));
        assert_eq!(moment_label(&calendar("Day"), 10820), "Day 1082");
        assert_eq!(moment_label(&calendar("Sol"), 50), "Sol 5");
        assert_eq!(time_passed(3, "night"), "3 nights have passed");
        assert_eq!(
            keeps_going(5 * 3600 + 10, "sol"),
            "Keeps going without you · next sol in 6 h"
        );
        set_thread_language(Some(Language::SimplifiedChinese));
        assert_eq!(moment_label(&calendar("Day"), 10820), "第1082天");
        assert_eq!(moment_label(&calendar("Sol"), 1560), "第156个火星日");
        assert_eq!(moment_label(&calendar("Night"), 1200), "第120夜");
        assert_eq!(span_label(&calendar("Day"), 20, 50), "第2–5天");
        assert_eq!(time_passed(5, "day"), "已过去 5 天");
        let line = keeps_going(6 * 3600, "day");
        assert_eq!(line, "你不在时，这里照常继续 · 6 小时后便是新的一天");
        assert!(!line.chars().any(|c| c.is_ascii_alphabetic()));
        assert_eq!(here_since("Day", 703), "自第703天起在这里");
        set_thread_language(Some(Language::Japanese));
        assert_eq!(moment_label(&calendar("Day"), 10820), "1082日目");
        assert_eq!(moment_label(&calendar("Sol"), 1560), "156ソル目");
        assert_eq!(span_label(&calendar("Day"), 20, 50), "2–5日目");
        assert_eq!(time_passed(5, "day"), "5日が過ぎました");
        let line = keeps_going(6 * 3600, "day");
        assert_eq!(
            line,
            "留守のあいだも暮らしは続きます · あと6時間で新しい一日"
        );
        assert!(!line.chars().any(|c| c.is_ascii_alphabetic()));
        assert_eq!(year_label(2), "2年目");
        assert_eq!(season_in_year("Spring", 2), "2年目 · 春");
        set_thread_language(None);
    }

    /// The build card's words and a count on a gauge come through in
    /// Chinese.
    #[test]
    fn plots_districts_and_counts_are_shown_in_chinese() {
        world_i18n::install("Cheese shop\t奶酪店\nThe square\t广场\nIn work\t在职\n");
        let mut snapshot = ProjectionSnapshot::default();
        snapshot.canvas.plots.push(Plot {
            id: "p".into(),
            offers: vec![PlotOffer {
                label: "Cheese shop".into(),
                cost: Some("50".into()),
                ..Default::default()
            }],
            ..Default::default()
        });
        snapshot.canvas.districts.push(District {
            id: "square".into(),
            label: "The square".into(),
            from: 0.0,
            to: 1.0,
        });
        snapshot.gauges.push(Gauge {
            id: "work".into(),
            label: "In work".into(),
            value: 1.0,
            reading: "14 of 14".into(),
            tone: Tone::Good,
        });
        set_thread_language(Some(Language::SimplifiedChinese));
        let shown = localize(snapshot);
        set_thread_language(None);
        assert_eq!(shown.canvas.plots[0].offers[0].label, "奶酪店");
        assert_eq!(shown.canvas.districts[0].label, "广场");
        assert_eq!(shown.gauges[0].reading, "14 / 14");
    }
}
