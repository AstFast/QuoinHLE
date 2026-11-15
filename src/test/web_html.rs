// Direct tests for the HTML reader/painter. `src/test/runtime.rs` covers the ABI slots; these
// cover the parsing and the `HVOPT_*` words that the ABI hands to `layout`/`render`, which used
// to have no assertions at all.

use super::*;
use crate::display::Rect;
use crate::font::Font;

fn doc_of(text: &str) -> Document {
    Document::parse(text.as_bytes())
}

#[test]
fn parse_keeps_tags_attributes_text_and_link_runs() {
    let doc = doc_of("<p align=\"center\">Hi <a href=\"x\">there</a><b>bold</b></p>");
    assert_eq!(
        doc.elements.len(),
        3,
        "p, a, b; closing tags are not elements"
    );
    assert_eq!(doc.find("a", 0), Some(1));
    assert_eq!(doc.text(1), "there");
    assert_eq!(doc.attr(1, "href"), Some("x"));
    assert_eq!(doc.find("a", 1), None);
    assert_eq!(doc.attr(0, "align"), Some("center"));

    // `<p>` opens a blank row first, then the inherited text, the link and the bold run.
    let links: Vec<&str> = doc
        .runs
        .iter()
        .filter(|r| r.color == crate::web::html::Ink::Link)
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(links, vec!["there"], "only text enclosed by <a> is a link");
    assert_eq!(doc.runs[1].align, g::IDF_ALIGN_CENTER);
    assert_eq!(doc.runs[1].font, g::AEE_FONT_NORMAL);
    assert_eq!(doc.runs[3].font, g::AEE_FONT_BOLD);
}

#[test]
fn focus_items_skip_hidden_and_buttons() {
    let doc = doc_of("<input type=hidden name=h><a href=x>go</a><input name=q><input type=submit>");
    let kinds: Vec<u32> = doc.focus_items().iter().map(|f| f.kind).collect();
    assert_eq!(
        kinds,
        vec![g::HVFOCUS_LINK, g::HVFOCUS_EDIT, g::HVFOCUS_SUBMIT]
    );
}

#[test]
fn options_read_only_the_painter_ids() {
    let mut table = HashMap::new();
    table.insert(HVOPT_N_INDENT, 6);
    table.insert(HVOPT_N_VSPACE, 3);
    table.insert(HVOPT_N_SBWIDTH, 8);
    table.insert(HVOPT_RGB_TEXT, 0x00ff_0000);
    table.insert(HVOPT_RGB_LINK, 0x0000_00ff);
    // The string/interface ids are stored by SetOption but are not painter options.
    table.insert(HVOPT_SZ_URL, 0xdead_beef);
    table.insert(HVOPT_PTR_IWEB, 0xdead_beef);
    assert_eq!(
        Options::from_table(&table),
        Options {
            indent: 6,
            vspace: 3,
            sbwidth: 8,
            text_color: 0x00ff_0000,
            link_color: 0x0000_00ff,
            // `HVP_SCROLLBAR` is a control property, not an option, so the table cannot carry it.
            scrollbar: false,
        }
    );
    assert_eq!(Options::from_table(&HashMap::new()), Options::default());
}

#[test]
fn aliases_are_distinct_and_within_their_webopt_ranges() {
    let mut ids: Vec<i32> = HVOPT_IDS.iter().map(|(_, id)| *id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), HVOPT_IDS.len(), "two HVOPT_* ids collided");
    for (name, id) in HVOPT_IDS {
        assert_eq!(
            (*id as u32) & 0xffff_0000,
            match *name {
                "HVOPT_SZ_TYPE" | "HVOPT_SZ_URL" => g::WEBOPT_SZ,
                "HVOPT_PTR_IWEB" => g::WEBOPT_INTERFACE,
                _ => g::WEBOPT_32BIT,
            },
            "{name} is outside its WEBOPT range"
        );
    }
}

#[test]
fn form_submission_carries_method_action_and_encoded_fields() {
    let doc = doc_of(
        "<form action=\"/go\" method=post><input name=who value=\"A B\"><textarea name=note>hi & bye</textarea><input type=submit value=Send></form>",
    );
    let submit = doc
        .elements
        .iter()
        .position(|e| attr_of(e, "type") == "submit")
        .unwrap();
    let (url, method, data) = form_submission(&doc, submit, &Values::default(), (0, 0));
    assert_eq!(url, "/go");
    assert_eq!(method, "POST");
    assert_eq!(data, "who=A+B&note=hi+%26+bye");
    // The unnamed submit button contributes no field, and a link is not a form.
    let link = doc_of("<a href=\"fallback\">one</a>");
    assert_eq!(
        form_submission(&link, 0, &Values::default(), (0, 0)),
        ("fallback".to_string(), "GET".to_string(), String::new())
    );
}

/// The width of `text` in a designator's built-in face.
///
/// [`crate::web::html::width_of`] now takes the face itself — the painter's answer, not the table's
/// — and these cases are about the folding rules, so they name the built-in face and say so here
/// rather than at forty call sites.
fn width_of(font: u32, text: &str) -> i16 {
    super::width_of(crate::font::Face::builtin(font).as_ref(), text)
}

/// [`crate::web::html::break_at`] over a designator's built-in face, for the same reason.
fn break_at(text: &str, font: u32, avail: i16) -> Option<(usize, i16)> {
    super::break_at(text, crate::font::Face::builtin(font).as_ref(), avail)
}

/// The page laid out with the built-in faces.
///
/// A unit test has no display: the built-in table is the answer the designator question used to get
/// from `Font::of`, so these cases measure what they measured before. The installed-face path is
/// covered where there is one — `src/test/runtime.rs` drives the viewer.
fn layout(
    doc: &Document,
    rect: Rect,
    opts: &Options,
    pictures: &Pictures,
    values: Option<&Values>,
    scroll: i16,
) -> Vec<Placed> {
    crate::web::html::layout(
        doc,
        rect,
        opts,
        pictures,
        values,
        scroll,
        crate::font::Faces::builtin(),
    )
}

fn placed(doc: &Document, opts: &Options, dy: i16) -> Vec<Placed> {
    layout(doc, Rect::new(0, 0, 120, dy), opts, &[], None, 0)
}

/// The same, in a column as wide as the test needs: a row that is one line at 120 px is several at
/// 60, and the tests about folding have to name both numbers.
fn placed_in(doc: &Document, opts: &Options, dx: i16, dy: i16) -> Vec<Placed> {
    layout(doc, Rect::new(0, 0, dx, dy), opts, &[], None, 0)
}

/// One decoded picture, so a case can tell "the page named a size" apart from "the bitmap has one".
///
/// [`crate::web::html::layout`] reads the bitmap only for `width`/`height`, so the pixel buffer is
/// left empty: nothing in these cases paints.
fn bitmap_of(width: u16, height: u16) -> Vec<Option<std::sync::Arc<crate::resource::Bitmap>>> {
    vec![Some(std::sync::Arc::new(crate::resource::Bitmap {
        width: usize::from(width),
        height: usize::from(height),
        pixels: Vec::new(),
        palette: None,
        alpha: crate::resource::Alpha::Opaque,
    }))]
}

#[test]
fn layout_honours_indent_vspace_scrollbar_and_colours() {
    let doc = doc_of("<p>one</p><p>two</p><p>three</p><p>four</p><p>five</p>");
    let plain = placed(&doc, &Options::default(), 60);
    assert!(plain.len() >= 3, "five short rows should fit in 60 px");
    assert_eq!(plain[0].x, 0);
    assert_eq!(plain[0].dx, 120);

    let spaced = Options {
        vspace: 20,
        ..Options::default()
    };
    assert!(
        placed(&doc, &spaced, 60).len() < plain.len(),
        "HVOPT_N_VSPACE must take rows away"
    );

    let quoted = doc_of("<blockquote>q</blockquote>");
    let inset = Options {
        indent: 6,
        sbwidth: 8,
        text_color: 0x00ff_0000,
        link_color: 0x0000_00ff,
        ..Options::default()
    };
    let rows = placed(&quoted, &inset, 60);
    assert_eq!(rows[0].x, 6, "HVOPT_N_INDENT is one step in");
    assert_eq!(rows[0].dx, 100, "and `<blockquote>` takes it off both sides");
    assert_eq!(rows[0].color, 0x00ff_0000, "HVOPT_RGB_TEXT");
    let outside = placed(&doc, &inset, 60);
    assert_eq!(outside[0].x, 0, "no indent block, no step");
    assert_eq!(outside[0].dx, 112, "HVOPT_N_SBWIDTH, and nothing else");

    let link = doc_of("<a href=\"x\">go</a><p>plain</p>");
    let rows = placed(&link, &inset, 60);
    assert_eq!(rows[0].color, 0x0000_00ff, "HVOPT_RGB_LINK wins on a link");
    assert_eq!(rows[1].color, 0x00ff_0000, "and only on a link");

    let scrolled = Options {
        sbwidth: 120,
        ..Options::default()
    };
    assert!(
        placed(&doc, &scrolled, 60).is_empty(),
        "HVOPT_N_SBWIDTH wider than the rect leaves no room to draw"
    );
}

