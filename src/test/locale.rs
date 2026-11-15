//! Tests for the host's own language table.

use crate::locale::{self, Order};

#[test]
fn an_unknown_or_empty_tag_gets_the_default_language() {
    assert_eq!(locale::find("nope").tag, locale::DEFAULT);
    assert_eq!(locale::find("").tag, locale::DEFAULT);
    assert_eq!(locale::find("   ").tag, locale::DEFAULT);
}

#[test]
fn a_tag_is_matched_on_its_language_subtag_whatever_the_case_or_region() {
    for tag in ["en", "EN", "en-GB", "en_US", " en "] {
        assert_eq!(locale::find(tag).tag, "en", "tag {tag}");
    }
    assert_eq!(locale::find("zh-Hans-CN").tag, "zh");
    assert_eq!(locale::find("JA").tag, "ja");
    assert_eq!(locale::find("de-AT").tag, "de");
    // `zhx` is not `zh`: a subtag has to end at a separator, not be a prefix of a longer word.
    assert_eq!(locale::find("zhx").tag, locale::DEFAULT);
}

#[test]
fn every_language_names_seven_days_and_twelve_months_and_none_of_them_is_blank() {
    for l in locale::LANGUAGES {
        assert_eq!(l.days.len(), 7, "{}", l.tag);
        assert_eq!(l.months.len(), 12, "{}", l.tag);
        for d in l.days {
            assert!(!d.trim().is_empty(), "{} has an empty day", l.tag);
        }
        for m in l.months {
            assert!(!m.trim().is_empty(), "{} has an empty month", l.tag);
        }
    }
}

#[test]
fn the_days_are_in_dow_order_which_starts_on_sunday() {
    // `DOW_SUNDAY` is 0 and `DOW_SATURDAY` is 6, so entry zero has to be Sunday and entry six
    // Saturday. Checking two languages that share no letters is what makes this a check of the order
    // rather than of the spelling: a table shifted by one would pass a spelling test in either.
    let en = locale::find("en");
    assert!(en.days[0].starts_with("Sun"), "{:?}", en.days);
    assert!(en.days[6].starts_with("Sat"), "{:?}", en.days);
    assert!(en.months[0].starts_with("Jan"), "{:?}", en.months);
    assert!(en.months[11].starts_with("Dec"), "{:?}", en.months);

    let zh = locale::find("zh");
    assert!(zh.days[0].ends_with("日"), "{:?}", zh.days);
    assert!(zh.days[6].ends_with("六"), "{:?}", zh.days);

    let ja = locale::find("ja");
    assert!(ja.days[0].starts_with("日"), "{:?}", ja.days);
    assert!(ja.days[6].starts_with("土"), "{:?}", ja.days);
}

#[test]
fn every_tag_is_listed_once_and_can_be_found_by_itself() {
    let tags = locale::tags();
    assert!(tags.contains(&locale::DEFAULT), "{tags:?}");
    let mut sorted = tags.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), tags.len(), "a tag is listed twice: {tags:?}");
    for t in tags {
        assert_eq!(locale::find(t).tag, t);
    }
}

#[test]
fn the_date_orders_are_the_ones_each_language_writes() {
    assert_eq!(locale::find("en").order, Order::MonthDayYear);
    assert_eq!(locale::find("fr").order, Order::DayMonthYear);
    assert_eq!(locale::find("de").order, Order::DayMonthYear);
    assert_eq!(locale::find("zh").order, Order::YearMonthDay);
    assert_eq!(locale::find("ja").order, Order::YearMonthDay);
    assert_eq!(locale::find("ko").order, Order::YearMonthDay);
}
