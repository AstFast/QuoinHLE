//! `Widgets`' half of an `IHtmlViewer`: the option table, the document, the decode cache.
//!
//! An `IHtmlViewer` is a control like any other, so its state lives on the `Control` record and
//! these are methods on [`Widgets`]. They live here rather than in `widget.rs` because this is the
//! whole of what the class is — everything left in that file is `IControl` and its other children.
//!
//! Nothing here reaches into guest memory except where a slot's own signature does (`SetData`
//! copies a block in, `GetViewState` writes one out). The document itself is host-owned, parsed
//! once, and the decode cache sits beside it so a picture survives the frame that found it.

use super::html::{attr_of, has_attr, input_kind, Html};
use crate::hle::{Error as HleError, HResult};
use crate::mem::Mem;
use crate::widget::Widgets;

impl Widgets {
    // ------------------------------------------------------------- IHtmlViewer, slots 11-23

    /// The `IHtmlViewer` record of a control. Only that class carries one; a call through any other
    /// class is a missing record rather than a silent no-op.
    pub(crate) fn html(&mut self, addr: u32) -> HResult<&mut Html> {
        self.control(addr)?
            .html
            .as_mut()
            .ok_or(HleError::NotAnObject { addr })
    }

    /// `IHtmlViewer::SetOption`: `HVOpt` is `void *`, so the value is stored verbatim.
    pub fn html_set_option(&mut self, addr: u32, id: i32, value: u32) -> HResult<()> {
        self.html(addr)?.options.insert(id, value);
        Ok(())
    }

    /// `IHtmlViewer::GetOption`. A key nobody set answers 0, which is what a zeroed option table on
    /// the handset reads as.
    pub fn html_option(&mut self, addr: u32, id: i32) -> HResult<u32> {
        Ok(self.html(addr)?.options.get(&id).copied().unwrap_or(0))
    }

    /// Keep `HVOPT_SZ_URL` as text as well as in the option table: the pointer the caller passed
    /// is only promised for the length of `SetOption`, and resolving a later `href` needs the
    /// characters, not the address.
    pub fn html_set_base_url(&mut self, addr: u32, url: String) -> HResult<()> {
        self.html(addr)?.base_url = url;
        Ok(())
    }

    /// Keep `HVOPT_SZ_TYPE` as text as well as in the option table, for the same reason
    /// [`Widgets::html_set_base_url`] keeps the URL: the pointer is only promised for the length of
    /// `SetOption`, and the next parse is what reads the type.
    pub fn html_set_doc_type(&mut self, addr: u32, media_type: String) -> HResult<()> {
        let html = self.html(addr)?;
        html.doc_type = media_type;
        // The type decides how the same bytes are read, so the parse cache cannot survive it — and
        // neither can the edits: `text/plain` has no elements at all, so the indexes they are keyed
        // by no longer name the controls they did. The position is the same kind of thing: an
        // offset and a focus index name rows and controls of the document that is being replaced.
        html.parsed = None;
        html.marquee_epoch = None;
        html.marquee_elapsed = 0;
        html.values.clear();
        html.drag = None;
        html.scroll = 0;
        html.focus = 0;
        html.content = 0;
        html.content_key = None;
        Ok(())
    }

    pub fn html_base_url(&mut self, addr: u32) -> HResult<String> {
        Ok(self.html(addr)?.base_url.clone())
    }

    /// How many `src` values the page has settled — decoded or refused. Exposed so a test can see
    /// that a second redraw paints the same picture without decoding it again.
    pub fn html_image_cache_len(&mut self, addr: u32) -> HResult<usize> {
        Ok(self.html(addr)?.images.len())
    }

    /// Whether `src` has an answer already — a picture, or a note that there will not be one.
    ///
    /// A `None` counts: a `src` that 404'd or failed to decode is settled, and asking for it again
    /// on every redraw (or every `HVOPT_SZ_URL`) is exactly what the cache exists to prevent.
    pub fn html_picture_settled(&mut self, addr: u32, src: &str) -> HResult<bool> {
        Ok(self.html(addr)?.images.contains_key(src))
    }