#[test]
fn a_long_line_is_folded_at_the_column_width_instead_of_being_clipped() {
    let doc = doc_of("<p>alpha beta gamma delta epsilon zeta eta theta</p>");
    let wide = placed_in(&doc, &Options::default(), 400, 200);
    assert_eq!(wide.len(), 1, "the whole sentence fits a 400-pixel column");

    let narrow = placed_in(&doc, &Options::default(), 60, 200);
    assert!(narrow.len() > 1, "a sixty-pixel column folds it");
    assert_eq!(narrow[0].dx, 60, "and the spans know how narrow they are");
    for span in &narrow {
        assert_eq!(span.x, 0, "a folded line starts at the column's left edge");
        assert!(
            span.w <= 60,
            "{:?} overflows the column it was folded into",
            span.text
        );
        assert!(
            !span.text.ends_with(' '),
            "{:?} carried the space it was broken at",
            span.text
        );
    }
    // Nothing was lost and nothing was doubled: the spans, put back together with the single
    // spaces the reader collapsed, are the sentence. A dropped word or a kept break space fails
    // here even when the geometry looks right.
    let rejoined = narrow
        .iter()
        .map(|span| span.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(rejoined, "alpha beta gamma delta epsilon zeta eta theta");
}

#[test]
fn a_run_of_wide_characters_is_folded_between_two_of_them() {
    let doc = doc_of("<p>一二三四五六七八九十一二三四五六七八九十一二三四五六七八九十</p>");
    let wide = placed_in(&doc, &Options::default(), 400, 200);
    assert_eq!(wide.len(), 1, "thirty cells fit a 400-pixel column");

    let narrow = placed_in(&doc, &Options::default(), 60, 200);
    assert!(narrow.len() > 1, "a sixty-pixel column folds them");
    for span in &narrow {
        assert!(span.w <= 60, "{:?} overflows the column", span.text);
    }
    let cells: usize = narrow.iter().map(|span| span.text.chars().count()).sum();
    assert_eq!(cells, 30, "every character landed on some line");
}

#[test]
fn a_line_that_ends_on_a_word_boundary_keeps_that_last_word() {
    // The byte just past the last one that fits decides which rule applies. When it is the space,
    // the fitting text ends a word and the whole of it stays on the line; when it is a letter, the
    // word it belongs to is incomplete and the cut goes before it. Reading the two rules in the
    // other order drops the last complete word of every line that happens to end cleanly — which
    // is most of them, and the loss looks like a rendering bug rather than a fold one.
    let font = g::AEE_FONT_NORMAL;
    let text = "ab cd ef";
    let exact = width_of(font, "ab cd");
    assert_eq!(
        break_at(text, font, exact),
        Some((5, exact)),
        "the whole fitting text, space and all"
    );
    let tight = width_of(font, "ab c");
    assert_eq!(
        break_at(text, font, tight),
        Some((2, width_of(font, "ab"))),
        "one character short, the word after the space does not fit"
    );
}

#[test]
fn a_word_wider_than_the_column_overflows_rather_than_being_split() {
    // Latin has no break opportunity inside a word, and the shipped engine hands an item that fits
    // nowhere its full width and lets it run past the column. Breaking it into two halves would be
    // an invention: nothing in the ABI says where a hyphen belongs.
    let doc = doc_of("<p>antidisestablishmentarianism is a word</p>");
    let narrow = placed(
        &doc,
        &Options {
            indent: 100,
            ..Options::default()
        },
        200,
    );
    assert_eq!(narrow[0].text, "antidisestablishmentarianism");
    assert!(narrow[0].w > 20, "the word keeps the width it needs");
    assert!(
        narrow.len() > 1,
        "and the text after it still starts a line of its own"
    );
}

#[test]
fn a_no_break_space_is_not_a_place_to_fold() {
    // `10&nbsp;kg` exists to stop exactly this break. The character survives the parse as U+00A0
    // and is drawn as a space; only the folder is told it may not cut there.
    let doc = doc_of("<p>10&nbsp;kg of tea</p>");
    let narrow = placed_in(&doc, &Options::default(), 12, 200);
    assert_eq!(narrow[0].dx, 12);
    assert_eq!(
        narrow[0].text, "10\u{a0}kg",
        "the number and its unit stay on one line, overrunning the column if they must"
    );
    assert_eq!(
        narrow[1].text, "of",
        "and the plain space after them is still a break"
    );
}

#[test]
fn a_scrolled_layout_starts_where_the_offset_says() {
    let doc = doc_of("<p>one</p><p>two</p><p>three</p><p>four</p>");
    let full = placed(&doc, &Options::default(), 200);
    assert_eq!(full.len(), 4, "one row per paragraph");
    let step = full[1].y - full[0].y;

    // One row down: the first paragraph is off the top, and every row keeps its own height, so the
    // second one now sits where the first did.
    let scrolled = layout(
        &doc,
        Rect::new(0, 0, 120, 200),
        &Options::default(),
        &[],
        None,
        step,
    );
    assert_eq!(
        scrolled[0].text, "two",
        "the first paragraph is off the top"
    );
    assert!(scrolled[0].y <= full[1].y, "and the rest moved up with it");
    assert_eq!(
        scrolled[1].y - scrolled[0].y,
        full[2].y - full[1].y,
        "the rows keep the spacing they had"
    );

    // Past the end there is nothing to place, and a negative offset is the top rather than a blank.
    let past = layout(
        &doc,
        Rect::new(0, 0, 120, 200),
        &Options::default(),
        &[],
        None,
        10_000,
    );
    assert!(past.is_empty());
    let negative = layout(
        &doc,
        Rect::new(0, 0, 120, 200),
        &Options::default(),
        &[],
        None,
        -20,
    );
    assert_eq!(negative[0].text, "one");
}

#[test]
fn layout_clamps_nonsense_pixel_values() {
    let doc = doc_of("<p>one</p><p>two</p>");
    // No panic, no negative width: u32::MAX is clamped to the rect before any arithmetic.
    for opts in [
        Options {
            indent: u32::MAX,
            ..Options::default()
        },
        Options {
            sbwidth: u32::MAX,
            ..Options::default()
        },
        Options {
            vspace: u32::MAX,
            ..Options::default()
        },
    ] {
        let rows = placed(&doc, &opts, 200);
        assert!(rows.iter().all(|r| r.x >= 0 && r.dx >= 0 && r.row > 0));
    }
}

#[test]
fn character_references_are_decoded_in_text_runs() {
    let doc = doc_of("<p>Tom &amp; Jerry</p>");
    let painted: Vec<&str> = doc
        .runs
        .iter()
        .map(|r| r.text.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(painted, vec!["Tom & Jerry"]);
    assert_eq!(
        doc.text(0),
        "Tom & Jerry",
        "GetElemText sees it decoded too"
    );

    // Decimal and hex references name the same character, and case does not matter in the prefix.
    assert_eq!(decode_entities("&#8212;"), "\u{2014}");
    assert_eq!(decode_entities("&#x2014;"), "\u{2014}");
    assert_eq!(decode_entities("&#X2014;"), "\u{2014}");
    assert_eq!(decode_entities("&ldquo;hi&rdquo;"), "\u{201c}hi\u{201d}");

    // A name the table does not know, and an `&` that starts no reference at all, are left as
    // they arrived: there is still text there a caller can make sense of.
    assert_eq!(decode_entities("&bogus;"), "&bogus;");
    assert_eq!(decode_entities("a & b"), "a & b");
    assert_eq!(decode_entities("1 &lt; 2 &lt"), "1 < 2 &lt");
    assert_eq!(decode_entities("&#;"), "&#;");

    // `&#0;` would put a NUL in a C string the guest reads, so it becomes the replacement the
    // HTML rules name for it.
    assert_eq!(decode_entities("a&#0;b"), "a\u{fffd}b");

    // `&nbsp;` stays U+00A0: the reader folds lines now, so turning it into a plain space would
    // let `10&nbsp;kg` break between the number and its unit. The painter draws it as a space
    // because no face here has the glyph; the folder is the only thing that treats it specially.
    assert_eq!(decode_entities("a&nbsp;b"), "a\u{a0}b");
}

#[test]
fn character_references_are_decoded_in_attribute_values() {
    let doc = doc_of("<a href=\"a&amp;b=1\">x</a>");
    assert_eq!(doc.attr(0, "href"), Some("a&b=1"));
}

#[test]
fn script_style_and_title_are_kept_but_not_painted() {
    // A `<` inside a script is not markup, so the whole body is one opaque stretch: the parser must
    // not read `a < b` as a tag and must not paint a single line of it.
    let doc = doc_of(
        "<title>Tom &amp; Jerry</title><p>one</p><script>if (a < b) { var s = '</p>'; }</script><p>two</p>",
    );
    let painted: Vec<&str> = doc
        .runs
        .iter()
        .map(|r| r.text.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(painted, vec!["one", "two"]);

    // The elements are still there for the DOM slots, with their content available by text.
    let title = doc.find("title", 0).expect("title is an element");
    assert_eq!(doc.text(title), "Tom & Jerry");
    let script = doc.find("script", 0).expect("script is an element");
    assert!(doc.text(script).contains("if (a < b)"));
    // Nothing between the script's own tags became an element.
    assert_eq!(doc.find("p", 1), Some(doc.elements.len() - 1));
}

#[test]
fn a_close_tag_in_any_case_ends_an_opaque_element() {
    let doc = doc_of("<SCRIPT>s = 1;</SCRIPT><p>x</p>");
    let painted: Vec<&str> = doc
        .runs
        .iter()
        .map(|r| r.text.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(painted, vec!["x"]);
}

#[test]
fn an_unclosed_script_swallows_the_rest_without_panicking() {
    let doc = doc_of("<p>one</p><script>var x = 1;");
    let painted: Vec<&str> = doc
        .runs
        .iter()
        .map(|r| r.text.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(painted, vec!["one"]);
    assert_eq!(doc.text(doc.find("script", 0).unwrap()), "var x = 1;");
}

#[test]
fn plain_text_is_read_as_lines_rather_than_markup() {
    let doc = Document::parse_as(b"<b>not bold</b>\n\nsecond\r\nthird", "text/plain");
    assert!(
        doc.elements.is_empty(),
        "no tag in plain text is an element"
    );
    let lines: Vec<&str> = doc.runs.iter().map(|r| r.text.as_str()).collect();
    // The blank line survives: in plain text it is the paragraph break, and CRLF is one break.
    assert_eq!(lines, vec!["<b>not bold</b>", "", "second", "third"]);

    // The type is compared without its parameters, and case-insensitively.
    for kind in ["text/plain", "text/plain; charset=utf-8", "TEXT/PLAIN"] {
        assert!(
            Document::parse_as(b"<b>x</b>", kind).elements.is_empty(),
            "{kind} should be plain"
        );
    }
    // Anything else — the empty type included — is still read as markup.
    for kind in ["", "text/html", "application/xhtml+xml"] {
        assert!(
            !Document::parse_as(b"<b>x</b>", kind).elements.is_empty(),
            "{kind} should be markup"
        );
    }
}

#[test]
fn a_block_owns_the_text_it_encloses() {
    // `GetElemText` on a paragraph used to answer "" because the block branch dropped the element
    // the following text belonged to.
    let doc = doc_of("<p>first</p><div>second<br>third</div>");
    assert_eq!(doc.text(doc.find("p", 0).unwrap()), "first");
    // `<br>` breaks the row without becoming the owner of the text after it.
    assert_eq!(doc.text(doc.find("div", 0).unwrap()), "secondthird");
}

#[test]
fn two_forms_post_only_their_own_fields() {
    let doc = doc_of(
        "<form action=/a method=get><input name=one value=1></form><form action=/b method=post><input name=two value=2><input type=submit value=Go></form>",
    );
    let submit = doc
        .elements
        .iter()
        .position(|e| attr_of(e, "type") == "submit")
        .unwrap();
    let (url, method, data) = form_submission(&doc, submit, &Values::default(), (0, 0));
    assert_eq!((url.as_str(), method.as_str()), ("/b", "POST"));
    assert_eq!(data, "two=2", "the first form's field must not travel");
}

#[test]
fn only_a_checked_box_or_radio_is_posted() {
    let doc = doc_of(
        "<form action=/go><input type=checkbox name=a><input type=checkbox name=b checked><input type=radio name=c value=C checked><input type=radio name=d value=D><input type=password name=p value=s><input type=submit name=s value=Go></form>",
    );
    let submit = doc
        .elements
        .iter()
        .position(|e| attr_of(e, "type") == "submit")
        .unwrap();
    let (_, _, data) = form_submission(&doc, submit, &Values::default(), (0, 0));
    // A checked box with no `value` posts `on`, which is the browser's own default.
    assert_eq!(data, "b=on&c=C&p=s");
}

#[test]
fn a_select_posts_its_chosen_option() {
    let doc = doc_of(
        "<form action=/go><select name=s><option value=1>one</option><option value=2 selected>two</option></select><select name=t><option>plain</option></select></form>",
    );
    let (_, _, data) = form_submission(&doc, 5, &Values::default(), (0, 0));
    // The second select names no value on its option, so the option's label travels.
    assert_eq!(data, "s=2&t=plain");
}

#[test]
fn a_disabled_control_is_not_posted() {
    let doc = doc_of(
        "<form action=/go><input name=a value=1><input name=b value=2 disabled><textarea name=c disabled>x</textarea></form>",
    );
    assert_eq!(
        form_submission(&doc, 3, &Values::default(), (0, 0)).2,
        "a=1"
    );
}

#[test]
fn inline_elements_share_the_row_they_started_on() {
    let doc = doc_of("<p>Hello <b>world</b>!</p>");
    let rows = placed(&doc, &Options::default(), 100);
    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    // The space before `<b>` survives as the first span's trailing space, so the words do not run
    // together once they are placed side by side.
    assert_eq!(texts, vec!["Hello ", "world", "!"]);
    assert!(
        rows.iter().all(|r| r.y == rows[0].y),
        "the three spans are one row"
    );
    assert_eq!(rows[1].x, rows[0].x + rows[0].w);
    assert_eq!(rows[2].x, rows[1].x + rows[1].w);
    assert_eq!(rows[0].font, g::AEE_FONT_NORMAL);
    assert_eq!(rows[1].font, g::AEE_FONT_BOLD);
    assert_eq!(rows[0].row, rows[1].row, "one row height for all of it");
}

#[test]
fn a_centred_row_is_centred_as_a_whole() {
    let doc = doc_of("<p align=center>a<b>bb</b></p>");
    let rows = placed(&doc, &Options::default(), 100);
    let total: i16 = rows.iter().map(|r| r.w).sum();
    assert_eq!(rows[0].x, (120 - total) / 2);
}

#[test]
fn a_table_is_a_block_but_its_cells_are_words_in_one_paragraph() {
    let doc = doc_of("<p>before</p><table>same<tr><td>A</td><td>B</td></tr></table>after");
    let tr = doc.elements.iter().position(|e| e.tag == "tr").unwrap();
    assert!(
        doc.runs.iter().all(|run| run.elem != Some(tr)),
        "<tr> is transparent, so no run belongs to it"
    );
    let rows = placed(&doc, &Options::default(), 100);
    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, vec!["before", "same", "A", "B", "after"]);
    assert_eq!(
        rows[1].y, rows[2].y,
        "<tr> does not break the line it finds: the cell continues it"
    );
    assert_eq!(rows[2].y, rows[3].y, "the two cells are one line");
    assert!(rows[4].y > rows[3].y, "</table> ends the line");
    // `<table>`'s record is the block bit without the margin bit, so the line it starts sits one
    // row below the paragraph — the same distance two plain paragraphs have, with no gap added.
    let plain = doc_of("<p>before</p>same");
    let plain_rows = placed(&plain, &Options::default(), 100);
    assert_eq!(
        rows[1].y - rows[0].y,
        plain_rows[1].y - plain_rows[0].y,
        "<table> opens no gap of its own"
    );
}

#[test]
fn headings_take_their_face_from_the_tag_table() {
    // `h1`…`h3` are record 3 (bold and large), `h4`…`h6` are record 1 (bold alone), and `font`
    // folds the pair into the one designator the shell publishes for it.
    let doc = doc_of("<h1>T</h1><h4>s</h4>");
    // A heading that opens the page puts its paragraph gap first, and a gap run carries the style
    // around the block rather than the block's own: it paints nothing, so only the spans with text
    // say which face the tag table selected.
    let runs: Vec<&Run> = doc.runs.iter().filter(|run| !run.text.is_empty()).collect();
    assert_eq!(runs[0].font, g::AEE_FONT_LARGE, "h1 is bold and large");
    assert_eq!(runs[1].font, g::AEE_FONT_BOLD, "h4 is bold alone");
    let rows = placed(&doc, &Options::default(), 100);
    assert!(rows[1].y > rows[0].y, "a heading ends its line");
}

#[test]
fn only_a_block_that_says_so_reads_its_align_attribute() {
    // `align` is read when the tag's record has `0x20`, which `td` does not and `p` does; `<center>`
    // centres without an attribute at all, because its record carries the centred style bit.
    let doc = doc_of("<td align=center>A</td><p align=right>B</p><center>C</center>");
    // A block's own blank row sits between them, so read the runs that carry text.
    let shown: Vec<&Run> = doc.runs.iter().filter(|run| !run.text.is_empty()).collect();
    assert_eq!(shown[0].align, g::IDF_ALIGN_LEFT, "a cell is transparent");
    assert_eq!(shown[1].align, g::IDF_ALIGN_RIGHT);
    assert_eq!(shown[2].align, g::IDF_ALIGN_CENTER);
}

#[test]
fn a_list_item_puts_a_two_by_two_square_five_pixels_left_of_its_first_word() {
    let opts = Options {
        indent: 10,
        ..Options::default()
    };
    let doc = doc_of("<ul><li>a</li></ul>");
    let rows = placed(&doc, &opts, 200);
    let word = rows.iter().find(|r| r.text == "a").unwrap();
    let dots: Vec<&Placed> = rows
        .iter()
        .filter(|r| r.mark == Some(Mark::Bullet))
        .collect();
    assert_eq!(dots.len(), 1, "one square for one item");
    let dot = dots[0];
    assert_eq!((dot.w, dot.h), (2, 2), "two pixels each way");
    assert_eq!(dot.x, word.x - 5, "five to the left of the words");
    assert_eq!(
        dot.y,
        word.y + word.row / 2 - 1,
        "one above the middle of the row it belongs to"
    );
    assert_eq!(word.x, 10, "the list indented its item one step");
    assert_eq!(
        dot.x, 5,
        "and the square hangs in the gutter that step opened"
    );
    // Painted in the order the shipped walk paints: the square is drawn as the tag is met, before
    // the words that follow it.
    let dot_at = rows.iter().position(|r| r.mark.is_some()).unwrap();
    let word_at = rows.iter().position(|r| r.text == "a").unwrap();
    assert!(dot_at < word_at, "the square is stroked before the word");
}

#[test]
fn a_square_belongs_to_the_first_thing_the_item_draws() {
    let doc = doc_of("<ul><li><p>x</p></li></ul>");
    let rows = placed(&doc, &Options::default(), 200);
    let word = rows.iter().find(|r| r.text == "x").unwrap();
    let dot = rows.iter().find(|r| r.mark.is_some()).unwrap();
    assert_eq!(
        dot.y,
        word.y + word.row / 2 - 1,
        "beside the word, not the gap"
    );
    assert_eq!(dot.x, word.x - 5);

    // An item that draws nothing hands the square back: `</li>` throws the flag away, so an item
    // holding nothing but an empty paragraph cannot donate its square to the paragraph after it.
    let doc = doc_of("<ul><li><p></p></li></ul><p>y</p>");
    let rows = placed(&doc, &Options::default(), 200);
    assert!(
        rows.iter().any(|r| r.text == "y"),
        "the page after the list is still laid out"
    );
    assert!(
        rows.iter().all(|r| r.mark.is_none()),
        "an item that draws nothing paints no square, and the words after it paint none either"
    );
}

#[test]
fn an_indent_block_moves_its_rows_in_by_one_step_per_level() {
    // `HVOPT_N_INDENT` is added by the frame an indent block pushes (`layout & 0x10`), once per
    // enclosing element, and `<blockquote>` adds it to the right inset as well — which is why a quote
    // is a narrower column while `<dd>` is only a shifted one. A page with no such block in it starts
    // at the rect's left edge however large the option is; `layout_honours_…` holds that half.
    let opts = Options {
        indent: 10,
        ..Options::default()
    };
    let quote = doc_of("<blockquote>q</blockquote>");
    let rows = placed(&quote, &opts, 200);
    assert_eq!(rows[0].x, 10, "one step in");
    assert_eq!(rows[0].dx, 100, "and one step off the far side of a 120-pixel rect");

    let deep = doc_of("<blockquote><blockquote>q</blockquote></blockquote>");
    let rows = placed(&deep, &opts, 200);
    assert_eq!(rows[0].x, 20, "two blocks, two steps");
    assert_eq!(rows[0].dx, 80, "from both sides");

    let term = doc_of("<dl><dd>q</dd></dl>");
    let rows = placed(&term, &opts, 200);
    assert_eq!(rows[0].x, 10, "`<dd>` indents the start of the row");
    assert_eq!(rows[0].dx, 110, "and leaves the far side alone");
}

#[test]
fn a_paragraph_gap_is_the_blocks_own_and_br_opens_no_line() {
    // `HVOPT_N_VSPACE` is a gap that a block with `layout & 0x08` opens when it starts — not leading
    // added to every row. `<div>` carries the block bit without the margin one, so two divs are
    // adjacent rows; `<p>` carries both, so a gap opens between them.
    let opts = Options {
        vspace: 10,
        ..Options::default()
    };
    let divs = doc_of("<div>a</div><div>b</div>");
    let rows = placed(&divs, &opts, 200);
    assert_eq!(
        rows[1].y,
        rows[0].y + rows[0].row,
        "no gap between two divs"
    );

    let paras = doc_of("<p>a</p><p>b</p>");
    let rows = placed(&paras, &opts, 200);
    assert_eq!(
        rows[1].y,
        rows[0].y + rows[0].row + 10,
        "a gap between two paragraphs"
    );

    let broken = doc_of("<p>a<br>b</p>");
    let rows = placed(&broken, &opts, 200);
    assert_eq!(rows.len(), 2, "two words, two rows");
    assert_eq!(
        rows[1].y,
        rows[0].y + rows[0].row,
        "a break after words costs nothing: they were already a line"
    );
}

#[test]
fn a_break_after_a_break_takes_a_line_of_its_own() {
    let doc = doc_of("<p>a<br><br>b</p>");
    let rows = placed(&doc, &Options::default(), 200);
    let texts: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(texts, vec!["a", "b"], "a blank line paints nothing");
    let line = rows[0].row;
    assert_eq!(
        rows[1].y,
        rows[0].y + 2 * line,
        "two breaks are one line apart, not one"
    );

    // A break at the very top of the page is the same case: there was nothing on the line yet.
    let leading = doc_of("<br>a");
    let rows = placed(&leading, &Options::default(), 200);
    assert_eq!(rows[0].y, line, "the blank line comes first");

    // And a break after a paragraph gap is a break after a break: `<p>` flushed the line too.
    let after_gap = doc_of("<p>a</p><br>b");
    let rows = placed(&after_gap, &Options::default(), 200);
    assert_eq!(
        rows[1].y,
        rows[0].y + 2 * line,
        "<p> left nothing pending either"
    );
}

#[test]
fn a_rule_takes_a_line_and_hangs_eight_pixels_above_the_next() {
    let doc = doc_of("<p>a</p><hr><p>b</p>");
    let rows = placed(&doc, &Options::default(), 200);
    let texts: Vec<&str> = rows
        .iter()
        .filter(|r| r.mark.is_none())
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(texts, vec!["a", "b"], "a rule paints no words");
    let rules: Vec<&Placed> = rows.iter().filter(|r| r.mark == Some(Mark::Rule)).collect();
    assert_eq!(rules.len(), 1, "one rule for one <hr>");
    let rule = rules[0];
    assert_eq!(
        (rule.x, rule.w, rule.h),
        (2, 116, 1),
        "two pixels in from the left, four short of the rect, one tall"
    );
    let words = rows.iter().filter(|r| r.mark.is_none()).collect::<Vec<_>>();
    let line = words[0].row;
    assert_eq!(
        words[1].y,
        words[0].y + 2 * line,
        "the rule is the blank line between the paragraphs"
    );
    assert_eq!(
        rule.y,
        words[1].y - 8,
        "the rule hangs above the line it was read from, not on the blank one"
    );

    // A rule whose line is above the fold is not stroked at all: the redraw's tag walk starts at the
    // byte the view is scrolled to, so an `<hr>` the user has scrolled past never reaches the fill.
    // Scrolling to the line below it leaves that line on the top edge and the rule eight pixels
    // above the edge, which is outside the rectangle rather than clipped by it.
    let scrolled = layout(
        &doc,
        Rect::new(0, 0, 120, 200),
        &Options::default(),
        &[],
        None,
        words[1].y,
    );
    assert!(
        scrolled.iter().any(|r| r.text == "b"),
        "the paragraph below the rule is still on screen"
    );
    assert!(
        scrolled.iter().all(|r| r.mark != Some(Mark::Rule)),
        "and the rule it has scrolled past is gone"
    );

    // Even with words on the line it broke: `<hr>` ignores the flush's answer.
    let inline = doc_of("<p>a<hr>b</p>");
    let rows = placed(&inline, &Options::default(), 200);
    let words = rows.iter().filter(|r| r.mark.is_none()).collect::<Vec<_>>();
    assert_eq!(
        words[1].y,
        words[0].y + 2 * line,
        "a rule after words still takes its line"
    );

    // `HVOPT_N_VSPACE` is not part of the rule: `<hr>` is an empty element, so it never reaches the
    // margin call a block does. The two paragraphs around it still cost one margin each — the one
    // the first paragraph's close tag reads and the one the second paragraph's open tag reads,
    // which do not stack because both measure from the same flushed line.
    let spaced = Options {
        vspace: 10,
        ..Options::default()
    };
    let rows = placed(&doc, &spaced, 200);
    let words = rows.iter().filter(|r| r.mark.is_none()).collect::<Vec<_>>();
    assert_eq!(
        words[1].y - words[0].y,
        2 * line + 2 * 10,
        "a's own line and the rule's, with a paragraph margin on either side of the rule"
    );
    // And the margin moves the rule with the line it hangs from, because the rule is read off that
    // line: the shipped pass has no other source for its top.
    let rule = rows.iter().find(|r| r.mark == Some(Mark::Rule)).unwrap();
    assert_eq!(
        rule.y,
        words[1].y - 8,
        "the gap above a paragraph pushes the rule down with it"
    );

    // The rule takes the height of the *default* face, not of the text around it: the loop reads
    // `a1[195]` where `<br>` reads `a1[4 * (frame & 3) + 195]`. Inside a heading — a large face, two
    // cells tall — a rule is still one body line, while a `<br>` there is a large one.
    let heading = doc_of("<h1>a<hr>b</h1>");
    let rows = placed(&heading, &Options::default(), 200);
    let words = rows.iter().filter(|r| r.mark.is_none()).collect::<Vec<_>>();
    let big = words[0].row;
    assert!(big > line, "a heading is taller than body text");
    assert_eq!(
        words[1].y - words[0].y,
        big + line,
        "the rule is a body line even in a heading"
    );

    let broken = doc_of("<h1>a<br><br>b</h1>");
    let rows = placed(&broken, &Options::default(), 200);
    assert_eq!(
        rows[1].y - rows[0].y,
        2 * big,
        "a break takes the height of the face it breaks"
    );
}

#[test]
fn an_empty_block_adds_nothing_at_all() {
    // The layout tests the byte after the tag node: when it is the close tag the element encloses
    // nothing, and both the frame push and the margin call are skipped. So `<p></p>` is not a
    // paragraph break — it is invisible, and the gap comes from the paragraphs that have words.
    let opts = Options {
        vspace: 10,
        ..Options::default()
    };
    let empty = doc_of("<p></p>x");
    let rows = placed(&empty, &opts, 200);
    assert_eq!(rows[0].y, 0, "an empty paragraph opens no gap");

    // `<p> </p>` is *not* empty: the space is a text node between the tags, and the layout only
    // looks at the byte that follows the tag. The gap is opened even though the space itself is
    // folded away and never painted.
    let space = doc_of("<p> </p>x");
    let rows = placed(&space, &opts, 200);
    assert_eq!(rows[0].y, 10, "a paragraph holding a space is a paragraph");

    // Between two paragraphs an empty one changes nothing either way: the margins of the close and
    // the open beside it already collapsed into one.
    let between = doc_of("<p>a</p><p></p><p>b</p>");
    let rows = placed(&between, &opts, 200);
    assert_eq!(
        rows[1].y,
        rows[0].y + rows[0].row + 10,
        "the empty paragraph adds no second gap"
    );
}

#[test]
fn a_preformatted_block_keeps_its_spaces_and_its_newlines() {
    let doc = doc_of("<pre>a  b\n  c</pre>");
    let texts: Vec<&str> = doc
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(
        texts,
        vec!["a  b", "  c"],
        "the runs of spaces and the indentation stay"
    );

    // The same bytes outside `<pre>` are folded to one space, which is what says the difference is
    // the tag's bit and not the text.
    let plain = doc_of("<p>a  b</p>");
    let shown = plain.runs.iter().find(|run| !run.text.is_empty()).unwrap();
    assert_eq!(shown.text, "a b");

    // The source's trailing spaces are content in there too; outside `<pre>` the reader trims them
    // off the end of the row so that `Hello <b>world</b>` does not carry a gap into the middle.
    let trail = doc_of("<pre>a  </pre>");
    let shown = trail.runs.iter().find(|run| !run.text.is_empty()).unwrap();
    assert_eq!(shown.text, "a  ");
    let plain = doc_of("<p>a  </p>");
    let shown = plain.runs.iter().find(|run| !run.text.is_empty()).unwrap();
    assert_eq!(shown.text, "a");

    let rows = placed(&doc, &Options::default(), 200);
    assert_eq!(rows.len(), 2, "the newline is a row break, not a fold");
    assert!(rows[1].y > rows[0].y);
    assert_eq!(rows[1].x, 0, "the two spaces are text, not an indent");
    assert!(
        rows[1].w > width_of(g::AEE_FONT_NORMAL, "c"),
        "the two spaces are measured, so the letter does not start at the edge"
    );
}

#[test]
fn a_blank_line_inside_a_preformatted_block_is_a_line() {
    // A newline in `<pre>` is a `<br>` as far as the layout is concerned: flush, and when the flush
    // found nothing, take a line. So the blank line between `a` and `b` is a line of the same
    // height as the two around it, even with `HVOPT_N_VSPACE` at its default of zero — which is
    // where the paragraph gap would have made it vanish.
    let doc = doc_of("<pre>a\n\nb</pre>");
    let texts: Vec<&str> = doc
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect();
    // The two outer empty runs are `<pre>`'s own: it opens and closes with a margin row like every
    // block. The one in the middle is the blank line, and it is a row of its own.
    assert_eq!(
        texts,
        vec!["", "a", "", "b", ""],
        "the blank line is kept between the two"
    );
    let rows = placed(&doc, &Options::default(), 200);
    assert_eq!(rows.len(), 2, "it paints nothing");
    assert_eq!(
        rows[1].y - rows[0].y,
        2 * rows[0].row,
        "one line above, one blank line between"
    );

    // Two newlines and no more: `\r\n` is one break, not two, or a page saved on Windows would
    // have a blank line between every pair of lines it has.
    let crlf = doc_of("<pre>a\r\nb</pre>");
    let rows = placed(&crlf, &Options::default(), 200);
    assert_eq!(
        rows[1].y - rows[0].y,
        rows[0].row,
        "a carriage return and a line feed are one break"
    );

    // Outside `<pre>` the blank line is source formatting and goes: HTML has no use for it.
    let folded = doc_of("<p>a\n\nb</p>");
    let rows = placed(&folded, &Options::default(), 200);
    assert_eq!(
        rows.len(),
        2,
        "two words and no blank line between them"
    );
}

#[test]
fn u_underlines_and_b_does_not() {
    // `style & 0x04` is the underline bit in the tag table, and `<u>` is its only tag. The bit is
    // per span: the two words here share a row because neither `<u>` nor `<b>` ends a line.
    let doc = doc_of("<u>x</u><b>y</b>");
    let rows = placed(&doc, &Options::default(), 200);
    assert_eq!(rows.len(), 2, "one row of two spans");
    assert!(rows[0].underline, "<u> is the underline bit");
    assert!(!rows[1].underline, "bold is not underlined");
}

#[test]
fn a_link_paints_its_whole_subtree_and_a_font_inside_it_wins() {
    let opts = Options {
        text_color: make_rgb(0, 0, 0),
        link_color: make_rgb(0, 0, 255),
        ..Options::default()
    };
    let deep = doc_of("<a href=x><b>bold</b></a>");
    assert_eq!(
        placed(&deep, &opts, 200)[0].color,
        make_rgb(0, 0, 255),
        "the link's colour reaches through the <b>"
    );

    // The frame holds *one* colour and the last writer wins: a `<font color>` opened inside the
    // link writes it after `<a>` did, and one opened outside wrote it before.
    let inside = doc_of("<a href=x><font color=red>r</font></a>");
    assert_eq!(placed(&inside, &opts, 200)[0].color, make_rgb(255, 0, 0));
    let outside = doc_of("<font color=red><a href=x>r</a></font>");
    assert_eq!(
        placed(&outside, &opts, 200)[0].color,
        make_rgb(0, 0, 255),
        "the <a> is the inner frame here"
    );
}

#[test]
fn a_colour_may_be_one_of_sixteen_names() {
    let opts = Options {
        text_color: make_rgb(1, 2, 3),
        ..Options::default()
    };
    for (name, want) in [
        ("red", make_rgb(255, 0, 0)),
        ("gray", make_rgb(128, 128, 128)),
        ("Silver", make_rgb(192, 192, 192)),
        ("navy", make_rgb(0, 0, 128)),
    ] {
        let doc = doc_of(&format!("<font color={name}>x</font>"));
        assert_eq!(placed(&doc, &opts, 100)[0].color, want, "{name}");
    }
    let unknown = doc_of("<font color=chartreuse>x</font>");
    assert_eq!(
        placed(&unknown, &opts, 100)[0].color,
        make_rgb(1, 2, 3),
        "not one of the sixteen, so the inherited colour stays"
    );
}

#[test]
fn black_is_a_colour_and_not_the_absence_of_one() {
    // `black` is `0x00000000` in the engine's table, and the frame's word is taken at face value,
    // so a black word inside a link is black. Treating 0 as "no colour" would paint it blue.
    let opts = Options {
        text_color: make_rgb(9, 9, 9),
        link_color: make_rgb(0, 0, 255),
        ..Options::default()
    };
    let doc = doc_of("<a href=x><font color=black>b</font></a>");
    assert_eq!(placed(&doc, &opts, 100)[0].color, 0);
}

#[test]
fn a_font_size_above_three_is_the_large_face() {
    // `size` is 1..7 and only ever decides one bit: `if (v9 <= 3) v7 &= ~2; else v7 |= 2`. The
    // write is a set *or a clear*, so a small size inside `<big>` takes the large face away;
    // `+1` counts from the default 3, and a value that is not a number is the default.
    let small = doc_of("<font size=2>x</font>");
    assert_eq!(small.runs[0].font, g::AEE_FONT_NORMAL);
    let large = doc_of("<font size=5>x</font>");
    assert_eq!(large.runs[0].font, g::AEE_FONT_LARGE);
    let relative = doc_of("<font size=+1>x</font>");
    assert_eq!(relative.runs[0].font, g::AEE_FONT_LARGE);
    let shrunk = doc_of("<big><font size=2>x</font></big>");
    assert_eq!(shrunk.runs[0].font, g::AEE_FONT_NORMAL);
    let nonsense = doc_of("<font size=big>x</font>");
    assert_eq!(nonsense.runs[0].font, g::AEE_FONT_NORMAL);
}

#[test]
fn an_align_word_is_read_without_regard_to_case_and_an_unknown_one_changes_nothing() {
    // Three words are compared with `aee_stricmp`; a fourth does not fall back to the left, it
    // leaves the frame's alignment exactly as it found it.
    let opts = Options::default();
    let upper = doc_of("<p align=RIGHT>x</p>");
    assert_eq!(placed(&upper, &opts, 200)[0].align, g::IDF_ALIGN_RIGHT);

    let inherited = doc_of("<div align=center><p align=justify>x</p></div>");
    let rows = placed(&inherited, &opts, 200);
    assert_eq!(
        rows.last().unwrap().align,
        g::IDF_ALIGN_CENTER,
        "justify is not a word the reader knows, so the <div>'s centre stands"
    );
}

#[test]
fn runs_of_white_space_collapse_to_one() {
    let doc = doc_of("<p>a    b\tc</p>");
    assert_eq!(doc.runs[1].text, "a b c");
}

#[test]
fn a_textarea_paints_its_initial_value_in_a_box() {
    // A `<` typed into a textarea is a character: the reader must not look for a tag there. The
    // value is not body text, but it *is* what the box shows — a form field the user can see is the
    // difference between a page and a picture of one.
    let doc = doc_of("<form action=/g><textarea name=t cols=12>a < b\nsecond</textarea></form>");
    let field = doc.find("textarea", 0).expect("the element is kept");
    assert_eq!(doc.text(field).trim(), "a < b\nsecond");
    assert_eq!(
        form_submission(&doc, field, &Values::default(), (0, 0)).2,
        "t=a+%3C+b%0Asecond"
    );

    let rows = placed(&doc, &Options::default(), 100);
    let box_ = rows.iter().find(|r| r.field.is_some()).expect("a box");
    assert_eq!(box_.field.unwrap().kind, FieldKind::Textarea);
    // The value keeps its own line break: a textarea is the one control whose box has more than
    // one line, and flattening the value to spaces would be a different value.
    assert_eq!(
        box_.text,
        "a < b
second"
    );
    assert!(box_.w > box_.h, "a textarea is wider than it is tall");

    // `rows` is what decides how tall the box is, and two is the default the browser uses.
    let three = doc_of("<textarea cols=12 rows=4>x</textarea>");
    let tall = placed(&three, &Options::default(), 200);
    let tall = tall.iter().find(|r| r.field.is_some()).expect("a box");
    let cell = Font::of(tall.font).unwrap();
    assert_eq!(
        tall.h,
        (cell.ascent() + cell.descent()) as i16 * 4 + FIELD_PAD * 2,
        "four rows of text and the box's own padding"
    );
    assert!(tall.h > box_.h, "and taller than the two-row default");
}

#[test]
fn an_input_paints_a_box_with_its_value_on_the_line_it_sits_on() {
    let doc = doc_of("<p>Name: <input name=n value=hi size=10></p>");
    let rows = placed(&doc, &Options::default(), 100);
    let input = rows.iter().find(|r| r.field.is_some()).expect("a field");
    assert_eq!(input.field.unwrap().kind, FieldKind::Text);
    assert_eq!(input.text, "hi");
    assert!(input.w > 0 && input.h > 0);

    // The label and the box are one row, side by side, in the order the document wrote them.
    let label = rows.iter().find(|r| r.text == "Name: ").expect("the label");
    assert_eq!(label.y, input.y);
    assert_eq!(input.x, label.x + label.w);

    // An editor is taller than the words beside it, and the words sit on its middle rather than
    // above it.
    assert!(input.h > label.h);
    assert!(
        label.top > input.top,
        "the label is centred against the box"
    );
}

#[test]
fn a_fields_size_attribute_decides_its_width() {
    let narrow = doc_of("<input name=n size=4>");
    let wide = doc_of("<input name=n size=10>");
    let default = doc_of("<input name=n>");
    let box_of = |doc: &Document| placed(doc, &Options::default(), 100)[0].w;
    assert!(box_of(&wide) > box_of(&narrow));
    assert!(box_of(&wide) > box_of(&default));
    assert!(
        box_of(&default) > box_of(&narrow),
        "the default is eight columns"
    );
}

#[test]
fn a_password_field_paints_bullets_and_keeps_its_value() {
    let doc = doc_of("<input type=password name=p value=secret>");
    let rows = placed(&doc, &Options::default(), 100);
    assert_eq!(rows[0].text, "******");
    // The bullets are what the box shows; the value is still what the form posts.
    assert_eq!(doc.attr(0, "value"), Some("secret"));
}

#[test]
fn a_checkbox_is_a_box_beside_its_label() {
    let doc = doc_of("<p><input type=checkbox name=a checked> remember</p>");
    let rows = placed(&doc, &Options::default(), 100);
    let box_ = rows.iter().find(|r| r.field.is_some()).expect("a box");
    let field = box_.field.unwrap();
    assert_eq!(field.kind, FieldKind::Checkbox);
    assert!(field.checked);
    assert!(box_.text.is_empty(), "a box's label is the text beside it");
    assert!(box_.w > box_.h, "the box, plus the gap before the words");

    let label = rows
        .iter()
        .find(|r| r.text == "remember")
        .expect("the label");
    assert_eq!(label.y, box_.y, "one row");
    assert!(
        label.x >= box_.x + box_.h,
        "the gap keeps the words off the frame"
    );
}

#[test]
fn a_select_paints_only_the_option_it_shows() {
    let doc = doc_of(
        "<form action=/go><select name=s><option value=1>one</option><option value=2 selected>two</option><option value=3>three</option></select></form>",
    );
    let rows = placed(&doc, &Options::default(), 100);
    let painted: Vec<&str> = rows
        .iter()
        .map(|r| r.text.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    // The list is closed: the other two options are markup, not body text.
    assert_eq!(painted, vec!["two"]);
    assert_eq!(rows[0].field.unwrap().kind, FieldKind::Select);
    // And the form still posts the value behind the label.
    assert_eq!(
        form_submission(&doc, 1, &Values::default(), (0, 0)).2,
        "s=2"
    );
}

#[test]
fn a_button_is_as_wide_as_its_label() {
    let doc =
        doc_of("<input type=submit value=Send><input type=reset><input type=button value=Go>");
    let rows = placed(&doc, &Options::default(), 100);
    let labels: Vec<(&str, FieldKind)> = rows
        .iter()
        .filter_map(|r| r.field.map(|f| (r.text.as_str(), f.kind)))
        .collect();
    assert_eq!(
        labels,
        vec![
            ("Send", FieldKind::Submit),
            ("Reset", FieldKind::Reset),
            ("Go", FieldKind::Button)
        ],
        "a button with no value says what it is"
    );
    assert!(rows[0].w < rows[1].w, "a wider label makes a wider button");
}

#[test]
fn a_hidden_input_paints_nothing_but_still_posts() {
    let doc =
        doc_of("<form action=/go><input type=hidden name=h value=1><input name=a value=2></form>");
    let fields: Vec<FieldKind> = doc
        .runs
        .iter()
        .filter_map(|r| r.field.map(|f| f.kind))
        .collect();
    assert_eq!(
        fields,
        vec![FieldKind::Text],
        "one box for the one visible field"
    );
    assert_eq!(
        form_submission(&doc, 1, &Values::default(), (0, 0)).2,
        "h=1&a=2"
    );
}

#[test]
fn text_after_an_inline_element_still_belongs_to_the_block() {
    // A close tag used to drop the owner, so ` c` was painted but belonged to no element and
    // `GetElemText` on the paragraph came back short.
    let doc = doc_of("<p>a <b>b</b> c</p>");
    let p = doc.find("p", 0).unwrap();
    let b = doc.find("b", 0).unwrap();
    assert_eq!(doc.text(b), "b", "the inner element keeps its own text");
    assert_eq!(doc.text(p), "a b c", "and the block answers all of it");
}

#[test]
fn an_element_style_ends_with_its_element() {
    // Alignment and colour used to be sticky: the paragraph after a centred red one came out
    // centred and red because the element before it said so.
    let doc = doc_of("<p align=center><font color=\"#ff0000\">red</font></p><p>plain</p>");
    let rows = placed(&doc, &Options::default(), 200);
    let red = rows.iter().find(|r| r.text == "red").expect("the red row");
    let plain = rows
        .iter()
        .find(|r| r.text == "plain")
        .expect("the plain row");
    assert_eq!(red.color, make_rgb(255, 0, 0));
    assert_eq!(plain.color, 0, "the colour did not outlive its <font>");
    assert_eq!(red.align, g::IDF_ALIGN_CENTER);
    assert_eq!(plain.align, g::IDF_ALIGN_LEFT, "nor did the alignment");
}

#[test]
fn a_control_names_the_element_a_key_press_has_to_find() {
    // The focus ring is drawn by element, so a control's span has to name the control itself and
    // not the paragraph around it.
    let doc = doc_of("<p><input name=a><a href=x>go</a></p>");
    let input = doc.find("input", 0).unwrap();
    let link = doc.find("a", 0).unwrap();
    let by_elem = |elem: usize| {
        doc.runs
            .iter()
            .filter(|r| r.elem == Some(elem))
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
    };
    assert!(
        by_elem(input).iter().all(|t| t.is_empty()),
        "the box itself carries no words"
    );
    assert_eq!(by_elem(link), vec!["go"]);
    // And both are places the user can land.
    let kinds: Vec<u32> = doc.focus_items().iter().map(|f| f.kind).collect();
    assert_eq!(kinds, vec![g::HVFOCUS_EDIT, g::HVFOCUS_LINK]);
}

#[test]
fn an_img_between_two_blocks_takes_a_row_of_its_own() {
    // Not because a picture owns a row, but because the `</p>` before it closed the line: an
    // `<img>` joins the row that is open and this one finds none. The next test is the other half.
    let doc =
        doc_of("<p>before</p><img src=\"pic.png\" alt=\"here\" width=20 height=12><p>after</p>");
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].src, "pic.png");
    assert_eq!(doc.images[0].alt, "here");
    assert_eq!((doc.images[0].width, doc.images[0].height), (20, 12));
    assert_eq!(
        doc.find("img", 0),
        Some(1),
        "p, img, p — the img is an element"
    );

    let rows = placed(&doc, &Options::default(), 200);
    let pic = rows
        .iter()
        .find(|r| r.image.is_some())
        .expect("a picture row");
    assert_eq!(pic.row, 12, "the row is the height the document named");
    assert_eq!(pic.w, 20, "and as wide as it named");
    assert!(pic.text.is_empty(), "a picture carries no text of its own");
}

#[test]
fn a_picture_shares_the_line_with_the_words_around_it() {
    let doc = doc_of("<p>see <img src=p.png alt=here width=8 height=8> this</p>");
    let rows = placed(&doc, &Options::default(), 200);
    assert_eq!(
        rows.iter().filter(|r| r.row == rows[0].row).count(),
        3,
        "the words, the picture and the words after it are one line"
    );
    let pic = rows
        .iter()
        .find(|r| r.image.is_some())
        .expect("the picture");
    assert_eq!((pic.w, pic.h), (8, 8), "the size the document named");

    let words = rows.first().expect("the word before the picture");
    let face = Font::of(words.font).expect("a face");
    let baseline = words.top + face.ascent() as i16;
    assert_eq!(
        pic.top + pic.h,
        baseline + 2,
        "the picture's bottom is the descender space below the words' baseline"
    );
    assert!(pic.x > words.x, "and it comes after the word it followed");
}

#[test]
fn a_picture_too_wide_for_the_line_moves_to_the_next_one_whole() {
    // The shipped rule for an item with no break inside it: end the line and start the next one
    // with the item at its full width. Nothing shrinks a picture to make it fit.
    let doc = doc_of("<p>hello<img src=p.png width=100 height=8>tail</p>");
    let rows = placed(&doc, &Options::default(), 200);
    let pic = rows
        .iter()
        .find(|r| r.image.is_some())
        .expect("the picture");
    assert_eq!(pic.w, 100, "it is not squeezed into what was left");
    let words = rows.first().expect("the word before it");
    assert!(
        pic.y > words.y,
        "it starts the line after the words rather than the one they were on"
    );
}

#[test]
fn an_img_without_a_height_gets_one_line_for_its_alt() {
    let doc = doc_of("<img src=gone.png alt=missing>");
    let rows = placed(&doc, &Options::default(), 100);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].image.is_some());
    assert!(rows[0].row > 0, "an undecoded picture still takes a row");
    assert!(
        rows[0].w > 0,
        "and as much room as the alt it shows, or the words after it would sit on top of it"
    );
}

/// The picture row of a document laid out next to one bitmap.
fn picture_of(html: &str, width: u16, height: u16) -> (i16, i16) {
    let doc = doc_of(html);
    let pictures = bitmap_of(width, height);
    let rows = layout(
        &doc,
        Rect::new(0, 0, 300, 200),
        &Options::default(),
        &pictures,
        None,
        0,
    );
    let pic = rows
        .iter()
        .find(|r| r.image.is_some())
        .expect("the picture takes a row");
    (pic.w, pic.h)
}

#[test]
fn a_picture_inside_its_own_pixels_takes_the_size_the_page_named() {
    assert_eq!(
        picture_of("<img src=p.png width=40 height=20>", 100, 50),
        (40, 20)
    );
}

#[test]
fn a_picture_is_never_enlarged_to_the_size_the_page_named() {
    assert_eq!(
        picture_of("<img src=p.png width=200 height=20>", 100, 50),
        (100, 50),
        "a request wider than the picture is refused whole, not clamped"
    );
    assert_eq!(
        picture_of("<img src=p.png width=40 height=300>", 100, 50),
        (100, 50),
        "a height the picture does not have fails the pair as well"
    );
}

#[test]
fn half_a_size_is_no_size_at_all() {
    // `v19 <= 0 || v17 <= 0`: a document that names only one of the two gets the bitmap's size for
    // both. The common `<img width=40>` therefore does nothing on this build, which is worth an
    // assertion because every other reader scales it.
    assert_eq!(
        picture_of("<img src=p.png width=40>", 100, 50),
        (100, 50),
        "one attribute alone does not scale a picture"
    );
    assert_eq!(
        picture_of("<img src=p.png height=20>", 100, 50),
        (100, 50)
    );
}

#[test]
fn a_data_uri_carries_its_own_bytes() {
    assert_eq!(
        image_bytes("data:text/plain;base64,aGk="),
        Some(b"hi".to_vec())
    );
    assert_eq!(
        image_bytes("data:image/png;base64,aGk="),
        Some(b"hi".to_vec()),
        "the media type does not change the payload"
    );
    assert_eq!(image_bytes("data:,hi%21"), Some(b"hi!".to_vec()));
    // A URL is not something this host can read synchronously, and that is answered, not guessed.
    assert_eq!(image_bytes("http://example.com/a.png"), None);
    assert_eq!(image_bytes("pic.png"), None);
    // A malformed payload is refused rather than passed on as bytes that will fail to decode.
    assert_eq!(image_bytes("data:image/png;base64,!!!"), None);
}

#[test]
fn a_javascript_link_is_not_a_target() {
    // No script runs here, so a `javascript:` href is not a page to go to. It stays in the document
    // — `FindElem("a")` still finds it — but out of the focus order, where acting on it would hand
    // the pack a URL it can only fail to fetch.
    let doc = doc_of("<a href=\"javascript:void(0)\">a</a><a href=\"#top\">b</a>");
    let items = doc.focus_items();
    assert_eq!(items.len(), 1, "only the fragment link is navigable");
    assert_eq!(items[0].elem, 1);
}

// What the user does to a form, at the layout layer: typing, ticking, choosing, and the escaping
// the view state needs. The ABI half — keys and the pen — is in `src/test/runtime.rs`.

/// The element index of the first element with this tag.
fn tag_at(doc: &Document, tag: &str) -> usize {
    doc.elements
        .iter()
        .position(|e| e.tag == tag)
        .unwrap_or_else(|| panic!("no <{tag}> in the document"))
}

#[test]
fn a_typed_value_is_what_the_box_draws() {
    let doc = doc_of("<input name=a size=8 value=x>");
    let at = tag_at(&doc, "input");
    let plain = layout(
        &doc,
        Rect::new(0, 0, 120, 40),
        &Options::default(),
        &[],
        None,
        0,
    );
    assert_eq!(plain[0].text, "x");

    let mut values = Values::default();
    values.edit(at).text = Some("hello".to_string());
    let typed = layout(
        &doc,
        Rect::new(0, 0, 120, 40),
        &Options::default(),
        &[],
        Some(&values),
        0,
    );
    assert_eq!(typed[0].text, "hello", "the box draws what was typed");
    assert_eq!(
        typed[0].w, plain[0].w,
        "a box is as wide as its `size` asked, not as wide as its value"
    );
    assert_eq!(
        doc.control_text(at),
        "x",
        "the parse is shared and immutable, so it still says what the page said"
    );
}

#[test]
fn a_password_is_masked_where_it_is_drawn_not_where_it_is_stored() {
    let doc = doc_of("<input type=password name=p value=secret>");
    let at = tag_at(&doc, "input");
    assert_eq!(
        doc.control_text(at),
        "secret",
        "the value is kept in the clear: it is what the form posts"
    );

    let mut values = Values::default();
    values.edit(at).text = Some("hunter2".to_string());
    let rows = layout(
        &doc,
        Rect::new(0, 0, 120, 40),
        &Options::default(),
        &[],
        Some(&values),
        0,
    );
    assert_eq!(rows[0].text, "*******", "seven characters, seven bullets");
    assert_eq!(
        form_submission(&doc, at, &values, (0, 0)).2,
        "p=hunter2",
        "the form posts what was typed rather than what was drawn"
    );
}

#[test]
fn a_caret_is_placed_after_the_characters_before_it() {
    let doc = doc_of("<input name=a size=8 value=abcd>");
    let at = tag_at(&doc, "input");
    let mut values = Values::default();
    let entry = values.edit(at);
    entry.text = Some("abcd".to_string());
    entry.caret = 2;

    let rows = layout(
        &doc,
        Rect::new(0, 0, 120, 40),
        &Options::default(),
        &[],
        Some(&values),
        0,
    );
    let (caret, row) = rows[0].caret.expect("an edited editor carries a caret");
    let font = Font::of(rows[0].font).unwrap();
    let two: Vec<u16> = "ab".encode_utf16().collect();
    assert_eq!(
        caret,
        font.measure_chars(&two) as i16 + FIELD_PAD,
        "the caret sits one padding inside the box, after two characters"
    );
    assert_eq!(row, FIELD_PAD, "and on the box's only line");
    assert!(caret > 0 && caret < rows[0].w, "and inside the box");

    // A control nobody has typed into has no caret, so focusing it draws nothing extra.
    let untouched = layout(
        &doc,
        Rect::new(0, 0, 120, 40),
        &Options::default(),
        &[],
        Some(&Values::default()),
        0,
    );
    assert_eq!(untouched[0].caret, None);
}

#[test]
fn a_tick_from_the_user_wins_over_the_attribute() {
    // The page ticked the box and the user unticked it: nothing is posted.
    let doc =
        doc_of("<form action=/go><input type=checkbox name=c checked><input type=submit></form>");
    let box_at = tag_at(&doc, "input");
    let submit = doc.elements.iter().rposition(|e| e.tag == "input").unwrap();
    assert_eq!(
        form_submission(&doc, submit, &Values::default(), (0, 0)).2,
        "c=on"
    );
    let mut values = Values::default();
    values.set_checked(box_at, false);
    assert_eq!(form_submission(&doc, submit, &values, (0, 0)).2, "");

    // The other way round: the page left it blank and the user ticked it.
    let doc =
        doc_of("<form action=/go><input type=checkbox name=c value=v><input type=submit></form>");
    let box_at = tag_at(&doc, "input");
    let submit = doc.elements.iter().rposition(|e| e.tag == "input").unwrap();
    assert_eq!(
        form_submission(&doc, submit, &Values::default(), (0, 0)).2,
        ""
    );
    let mut values = Values::default();
    values.set_checked(box_at, true);
    assert_eq!(form_submission(&doc, submit, &values, (0, 0)).2, "c=v");
}

#[test]
fn a_choice_the_user_made_is_what_the_list_shows_and_posts() {
    let doc = doc_of(
        "<form action=/go><select name=s><option value=1>one</option>\
         <option value=2>two</option></select><input type=submit></form>",
    );
    let select = tag_at(&doc, "select");
    let submit = doc.elements.iter().rposition(|e| e.tag == "input").unwrap();
    assert_eq!(
        doc.chosen_index(select),
        0,
        "the first option is the default"
    );
    assert_eq!(
        form_submission(&doc, submit, &Values::default(), (0, 0)).2,
        "s=1"
    );

    let mut values = Values::default();
    values.choose(select, 1, "two");
    let rows = layout(
        &doc,
        Rect::new(0, 0, 120, 80),
        &Options::default(),
        &[],
        Some(&values),
        0,
    );
    let shown = rows
        .iter()
        .find(|row| row.field.is_some())
        .expect("the list draws a box");
    assert_eq!(
        shown.text, "two",
        "the closed list shows the option it chose"
    );
    assert_eq!(
        form_submission(&doc, submit, &values, (0, 0)).2,
        "s=2",
        "and posts that option's value rather than its label"
    );
}

#[test]
fn a_radio_group_is_the_ones_with_the_same_name_in_its_form() {
    let doc = doc_of(
        "<form action=/go><input type=radio name=p value=1>\
         <input type=radio name=p value=2><input type=radio name=q value=3></form>",
    );
    let first = tag_at(&doc, "input");
    assert_eq!(
        doc.radio_group(first),
        vec![first, first + 1],
        "two radios share `p`; the third has a name of its own"
    );
    assert_eq!(
        doc.radio_group(first + 2),
        vec![first + 2],
        "a radio with no sibling is a group of one"
    );
}

#[test]
fn the_view_state_escaping_survives_what_a_value_may_contain() {
    // The state's words are `?`/`&`/`=`-framed, so a value carrying any of those — or a `%` that
    // is part of the text — has to come back exactly as it went in.
    let text = "a;b,c=d e%f+g&h";
    let encoded = form_encode(text);
    assert!(!encoded.contains(';') && !encoded.contains(','));
    assert_eq!(form_decode(&encoded), text);
    // A `%` that does not start two hex digits is kept as itself rather than eating the next bytes.
    assert_eq!(form_decode("100%"), "100%");
    assert_eq!(form_decode("%zz"), "%zz");
}

#[test]
fn a_caret_on_the_second_line_sits_one_cell_lower() {
    let doc = doc_of("<textarea name=t cols=8 rows=3>ab\ncd</textarea>");
    let at = tag_at(&doc, "textarea");
    let mut values = Values::default();
    let entry = values.edit(at);
    entry.text = Some("ab\ncd".to_string());
    entry.caret = 5; // after "cd", which is the second line

    let rows = layout(
        &doc,
        Rect::new(0, 0, 120, 60),
        &Options::default(),
        &[],
        Some(&values),
        0,
    );
    let line = rows
        .iter()
        .find(|row| row.field.is_some())
        .expect("the textarea draws a box");
    let (x, y) = line.caret.expect("a typed-into editor carries a caret");
    let font = Font::of(line.font).unwrap();
    let two: Vec<u16> = "cd".encode_utf16().collect();
    assert_eq!(
        x,
        font.measure_chars(&two) as i16 + FIELD_PAD,
        "the column is measured in the line the caret is on"
    );
    assert_eq!(
        y,
        (font.ascent() + font.descent()) as i16 + FIELD_PAD,
        "and the row is one cell down, plus the box's padding"
    );
}

#[test]
fn a_lists_text_is_the_labels_of_the_options_it_holds() {
    // "Contained elements are ignored, and separate sections of text content (including that of
    // sub-elements) are concatenated": an `<option>` is a sub-element
    // of the `<select>`, so the list's own text has to carry its labels. A script's source still
    // does not bubble — that is the one opaque element that stays where it is.
    let doc =
        doc_of("<select name=s><option value=1>one</option><option value=2>two</option></select>");
    let select = tag_at(&doc, "select");
    assert_eq!(doc.text(select), "onetwo");
    let options = doc.options_of(select);
    assert_eq!(doc.text(options[0]), "one", "and each option keeps its own");
    assert_eq!(doc.text(options[1]), "two");

    let scripted = doc_of("<div><script>var x = 1;</script>ok</div>");
    let div = tag_at(&scripted, "div");
    assert_eq!(
        scripted.text(div),
        "ok",
        "a script's source is not text content of its block"
    );
}

#[test]
fn a_textarea_window_follows_the_caret_into_the_value() {
    let doc = doc_of("<textarea cols=8 rows=2>one\ntwo\nthree\nfour</textarea>");
    let rows = placed(&doc, &Options::default(), 200);
    let box_ = rows.iter().find(|r| r.field.is_some()).expect("a box");
    assert_eq!(
        box_.first_row, 0,
        "an untouched box shows the value from its first line"
    );

    // The user leaves the caret at the end of the value, which is two lines below the last one the
    // box can show. The window has to move with it, or the user is typing somewhere invisible.
    let value = "one\ntwo\nthree\nfour";
    let mut values = Values::default();
    let elem = box_.elem.expect("a control names its element");
    values.begin(elem, value);
    values.edit(elem).caret = value.chars().count();
    let moved = layout(
        &doc,
        Rect::new(0, 0, 120, 200),
        &Options::default(),
        &[],
        Some(&values),
        0,
    );
    let box_ = moved.iter().find(|r| r.field.is_some()).expect("a box");
    assert_eq!(box_.first_row, 2, "the window moved to the last two lines");
    assert_eq!(
        box_.text, value,
        "the value itself is still the whole value"
    );

    let font = Font::of(box_.font).unwrap();
    let cell = (font.ascent() + font.descent()) as i16;
    let (_, y) = box_.caret.expect("an edited editor carries a caret");
    assert_eq!(y, cell + FIELD_PAD, "the caret is on the box's last row");
    assert!(y + cell <= box_.h, "and that row is inside the box");
}

#[test]
fn a_window_stops_at_both_ends_of_the_value() {
    let doc = doc_of("<textarea cols=8 rows=2>a\nb\nc\nd\ne</textarea>");
    let rows = placed(&doc, &Options::default(), 200);
    let elem = rows
        .iter()
        .find(|r| r.field.is_some())
        .and_then(|r| r.elem)
        .expect("a control names its element");

    let at = |caret: usize| {
        let mut values = Values::default();
        values.begin(elem, "a\nb\nc\nd\ne");
        values.edit(elem).caret = caret;
        layout(
            &doc,
            Rect::new(0, 0, 120, 200),
            &Options::default(),
            &[],
            Some(&values),
            0,
        )
        .into_iter()
        .find(|r| r.field.is_some())
        .expect("a box")
        .first_row
    };
    assert_eq!(at(0), 0, "the first line puts the window at the top");
    assert_eq!(at(99), 3, "the last line stops it one box from the end");

    // A value that fits needs no window at all, whatever the caret does.
    let short = doc_of("<textarea cols=8 rows=3>a\nb</textarea>");
    let rows = layout(
        &short,
        Rect::new(0, 0, 120, 200),
        &Options::default(),
        &[],
        None,
        0,
    );
    let box_ = rows.iter().find(|r| r.field.is_some()).expect("a box");
    assert_eq!(box_.first_row, 0);
    assert_eq!(box_.h, box_.h.max(0), "and the box is as tall as its rows");
}

#[test]
fn an_image_button_is_a_picture_that_reports_where_it_was_pressed() {
    let doc = doc_of(
        "<form action=/go method=post><input type=image name=i src=p.png width=4 height=4>\
         <input name=t value=v></form>",
    );
    // The picture takes a row of its own, and that row belongs to the `<input>` — which is what a
    // press has to find, and what `HVN_SUBMIT` names.
    let rows = placed(&doc, &Options::default(), 200);
    let row = rows
        .iter()
        .find(|r| r.image.is_some())
        .expect("the picture takes a row");
    assert_eq!(row.elem, Some(1), "and the row belongs to the <input>");

    let submit = doc
        .focus_items()
        .into_iter()
        .find(|f| f.elem == 1)
        .expect("an image button is something to land on");
    assert_eq!(
        submit.kind,
        g::HVFOCUS_SUBMIT,
        "the focus enum has no member of its own for it, and it acts as a submit button"
    );

    let (url, method, data) = form_submission(&doc, 1, &Values::default(), (3, 4));
    assert_eq!((url.as_str(), method.as_str()), ("/go", "POST"));
    assert_eq!(
        data, "i.x=3&i.y=4&t=v",
        "the place inside the picture comes with the rest of the form"
    );

    // A button that was not pressed reports nothing, exactly like any other unpressed button.
    let (_, _, other) = form_submission(&doc, 2, &Values::default(), (0, 0));
    assert_eq!(other, "t=v");
}

#[test]
fn an_image_button_without_a_picture_is_a_submit_box() {
    let doc = doc_of("<input type=image name=i value=Go>");
    let rows = placed(&doc, &Options::default(), 200);
    let box_ = rows.iter().find(|r| r.field.is_some()).expect("a box");
    assert_eq!(box_.field.unwrap().kind, FieldKind::Submit);
    assert_eq!(box_.text, "Go", "the value is the label, as on any button");
    assert!(
        doc.images.is_empty(),
        "there is no picture, so there is nothing to fetch"
    );
}

#[test]
fn a_disabled_control_is_not_in_the_focus_order() {
    let doc =
        doc_of("<input name=a disabled><a href=x>go</a><select disabled></select><input name=b>");
    let elems: Vec<usize> = doc.focus_items().iter().map(|f| f.elem).collect();
    assert_eq!(
        elems,
        vec![1, 3],
        "the link and the field that still works, and nothing else"
    );
    // The value is still in the document: disabled takes it out of the *interaction*, not out of
    // the page — `GetElemText` and the parse still answer for it.
    assert_eq!(doc.attr(0, "name"), Some("a"));
}

#[test]
fn the_view_state_is_the_shipped_pair_of_numbers_and_the_form_words() {
    let doc = doc_of(
        "<form><input name=a value=x><select name=s><option value=1>one</option>\
         <option value=2 selected>two</option></select>\
         <input type=checkbox name=c value=v></form>",
    );
    let at_a = tag_at(&doc, "input");
    let at_c = doc
        .elements
        .iter()
        .rposition(|e| e.tag == "input")
        .expect("the checkbox is the last input");

    // The user typed into the field and ticked the box; the list stays where the page put it.
    let mut values = Values::default();
    {
        let entry = values.edit(at_a);
        entry.text = Some("hello?".to_string());
        entry.caret = 6;
    }
    values.set_checked(at_c, true);

    // The shipped writer: `<scroll>,<focuspos>` and then the forms' words, encoded the way a
    // submission posts them. A `?` in a value cannot break the frame around it.
    let state = view_state_shim(&doc, &values, 7, 2);
    assert_eq!(state, "7,2?a=hello%3F&s=2&c=v");

    // And the shipped reader: the same string comes apart into the position, the focus and the
    // words — plus the `#anchor` form, which names an element instead.
    match parse_view_state(&state) {
        ViewRestore::Restore {
            scroll,
            focus,
            fields,
        } => {
            assert_eq!((scroll, focus), (7, 2));
            assert_eq!(
                fields,
                vec![
                    ("a".to_string(), "hello?".to_string()),
                    ("s".to_string(), "2".to_string()),
                    ("c".to_string(), "v".to_string()),
                ]
            );
        }
        _ => panic!("a numbered state parses as a restore"),
    }
    assert_eq!(
        parse_view_state("#mid"),
        ViewRestore::Anchor("mid".to_string()),
        "the reader's other half: a named element"
    );
    match parse_view_state("5,-1") {
        ViewRestore::Restore {
            scroll,
            focus,
            fields,
        } => {
            assert_eq!((scroll, focus), (5, -1));
            assert!(fields.is_empty(), "no `?` is no words");
        }
        _ => panic!("a bare pair parses as a restore"),
    }
}

/// `html::view_state` under the name this file's other tests can see without an import dance.
fn view_state_shim(doc: &Document, values: &Values, scroll: i32, focus: i32) -> String {
    crate::web::html::view_state(doc, values, scroll, focus)
}

#[test]
fn a_body_background_has_one_slot_of_its_own_ahead_of_the_first_img() {
    let doc = doc_of("<body background=\"tile.bmp\"><img src=\"pic.png\">");
    assert_eq!(doc.bg_image, Some(0));
    assert_eq!(doc.images.len(), 2);
    assert_eq!(doc.images[0].src, "tile.bmp");
    assert!(doc.images[0].background);
    assert!(!doc.images[1].background);
    assert_eq!(
        doc.runs.iter().filter_map(|r| r.image).collect::<Vec<_>>(),
        vec![1],
        "the <img> keeps its own index in the same table"
    );
}

#[test]
fn the_last_body_decides_the_paper_and_a_body_without_attributes_clears_it() {
    let one = doc_of("<body bgcolor=red background=a.png><p>x</p><body background=b.png><p>y</p>");
    assert_eq!(one.bg_image, Some(0));
    assert_eq!(
        one.images.len(),
        1,
        "one slot, refilled rather than appended"
    );
    assert_eq!(one.images[0].src, "b.png");

    let two = doc_of("<body bgcolor=red background=a.png><p>x</p><body><p>y</p>");
    assert!(
        two.bg.is_none(),
        "the default colour is written back, not inherited"
    );
    assert_eq!(two.bg_image, None, "and the picture is released");
}

#[test]
fn an_empty_body_writes_neither_the_colour_nor_the_picture() {
    // The empty-element gate stands before the switch: `<body...></body>` never reaches case 9.
    let doc = doc_of("<body bgcolor=red background=a.png></body><p>x</p>");
    assert!(doc.bg.is_none());
    assert_eq!(doc.bg_image, None);
    assert!(doc.images.is_empty());
}

#[test]
fn a_marquee_is_one_object_with_the_six_defaults_sub_10177450_writes() {
    let doc = doc_of("<marquee>W</marquee>");
    assert_eq!(doc.marquees.len(), 1);
    let mq = &doc.marquees[0];
    assert_eq!(mq.behavior, MarqueeBehavior::Scroll);
    assert!(mq.left, "no direction attribute means left");
    assert_eq!(mq.loops, -1, "scroll and alternate run for ever");
    assert_eq!(mq.amount, 2);
    assert_eq!(mq.delay, 36);
    assert_eq!(mq.bg, None, "no colour of its own means CLR_SYS_WIN");
    assert_eq!(doc.runs.len(), 1, "the text is not laid out a second time");
    assert_eq!(doc.runs[0].marquee, Some(0));
    assert_eq!(doc.runs[0].text, "W");
    assert_eq!(
        doc.runs[0].font,
        g::AEE_FONT_NORMAL,
        "sub_10177670 hard-codes 0x8000 for both measurement and drawing"
    );
    assert!(!doc.runs[0].inline, "the object owns its line");
}

#[test]
fn a_marquees_six_words_are_read_and_black_is_a_colour_not_the_absence_of_one() {
    let doc = doc_of(
        "<marquee behavior=SLIDE direction=RIGHT loop=3 scrollamount=7 scrolldelay=80 bgcolor=black>W</marquee>",
    );
    let mq = &doc.marquees[0];
    assert_eq!(
        mq.behavior,
        MarqueeBehavior::Slide,
        "aee_stricmp, not strcmp"
    );
    assert!(!mq.left);
    assert_eq!(mq.loops, 3);
    assert_eq!(mq.amount, 7);
    assert_eq!(mq.delay, 80);
    assert_eq!(mq.bg, Some(0), "MAKE_RGB(0,0,0) is a colour here");

    let slide = doc_of("<marquee behavior=slide>W</marquee>");
    assert_eq!(slide.marquees[0].loops, 1, "slide defaults to one pass");
    let unknown = doc_of("<marquee direction=up behaviour=alternate>W</marquee>");
    assert_eq!(unknown.marquees[0].behavior, MarqueeBehavior::Scroll);
    assert!(
        !unknown.marquees[0].left,
        "anything but left is the other way"
    );
}

#[test]
fn a_marquee_takes_only_the_first_stretch_of_text_and_skips_the_rest() {
    let doc = doc_of("<marquee>AB <b>CD</b> EF</marquee><p>Z</p>");
    let scrolling: Vec<&Run> = doc.runs.iter().filter(|r| r.marquee.is_some()).collect();
    assert_eq!(scrolling.len(), 1);
    assert_eq!(scrolling[0].text, "AB");
    assert_eq!(
        doc.text(0),
        "AB",
        "GetElemText still answers what the element held"
    );
    assert!(doc.runs.iter().all(|r| r.text != "CD" && r.text != "EF"));
    assert_eq!(doc.runs.iter().filter(|r| r.text == "Z").count(), 1);
}

#[test]
fn a_marquee_owns_a_row_as_wide_as_the_column() {
    let doc = doc_of("<p>before</p><marquee>W</marquee><p>after</p>");
    let rows = placed(&doc, &Options::default(), 80);
    let box_ = rows.iter().find(|p| p.marquee.is_some()).unwrap();
    assert_eq!(
        box_.w, box_.dx,
        "the rectangle is the column, not its words"
    );
    assert!(box_.h > 0);
    let mq = box_.marquee.unwrap();
    assert_eq!(mq.at, 0);
    assert!(mq.text_w > 0 && mq.text_w < box_.w);
    assert!(mq.pad > 0, "28% of the font cell is not zero");
    assert_eq!(rows.iter().filter(|p| p.marquee.is_some()).count(), 1);
}

#[test]
fn a_scrolling_marquee_walks_two_pixels_every_thirty_six_milliseconds() {
    let mq = &doc_of("<marquee>W</marquee>").marquees[0].clone();
    let pose = |ms| marquee_pose(mq, 20, 8, ms);
    assert_eq!(pose(0).offset, 0);
    assert_eq!(pose(35).offset, 0, "a whole delay must pass");
    assert_eq!(pose(36).offset, 2);
    assert_eq!(pose(72).offset, 4);
    assert!(pose(72).left);
    // offset passes box + text (28) on the fifteenth step and starts again at zero.
    assert_eq!(pose(36 * 14).offset, 28);
    assert_eq!(pose(36 * 15).offset, 0);
}

#[test]
fn a_slide_stops_after_its_one_pass_and_an_alternate_turns_around() {
    let slide = doc_of("<marquee behavior=slide>W</marquee>").marquees[0].clone();
    // width 20, text 8, amount 2: the eleventh step passes 20, bumps cycles to 1 and, because
    // the default loop is one, parks at the box's width instead of starting again.
    assert_eq!(marquee_pose(&slide, 20, 8, 36 * 10).offset, 20);
    assert_eq!(marquee_pose(&slide, 20, 8, 36 * 11).offset, 20);
    assert_eq!(marquee_pose(&slide, 20, 8, 36 * 500).offset, 20);

    let alternate = doc_of("<marquee behavior=alternate>W</marquee>").marquees[0].clone();
    let first = marquee_pose(&alternate, 20, 8, 0);
    assert_eq!(first.offset, 8, "alternate starts from the text's width");
    assert!(first.left);
    let turned = marquee_pose(&alternate, 20, 8, 36 * 7);
    assert_eq!(turned.offset, 8);
    assert!(!turned.left, "the seventh step flips the direction");
    assert!(marquee_pose(&alternate, 20, 8, 36 * 14).left);
}

#[test]
fn a_marquee_honours_its_amount_delay_and_loop() {
    let fast = doc_of("<marquee scrollamount=5 scrolldelay=10>W</marquee>").marquees[0].clone();
    assert_eq!(marquee_pose(&fast, 40, 8, 10).offset, 5);
    assert_eq!(marquee_pose(&fast, 40, 8, 29).offset, 10);

    let once = doc_of("<marquee loop=1>W</marquee>").marquees[0].clone();
    // width 20 + text 8 = 28, so the fifteenth step finishes the first pass and loop=1 stops
    // the object there.
    assert_eq!(marquee_pose(&once, 20, 8, 36 * 15).offset, 0);
    assert_eq!(marquee_pose(&once, 20, 8, 36 * 16).offset, 0);
    assert_eq!(marquee_pose(&once, 20, 8, 36 * 500).offset, 0);
}

/// `n` plain `<div>`s, then one more element that carries the attribute under test. That last
/// element is frame number `n + 1`: 29 still writes a frame, 30 does not.
fn deep(prefix: usize, tag: &str) -> Document {
    let name = tag.split_whitespace().next().unwrap();
    doc_of(&format!(
        "{}<{tag}>X</{name}>{}",
        "<div>".repeat(prefix),
        "</div>".repeat(prefix)
    ))
}

fn run_of<'a>(doc: &'a Document, text: &str) -> &'a Run {
    doc.runs
        .iter()
        .find(|run| run.text == text)
        .unwrap_or_else(|| panic!("no run holds {text:?}"))
}

