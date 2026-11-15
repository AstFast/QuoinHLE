//! The words a date control answers with, in the languages this host carries.
//!
//! `IDateCtl::GetDayString`, `GetMonthString` and `GetDateString` ask a control for text the
//! *platform* owns: a handset reads its weekday and month names out of its own localised table, not
//! out of the pack, which is why a pack cannot supply them itself. A host with no table has to refuse
//! those three slots, and that is what this host did until this table existed.
//!
//! ## Why the language is a command-line choice
//!
//! The device profile has a language word (`AEE_LNG_*`, written into `AEEDeviceInfo::dwLang` and used
//! to pick a resource's language block), and it would be neat to key this table off it. The values of
//! that family are not published anywhere in this tree — `tools/abi_coverage.py`'s table carries no
//! `AEE_LNG_*` name, `spec/brew_abi.json` has none either — so a mapping from the word to a language
//! would be a guess with a number in it. The two are therefore independent: `dwLang` keeps doing what
//! it already did for resources, and the words a run answers with come from `--lang`.
//!
//! What a pack sees is the same either way: text in the language the run was started with, and
//! English when the tag is one this host does not know.

/// The seven weekday names, Sunday first — the order `DOW_SUNDAY = 0` numbers them in.
pub type Days = [&'static str; 7];
/// The twelve month names, January first.
pub type Months = [&'static str; 12];

/// The order a whole date is written in, which is the part of a locale that is not a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    YearMonthDay,
    MonthDayYear,
    DayMonthYear,
}

/// One language: what a date control says for a weekday, a month, and a whole date.
#[derive(Debug)]
pub struct Language {
    /// The tag this language is chosen by, matched without regard to case and ignoring any region
    /// suffix, so `zh-Hans-CN` and `ZH` both land on the same entry.
    pub tag: &'static str,
    /// What people say. Each name has to be short enough to sit in a control's row, which is why the
    /// month names are words and not numbers with a suffix.
    pub days: Days,
    pub months: Months,
    pub order: Order,
}

/// The languages this host answers in. Not a claim about a handset's firmware: a stand-in, the same
/// way the glyph table is, and listed here so that a run in a language can be compared with the host
/// answering in English.
pub const LANGUAGES: &[Language] = &[
    Language {
        tag: "en",
        days: [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ],
        months: [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ],
        order: Order::MonthDayYear,
    },
    Language {
        tag: "zh",
        days: [
            "星期日",
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
        ],
        months: [
            "一月",
            "二月",
            "三月",
            "四月",
            "五月",
            "六月",
            "七月",
            "八月",
            "九月",
            "十月",
            "十一月",
            "十二月",
        ],
        order: Order::YearMonthDay,
    },
    Language {
        tag: "ja",
        days: [
            "日曜日",
            "月曜日",
            "火曜日",
            "水曜日",
            "木曜日",
            "金曜日",
            "土曜日",
        ],
        months: [
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
        order: Order::YearMonthDay,
    },
    Language {
        tag: "ko",
        days: [
            "일요일",
            "월요일",
            "화요일",
            "수요일",
            "목요일",
            "금요일",
            "토요일",
        ],
        months: [
            "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
        ],
        order: Order::YearMonthDay,
    },
    Language {
        tag: "fr",
        days: [
            "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
        ],
        months: [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ],
        order: Order::DayMonthYear,
    },
    Language {
        tag: "de",
        days: [
            "Sonntag",
            "Montag",
            "Dienstag",
            "Mittwoch",
            "Donnerstag",
            "Freitag",
            "Samstag",
        ],
        months: [
            "Januar",
            "Februar",
            "März",
            "April",
            "Mai",
            "Juni",
            "Juli",
            "August",
            "September",
            "Oktober",
            "November",
            "Dezember",
        ],
        order: Order::DayMonthYear,
    },
];

/// The tag this host answers in when a run names none or names one it does not carry.
pub const DEFAULT: &str = "en";

/// The language for a tag, or the default one when the tag is not carried.
///
/// A tag is matched on its first subtag only, so `en-GB` is English and `zh-Hans` is the Chinese
/// entry. Case does not matter. Nothing here is locale-exact — a region can change a month's
/// abbreviation — but staying silent about a language this host does have would be worse than
/// answering with the names of the language it is in.
pub fn find(tag: &str) -> &'static Language {
    let head = tag.split(['-', '_']).next().unwrap_or("").trim();
    LANGUAGES
        .iter()
        .find(|l| l.tag.eq_ignore_ascii_case(head))
        .or_else(|| LANGUAGES.iter().find(|l| l.tag == DEFAULT))
        .expect("the default language is in the table")
}

/// Every tag this host carries, for `--help` and for the error a bad `--lang` gets.
pub fn tags() -> Vec<&'static str> {
    LANGUAGES.iter().map(|l| l.tag).collect()
}

#[cfg(test)]
#[path = "test/locale.rs"]
mod tests;