    /// Record what became of one `<img src>`: the decoded picture, or `None` for a `src` that could
    /// not be had. The key is the attribute text, because that is what a redraw looks up.
    pub fn html_store_picture(
        &mut self,
        addr: u32,
        src: &str,
        picture: Option<std::sync::Arc<crate::resource::Bitmap>>,
    ) -> HResult<()> {
        let html = self.html(addr)?;
        html.images.insert(src.to_string(), picture);
        // A picture that has just settled gives its row a height it did not have a frame ago, so
        // the page's measured height is no longer the page's height.
        html.pictures = html.pictures.wrapping_add(1);
        Ok(())
    }

    /// `IHtmlViewer::IsBusy`'s answer while pictures are still in flight.
    ///
    /// The document's own bytes arrive synchronously, so this is only ever true between a load and
    /// the last `<img src>` it named coming back — which is why a page without URL images is never
    /// busy at all.
    pub fn html_set_busy(&mut self, addr: u32, busy: bool) -> HResult<()> {
        self.html(addr)?.busy = busy;
        Ok(())
    }

    pub fn html_reset_viewer(&mut self, addr: u32) -> HResult<()> {
        {
            let html = self.html(addr)?;
            html.data.clear();
            html.busy = false;
            html.focus = 0;
            html.scroll = 0;
            html.content = 0;
            html.content_key = None;
            html.parsed = None;
            html.marquee_epoch = None;
            html.marquee_elapsed = 0;
            html.values.clear();
            html.drag = None;
            // `Dox_SetUrl(dox, 0)`: the URL out of the option table and the text copy beside it.
            html.options.remove(&crate::web::html::HVOPT_SZ_URL);
            html.base_url.clear();
        }
        // "Immediately leave active/focus mode." No repaint is owed from here — the reset drops the
        // page the viewer was holding, and the caller draws the emptied control itself.
        self.set_active(addr, 0, false).map(|_| ())
    }

    /// `IHtmlViewer::SetNotifyFn`: the callback and its context, stored even when the pointer is null,
    /// because null is what un-installs it.
    pub fn html_set_notify(&mut self, addr: u32, notify: u32, user: u32) -> HResult<()> {
        let html = self.html(addr)?;
        html.notify = notify;
        html.user = user;
        Ok(())
    }

    /// `IHtmlViewer::LoadStream` / `LoadSource`: store the bytes [`Runtime::html_drain_source`]
    /// read from the handle. The load is synchronous, so the view is never left busy and there is
    /// nothing left to read later; an empty vector means the stream was null, spent, or would
    /// block.
    pub fn html_set_bytes(&mut self, addr: u32, bytes: Vec<u8>) -> HResult<()> {
        let html = self.html(addr)?;
        html.data = bytes;
        html.busy = false;
        // A new document is read from its top: the scroll was a position in the old one, and the
        // measured height belonged to it too.
        html.focus = 0;
        html.scroll = 0;
        html.content = 0;
        html.content_key = None;
        // New bytes are a new document; the parse cache is stale from here, and so is every edit —
        // `Values` is keyed by element index, and the indexes belong to the document that is gone.
        html.parsed = None;
        html.marquee_epoch = None;
        html.marquee_elapsed = 0;
        html.values.clear();
        // A pen that is still down went down on the page that is gone, so the gesture goes with it.
        html.drag = None;
        Ok(())
    }

    /// `IHtmlViewer::SetData`: copy the document into the control's own record. `cbLen == -1` means
    /// "to the NUL", which is what `IHTMLVIEWER_ParseBuffer` passes.
    pub fn html_set_data(&mut self, mem: &Mem, addr: u32, psz: u32, len: i32) -> HResult<()> {
        let bytes = if psz == 0 {
            Vec::new()
        } else if len < 0 {
            mem.read_cstr(psz, 1 << 20).map_err(HleError::Mem)?
        } else {
            mem.read_bytes(psz, len as usize).map_err(HleError::Mem)?
        };
        let html = self.html(addr)?;
        html.data = bytes;
        html.busy = false;
        // A new document is read from its top: the scroll was a position in the old one, and the
        // measured height belonged to it too.
        html.focus = 0;
        html.scroll = 0;
        html.content = 0;
        html.content_key = None;
        // New bytes are a new document; the parse cache is stale from here, and so is every edit.
        html.parsed = None;
        html.marquee_epoch = None;
        html.marquee_elapsed = 0;
        html.values.clear();
        html.drag = None;
        Ok(())
    }