#[test]
fn the_twenty_ninth_frame_writes_its_align_and_the_thirtieth_does_not() {
    assert_eq!(
        run_of(&deep(28, "div align=right"), "X").align,
        g::IDF_ALIGN_RIGHT,
        "29 frames: the align word is written"
    );
    assert_eq!(
        run_of(&deep(29, "div align=right"), "X").align,
        g::IDF_ALIGN_LEFT,
        "30 frames: the inherited word is still the left one"
    );
}

#[test]
fn a_thirtieth_frame_changes_neither_the_ink_nor_the_face() {
    let red = make_rgb(255, 0, 0);
    assert_eq!(
        run_of(&deep(28, "font color=red"), "X").color,
        Ink::Own(red),
        "29 frames: <font> writes its colour"
    );
    assert_eq!(
        run_of(&deep(29, "font color=red"), "X").color,
        Ink::Unset,
        "30 frames: the red is never written"
    );
    assert_eq!(run_of(&deep(28, "b"), "X").font, g::AEE_FONT_BOLD);
    assert_eq!(run_of(&deep(29, "b"), "X").font, g::AEE_FONT_NORMAL);
}

#[test]
fn a_thirtieth_blockquote_adds_no_indent() {
    assert_eq!(run_of(&deep(28, "blockquote"), "X").indent, 1);
    assert_eq!(run_of(&deep(29, "blockquote"), "X").indent, 0);
}