    pub fn html_busy(&mut self, addr: u32) -> HResult<bool> {
        Ok(self.html(addr)?.busy)
    }

    pub fn html_stop(&mut self, addr: u32) -> HResult<()> {
        self.html(addr)?.busy = false;
        Ok(())
    }

    /// `IHtmlViewer::SetViewState`: put the page back where a previous `GetViewState` left it.
    ///
    /// The string is the scroll offset plus one record per control the user had touched, which is
    /// what "restores the HTML viewer state (such as scroll position)" means once a page has forms
    /// in it. A string this host cannot read — an older build's, or a pack's own invention — is
    /// ignored rather than refused: the interface returns `void`, so there is nobody to tell.
    /// `IHtmlViewer::GetViewState`: the shipped string — position, focus and the forms' words —
    /// that `SetViewState` reads back.
    pub fn html_view_state(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
    ) -> HResult<String> {
        let focus = if doc.focus_items().is_empty() {
            -1
        } else {
            let html = self.html(addr)?;
            (html.focus as i32).min(doc.focus_items().len() as i32 - 1)
        };
        let html = self.html(addr)?;
        Ok(crate::web::html::view_state(
            doc,
            &html.values.clone(),
            i32::from(html.scroll.max(0)),
            focus,
        ))
    }

    pub fn html_get_state(&mut self, mem: &mut Mem, addr: u32, buf: u32, cb: i32) -> HResult<u32> {
        let doc = self.html_document(addr)?;
        let state = self.html_view_state(addr, &doc)?;
        let bytes = state.as_bytes();
        let need = bytes.len() as u32;
        if buf == 0 || cb <= 0 {
            return Ok(need);
        }
        let n = bytes.len().min(cb as usize - 1);
        mem.write_bytes(buf, &bytes[..n]).map_err(HleError::Mem)?;
        mem.write_u8(buf + n as u32, 0).map_err(HleError::Mem)?;
        Ok(need)
    }

    /// `IHtmlViewer::SetViewState`: restore what [`Widgets::html_view_state`] wrote, or follow an
    /// `#anchor`.
    pub fn html_set_state(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        state: &str,
        faces: crate::font::Faces<'_>,
    ) -> HResult<()> {
        use crate::web::html::ViewRestore;
        // A restore *replaces* what the user did rather than merging with it: the shipped reader
        // resets the text, list and tick fields before it applies the string's words,
        //so a page a state does not mention comes back as the
        // page itself.
        self.html(addr)?.values.clear();
        match crate::web::html::parse_view_state(state) {
            ViewRestore::Anchor(name) => {
                // `id` first, then `name`, case folded — the shipped match is `aee_stricmp`.
                //The first element that answers is the one
                // `Dox_NextItem` lands on.
                let needle = name.to_ascii_lowercase();
                let target = doc.elements.iter().position(|e| {
                    attr_of(e, "id").to_ascii_lowercase() == needle
                        || attr_of(e, "name").to_ascii_lowercase() == needle
                });
                let Some(elem) = target else {
                    return Ok(());
                };
                let rect = self.peek(addr)?.rect;
                if rect.is_empty() {
                    return Ok(());
                }
                let options = {
                    let html = self.html(addr)?;
                    crate::web::html::Options::from_table(&html.options)
                };
                let pictures = self.decode_images(addr, doc)?;
                let whole = crate::display::Rect::new(rect.x, rect.y, rect.dx, i16::MAX);
                let rows = {
                    let html = self.html(addr)?;
                    crate::web::html::layout(
                        doc,
                        whole,
                        &options,
                        &pictures,
                        Some(&html.values),
                        0,
                        faces,
                    )
                };
                let Some(line) = rows.iter().find(|line| line.elem == Some(elem)) else {
                    return Ok(());
                };
                // `Dox_ScrollPosToTop`: the element's first row on the view's top edge.
                let top = i32::from(line.y) - i32::from(rect.y);
                let html = self.html(addr)?;
                html.scroll = top.clamp(0, i32::from(i16::MAX)) as i16;
                Ok(())
            }
            ViewRestore::Restore {
                scroll,
                focus,
                fields,
            } => {
                // The words go to the fields they name, in document order — the shipped reader
                // walks the forms in order and matches each word against the field it is on.
                //A word that names nothing in this document is
                // dropped: a state restored across a document change must not paint a box that is
                // not there.
                let mut taken = vec![false; fields.len()];
                for at in 0..doc.elements.len() {
                    let e = &doc.elements[at];
                    let name = match e.tag.as_str() {
                        "input" | "select" | "textarea" => attr_of(e, "name"),
                        _ => continue,
                    };
                    if name.is_empty() || has_attr(e, "disabled") {
                        continue;
                    }
                    // A box or a button-name radio only takes the word that carries its own
                    // value — its group's members share a name and differ in value, so the name
                    // alone cannot tell which one the string means. Each control takes the first
                    // word for its name that is still free, which is what keeps two fields that
                    // share a name from both getting the first word's text.
                    let kind = input_kind(e);
                    let want_value = |value: &str| {
                        let attr = attr_of(e, "value");
                        attr.is_empty() && value == "on" || value == attr
                    };
                    let pick = fields
                        .iter()
                        .enumerate()
                        .find(|(index, (word_name, word_value))| {
                            !taken[*index]
                                && word_name == name
                                && match kind {
                                    Some("checkbox") | Some("radio") => want_value(word_value),
                                    _ => true,
                                }
                        })
                        .map(|(index, _)| index);
                    let Some(index) = pick else { continue };
                    taken[index] = true;
                    let value = fields[index].1.clone();
                    let values = &mut self.html(addr)?.values;
                    match e.tag.as_str() {
                        "select" => {
                            let options = doc.options_of(at);
                            let hit = options.iter().position(|&option| {
                                doc.option_value(option) == value
                                    || doc.option_label(option) == value
                            });
                            if let Some(index) = hit {
                                let label = doc.option_label(options[index]);
                                values.choose(at, index, &label);
                            }
                        }
                        _ => match kind {
                            Some("checkbox") | Some("radio") => {
                                values.set_checked(at, true);
                                if kind == Some("radio") {
                                    for peer in doc.radio_group(at) {
                                        if peer != at {
                                            values.set_checked(peer, false);
                                        }
                                    }
                                }
                            }
                            // An editor: the restored value puts the caret after its last
                            // character, which is where a user who typed it would have left it.
                            _ => {
                                let entry = values.edit(at);
                                entry.text = Some(value);
                                entry.caret = entry
                                    .text
                                    .as_ref()
                                    .map(|t| t.chars().count())
                                    .unwrap_or_default();
                            }
                        },
                    }
                }
                let html = self.html(addr)?;
                html.scroll = scroll.clamp(0, i32::from(i16::MAX)) as i16;
                // The shipped reader only moves the focus when the string names one;
                //-1 leaves it where the page has it.
                if focus > -1 && !doc.focus_items().is_empty() {
                    html.focus = (focus as usize).min(doc.focus_items().len() - 1);
                }
                Ok(())
            }
        }
    }

    /// The parsed document, rebuilt from the bytes the last loader stored. A document is not a
    /// snapshot-shaped value, so it is parsed on demand rather than cached.
    /// The parsed document. It is cached on the page because `Redraw` asks for it every frame, and
    /// the cache is dropped the moment the bytes or the media type change — see
    /// [`Widgets::html_set_data`], [`Widgets::html_set_bytes`] and [`Widgets::html_set_doc_type`].
    /// A document is not a snapshot-shaped value, but it *is* immutable once parsed.
    pub fn html_document(
        &mut self,
        addr: u32,
    ) -> HResult<std::sync::Arc<crate::web::html::Document>> {
        let html = self.html(addr)?;
        if let Some(doc) = &html.parsed {
            return Ok(std::sync::Arc::clone(doc));
        }
        let doc = std::sync::Arc::new(crate::web::html::Document::parse_as(
            &html.data,
            &html.doc_type,
        ));
        html.parsed = Some(std::sync::Arc::clone(&doc));
        Ok(doc)
    }