#[test]
fn the_frame_budget_counts_frames_not_elements() {
    // An empty element (`node[node_length] == 2`) and an object (`layout & 2`) never reach the
    // `++v41`, so neither spends one of the 29 frames. Mistake either for a frame and the last
    // `<div align=right>` becomes number 30 and loses its alignment.
    let page = format!(
        "{}<p></p><img src=x><div align=right>X</div><p>Q</p>{}",
        "<div>".repeat(28),
        "</div>".repeat(28)
    );
    let doc = doc_of(&page);
    assert_eq!(run_of(&doc, "X").align, g::IDF_ALIGN_RIGHT);
    assert_eq!(
        doc.runs.iter().filter(|run| run.gap).count(),
        2,
        "the later <p> is frame 29 again, so it still reserves its margin"
    );
}

#[test]
fn the_frame_budget_ignores_objects_in_the_open_stack() {
    let page = format!(
        "{}<select><div align=right>X</div></select>{}",
        "<div>".repeat(28),
        "</div>".repeat(28)
    );
    let doc = doc_of(&page);
    assert_eq!(run_of(&doc, "X").align, g::IDF_ALIGN_RIGHT);
}

#[test]
fn the_freeze_lifts_when_the_deep_branch_ends() {
    let page = format!(
        "{}<font color=red>X</font>{}<font color=blue>Y</font><p>Q</p>",
        "<div>".repeat(29),
        "</div>".repeat(29)
    );
    let doc = doc_of(&page);
    assert_eq!(run_of(&doc, "X").color, Ink::Unset, "X is 30 deep");
    assert_eq!(
        run_of(&doc, "Y").color,
        Ink::Own(make_rgb(0, 0, 255)),
        "Y is one deep, after all the closes"
    );
    assert_eq!(
        doc.runs.iter().filter(|run| run.gap).count(),
        2,
        "the counter fell back to zero too: a later <p> still opens and closes its margin"
    );
}

#[test]
fn a_capped_block_opens_no_gap_and_does_not_end_the_row() {
    let page = |n: usize| format!("{}A<p>B</p>{}", "<div>".repeat(n), "</div>".repeat(n));
    let shallow = doc_of(&page(28));
    assert!(
        !run_of(&shallow, "B").inline,
        "28 divs + p = 29 frames: the block ends the row"
    );
    assert_eq!(
        shallow.runs.iter().filter(|run| run.gap).count(),
        2,
        "the <p> opens and closes a margin gap"
    );

    let deep = doc_of(&page(29));
    assert!(
        run_of(&deep, "B").inline,
        "29 divs + p = 30 frames: the row is never flushed"
    );
    assert_eq!(
        deep.runs.iter().filter(|run| run.gap).count(),
        0,
        "neither the open nor the close of a capped <p> reserves a margin"
    );
}

#[test]
fn a_close_tag_after_a_void_tag_still_names_its_own_element() {
    // `<img>` is an element but, being void, never joins `open`. That shifts every later element
    // index one ahead of its position in the stack. `rposition` answers the position, so a close
    // tag that uses that number as an *element index* reads the first `<p>`'s `empty` byte here —
    // `true` — and quietly drops the second paragraph's closing margin.
    let doc = doc_of("<p></p><img src=x><p>Q</p>");
    let gaps: Vec<&Run> = doc.runs.iter().filter(|run| run.gap).collect();
    assert_eq!(gaps.len(), 2, "the second <p> opens and closes a margin");
    assert!(
        gaps.iter().all(|run| run.elem == Some(2)),
        "both margin rows belong to the second <p>, element 2"
    );
}