    /// The pictures a document names, positionally matching [`Document::images`].
    ///
    /// The decode is cached on `Html` rather than on the document because the page outlives any one
    /// parse. A failure is cached too: a `src` that names nothing must not be retried thirty times
    /// a second.
    pub(crate) fn decode_images(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
    ) -> HResult<Vec<Option<std::sync::Arc<crate::resource::Bitmap>>>> {
        if doc.images.is_empty() {
            return Ok(Vec::new());
        }
        let html = self.html(addr)?;
        let mut pictures = Vec::with_capacity(doc.images.len());
        for image in &doc.images {
            let picture = match html.images.get(&image.src).cloned() {
                Some(hit) => hit,
                None => {
                    // Only a `data:` URI is bytes the document carries itself. A URL belongs to
                    // somebody else's answer: the fetch for it starts at load time, and settling
                    // it here as "there is no picture" would decide the question before the reply
                    // had a chance to arrive — and `HVOPT_PTR_IWEB` set afterwards would then find
                    // the `src` already settled and never ask at all.
                    let Some(bytes) = crate::web::html::image_bytes(&image.src) else {
                        pictures.push(None);
                        continue;
                    };
                    let fresh = crate::resource::image(&[], &bytes)
                        .ok()
                        .map(std::sync::Arc::new);
                    html.images.insert(image.src.clone(), fresh.clone());
                    fresh
                }
            };
            pictures.push(picture);
        }
        Ok(pictures)
    }

    /// `IHtmlViewer::SetNotifyFn`'s callback and its context, for the synchronous notifications a
    /// load sends.
    pub fn html_notify_target(&mut self, addr: u32) -> HResult<(u32, u32)> {
        let html = self.html(addr)?;
        Ok((html.notify, html.user))
    }

    /// How tall the document is at the width the control has now, in pixels.
    ///
    /// Measured by asking [`crate::web::html::layout`] to lay the page out in the tallest rect an
    /// `i16` can name and taking the bottom of the last row — the same code that places the rows,
    /// which is the only way the two numbers can agree. The answer is cached on the page because a
    /// frame is not a reason to walk a thousand rows; only a new parse, a new column width, a new
    /// paragraph spacing, a picture that has just settled or a face installed after the last
    /// measurement can change it, and those are the cache key. The face one matters most on a
    /// handset: the page is measured with the face the painter draws with, so installing a 16-pixel
    /// CJK BBF under `AEE_FONT_NORMAL` halves the words a line and doubles the page — measured once
    /// before the install, the height would be wrong for the rest of the run.
    ///
    /// The width in the key is [`crate::web::html::content_width`] and not `rect.dx`, because rows
    /// fold: `HVOPT_N_INDENT` and `HVOPT_N_SBWIDTH` narrow the column without touching the
    /// rectangle, so a pack that widened its indent after the first measurement would otherwise be
    /// handed the height it had before — and the page it can no longer scroll to the bottom of
    /// gives no other sign of it.
    pub(crate) fn html_content_height(
        &mut self,
        addr: u32,
        doc: &std::sync::Arc<crate::web::html::Document>,
        pictures: &crate::web::html::Pictures,
        opts: &crate::web::html::Options,
        rect: crate::display::Rect,
        display: &crate::display::Display,
    ) -> HResult<i16> {
        let settled = self.html(addr)?.pictures;
        let key = (
            std::sync::Arc::as_ptr(doc) as usize,
            crate::web::html::content_width(rect, opts),
            // The indent step narrows every row inside a `<blockquote>`/`<dd>`/list, and it is not
            // part of `content_width` — the page's own column is the rect less the scrollbar. A key
            // without it would answer the height of the page as it was before the applet changed
            // `HVOPT_N_INDENT`.
            opts.indent,
            opts.vspace,
            settled,
            display.face_epoch(),
        );
        {
            let html = self.html(addr)?;
            if html.content_key == Some(key) {
                return Ok(html.content);
            }
        }
        let tall = crate::display::Rect::new(rect.x, rect.y, rect.dx, i16::MAX);
        // What the user has typed changes what the boxes say, not how tall the page is, but the
        // measurement has to be the same layout the frame will use or the two disagree at the end.
        let rows = {
            let html = self.html(addr)?;
            crate::web::html::layout(
                doc,
                tall,
                opts,
                pictures,
                Some(&html.values),
                0,
                display.faces(),
            )
        };
        let content = rows
            .iter()
            .map(|line| line.y.saturating_add(line.row))
            .max()
            .unwrap_or(rect.y)
            .saturating_sub(rect.y);
        let html = self.html(addr)?;
        html.content = content;
        html.content_key = Some(key);
        Ok(content)
    }