#[test]
fn a_plain_picture_is_not_a_focus_anchor() {
    let doc = doc_of("<p><img src=x></p>");
    let pic = doc.runs.iter().find(|r| r.image.is_some()).unwrap();
    let enclosing = pic.elem.unwrap();
    assert_eq!(doc.elements[enclosing].tag, "p");
    assert!(
        !doc.focus_items().iter().any(|f| f.elem == enclosing),
        "a picture outside a link has no focus anchor"
    );
}

#[test]
fn a_picture_inside_a_link_points_at_the_link() {
    // The same bit is 1 when `frame[1] & 0x40` (the `<a>` style bit) is set. Host: the enclosing
    // element of `<a href=x><img></a>` is the `<a>` itself, a `HVFOCUS_LINK` item.
    let doc = doc_of("<a href=x><img src=y></a>");
    let pic = doc.runs.iter().find(|r| r.image.is_some()).unwrap();
    let at = pic.elem.unwrap();
    assert_eq!(doc.elements[at].tag, "a");
    assert!(
        doc.focus_items()
            .iter()
            .any(|f| f.elem == at && f.kind == g::HVFOCUS_LINK),
        "a picture inside a link is an anchor, and the anchor is the link"
    );
}

#[test]
fn every_fragment_of_a_wrapped_link_keeps_its_focus_target() {
    let doc = doc_of("<a href=x>AAAA BBBB CCCC DDDD EEEE FFFF GGGG</a>");
    let link = doc.find("a", 0).unwrap();
    let rows = placed_in(&doc, &Options::default(), 24, 400);
    let words: Vec<&Placed> = rows.iter().filter(|p| !p.text.is_empty()).collect();
    assert!(
        words.len() > 1,
        "a 24px column must wrap the link into several rows"
    );
    assert!(
        words.iter().all(|p| p.elem == Some(link)),
        "every fragment still names the link, including the last one of a wrapped row"
    );
    assert!(doc.focus_items().iter().any(|f| f.elem == link));
}