    /// Scroll the page just enough to show the element the focus is on.
    ///
    /// This is the whole of the scrolling the interface has: `HVOPT_*` names no scroll position and
    /// the viewer documents no scrolling key, so the page follows the keyboard. The motion is the
    /// smallest one that reveals the focused row, which is also why nothing here needs to know how
    /// tall the page is — an adjustment that only ever moves by what is hidden cannot run past the
    /// end.
    pub(crate) fn html_reveal_focus(
        &mut self,
        addr: u32,
        doc: &std::sync::Arc<crate::web::html::Document>,
        faces: crate::font::Faces<'_>,
    ) -> HResult<()> {
        let rect = self.peek(addr)?.rect;
        if rect.is_empty() {
            return Ok(());
        }
        let Some((elem, _)) = self.html_focus(addr, doc)? else {
            return Ok(());
        };
        let options = {
            let html = self.html(addr)?;
            crate::web::html::Options::from_table(&html.options)
        };
        let pictures = self.decode_images(addr, doc)?;
        // The element's row has to be found in the *whole* document, not in what is on screen: the
        // one case that matters is the row that is not.
        let whole = crate::display::Rect::new(rect.x, rect.y, rect.dx, i16::MAX);
        let rows = {
            let html = self.html(addr)?;
            crate::web::html::layout(
                doc,
                whole,
                &options,
                &pictures,
                Some(&html.values),
                0,
                faces,
            )
        };
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        for line in &rows {
            if line.elem == Some(elem) {
                top = top.min(i32::from(line.y));
                bottom = bottom.max(i32::from(line.y) + i32::from(line.row));
            }
        }
        if top > bottom {
            // Nothing on the page paints for this element — a link with no text, say. There is
            // nothing to scroll to.
            return Ok(());
        }
        let scroll = i32::from(self.html(addr)?.scroll);
        let view_top = i32::from(rect.y);
        let view_bottom = view_top + i32::from(rect.dy);
        let next = if top - scroll < view_top {
            // Above the top edge: put its first row on the edge.
            scroll - (view_top - (top - scroll))
        } else if bottom - scroll > view_bottom {
            // Below the bottom edge: put its last row on the edge.
            scroll + (bottom - scroll - view_bottom)
        } else {
            scroll
        };
        self.html(addr)?.scroll = next.clamp(0, i32::from(i16::MAX)) as i16;
        Ok(())
    }

    /// The control's class, for the dispatch that has to tell an `IHtmlViewer` from its siblings.
    pub fn class(&self, addr: u32) -> HResult<&'static str> {
        Ok(self.peek(addr)?.class)
    }

    /// Where the user's focus is: the element index and its `HVFOCUS_*` kind, or `None` when the
    /// document has nothing to land on. A stored position past the end (the document changed under
    /// it) wraps to the first item.
    pub fn html_focus(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
    ) -> HResult<Option<(usize, u32)>> {
        let items = doc.focus_items();
        if items.is_empty() {
            return Ok(None);
        }
        let html = self.html(addr)?;
        if html.focus >= items.len() {
            html.focus = 0;
        }
        let at = html.focus;
        Ok(Some((items[at].elem, items[at].kind)))
    }

    /// Move the focus by one and answer where it landed.
    pub fn html_move_focus(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        up: bool,
    ) -> HResult<Option<(usize, u32)>> {
        let items = doc.focus_items();
        if items.is_empty() {
            return Ok(None);
        }
        let html = self.html(addr)?;
        if html.focus >= items.len() {
            html.focus = 0;
        }
        html.focus = if up {
            html.focus.saturating_sub(1)
        } else {
            (html.focus + 1).min(items.len() - 1)
        };
        let at = html.focus;
        Ok(Some((items[at].elem, items[at].kind)))
    }

    // ------------------------------------------------------------------ what the user does to a form

    /// What `elem` shows now: the user's text when there is one, the document's own value otherwise.
    /// This is where an edit starts, so it is also the value a caret is placed in.
    pub(crate) fn html_control_text(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
    ) -> HResult<String> {
        let edited = self.html(addr)?.values.text(elem).map(str::to_string);
        Ok(edited.unwrap_or_else(|| doc.control_text(elem)))
    }

    /// Put one character in at the caret and leave the caret after it.
    ///
    /// `base` is what the control showed before the first keystroke, so a field the document filled
    /// in keeps its text instead of starting empty.
    pub(crate) fn html_insert(
        &mut self,
        addr: u32,
        elem: usize,
        base: &str,
        ch: char,
    ) -> HResult<()> {
        let entry = self.html(addr)?.values.begin(elem, base);
        let mut chars: Vec<char> = entry.text.clone().unwrap_or_default().chars().collect();
        let at = entry.caret.min(chars.len());
        chars.insert(at, ch);
        entry.caret = at + 1;
        entry.text = Some(chars.into_iter().collect());
        Ok(())
    }

    /// Delete the character before the caret. `false` when there is nothing to delete, which is what
    /// lets the same key move the focus instead.
    pub(crate) fn html_backspace(&mut self, addr: u32, elem: usize, base: &str) -> HResult<bool> {
        let entry = self.html(addr)?.values.begin(elem, base);
        let mut chars: Vec<char> = entry.text.clone().unwrap_or_default().chars().collect();
        let at = entry.caret.min(chars.len());
        if at == 0 {
            return Ok(false);
        }
        chars.remove(at - 1);
        entry.caret = at - 1;
        entry.text = Some(chars.into_iter().collect());
        Ok(true)
    }

    /// Move the caret one line up or down inside a textarea, keeping its column.
    ///
    /// `false` when there is no line to move to — either the value has no line breaks at all (a
    /// one-line field, where the arrow belongs to the focus) or the caret is on the first/last
    /// line. That is the same boundary rule the left and right arrows use, and it is what lets one
    /// arrow key mean "walk the text" inside the box and "walk the page" outside it.
    pub(crate) fn html_move_caret_line(
        &mut self,
        addr: u32,
        elem: usize,
        base: &str,
        up: bool,
    ) -> HResult<bool> {
        let entry = self.html(addr)?.values.begin(elem, base);
        let chars: Vec<char> = entry.text.clone().unwrap_or_default().chars().collect();
        if !chars.contains(&'\n') {
            return Ok(false);
        }
        let caret = entry.caret.min(chars.len());
        let start = chars[..caret]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |at| at + 1);
        let column = caret - start;
        let next = if up {
            if start == 0 {
                return Ok(false);
            }
            // The break before this line, and the start of the line before that.
            let end = start - 1;
            let line = chars[..end]
                .iter()
                .rposition(|&c| c == '\n')
                .map_or(0, |at| at + 1);
            line + column.min(end - line)
        } else {
            let Some(end) = chars[caret..]
                .iter()
                .position(|&c| c == '\n')
                .map(|at| caret + at)
            else {
                return Ok(false);
            };
            let line = end + 1;
            let line_end = chars[line..]
                .iter()
                .position(|&c| c == '\n')
                .map_or(chars.len(), |at| line + at);
            line + column.min(line_end - line)
        };
        entry.caret = next;
        Ok(true)
    }

    /// Move the caret one place. `false` at either end of the text, so a caller can let the arrow
    /// leave the field at the boundary rather than swallowing it.
    pub(crate) fn html_move_caret(
        &mut self,
        addr: u32,
        elem: usize,
        base: &str,
        right: bool,
    ) -> HResult<bool> {
        let entry = self.html(addr)?.values.begin(elem, base);
        let len = entry.text.as_deref().unwrap_or("").chars().count();
        let at = entry.caret.min(len);
        if (right && at >= len) || (!right && at == 0) {
            return Ok(false);
        }
        entry.caret = if right { at + 1 } else { at - 1 };
        Ok(true)
    }

    /// Tick or untick a box, answering with the state it now has. A radio also clears the rest of
    /// its group — a set of radios is a choice of one, which is the whole difference between
    /// `HVFOCUS_RADIO` and `HVFOCUS_CHECKBOX`.
    pub(crate) fn html_toggle(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
        radio: bool,
    ) -> HResult<bool> {
        let was = self
            .html(addr)?
            .values
            .checked(elem)
            .unwrap_or_else(|| doc.is_checked(elem));
        if radio && was {
            // Choosing the radio that is already chosen leaves it chosen: a group has no state in
            // which none of them is on, once one has been.
            return Ok(true);
        }
        let group = if radio {
            doc.radio_group(elem)
        } else {
            vec![elem]
        };
        {
            let values = &mut self.html(addr)?.values;
            for at in group {
                values.set_checked(at, at == elem && !was);
            }
        }
        Ok(!was)
    }

    /// Whether a box is ticked now: the user's tick when there is one, the document's attribute
    /// otherwise.
    pub(crate) fn html_checked(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
    ) -> HResult<bool> {
        Ok(self
            .html(addr)?
            .values
            .checked(elem)
            .unwrap_or_else(|| doc.is_checked(elem)))
    }

    /// Which option a list shows, as an index into its options.
    pub(crate) fn html_choice(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
    ) -> HResult<usize> {
        Ok(self
            .html(addr)?
            .values
            .get(elem)
            .and_then(|e| e.choice)
            .unwrap_or_else(|| doc.chosen_index(elem)))
    }

    /// Walk a list to the next or previous option, answering `false` at either end so the arrow can
    /// leave the list rather than getting stuck in it.
    pub(crate) fn html_cycle(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
        step: i32,
    ) -> HResult<bool> {
        let options = doc.options_of(elem);
        if options.is_empty() {
            return Ok(false);
        }
        let at = self.html_choice(addr, doc, elem)?;
        let next = at as i32 + step;
        if next < 0 || next as usize >= options.len() {
            return Ok(false);
        }
        let next = next as usize;
        let label = doc.option_label(options[next]);
        self.html(addr)?.values.choose(elem, next, &label);
        Ok(true)
    }

    /// `<input type=reset>`: forget every edit in the form the button belongs to.
    pub(crate) fn html_reset_form(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
    ) -> HResult<()> {
        let (lo, hi) = doc.form_range(elem);
        self.html(addr)?.values.reset(lo..hi);
        Ok(())
    }

    /// Land the focus on one element, answering its `HVFOCUS_*` kind — or `None` when the element is
    /// not something a user can land on. Used by the pen, which names a place rather than a step.
    pub(crate) fn html_focus_on(
        &mut self,
        addr: u32,
        doc: &crate::web::html::Document,
        elem: usize,
    ) -> HResult<Option<u32>> {
        let items = doc.focus_items();
        let Some(at) = items.iter().position(|item| item.elem == elem) else {
            return Ok(None);
        };
        self.html(addr)?.focus = at;
        Ok(Some(items[at].kind))
    }

    /// Start an edit record at the end of what a field already holds, if it has none yet. Called
    /// when the focus lands in an editor, so the caret has somewhere to be before the first
    /// character arrives.
    pub(crate) fn html_begin_edit(&mut self, addr: u32, elem: usize, base: &str) -> HResult<()> {
        self.html(addr)?.values.begin(elem, base);
        Ok(())
    }

    /// `HVP_INLINE_EDIT`: whether a focused field takes typed characters.
    pub(crate) fn html_inline_edit(&self, addr: u32) -> HResult<bool> {
        Ok(self.properties(addr)? & crate::abi::generated::HVP_INLINE_EDIT != 0)
    }
}
