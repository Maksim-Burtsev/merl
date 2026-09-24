//! What is drawn over or beside the code: the `?` keymap, the file tree, the picker and
//! the tutor's lesson panel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use unicode_segmentation::UnicodeSegmentation;

use crate::app::{App, Focus, Mode, PickerKind};
use crate::buffer::{Buffer, Spans};
use crate::picker::PickItem;
use crate::search::MAX_HITS;
use crate::theme::Theme;
use crate::wrap;

use super::expand;
use super::status::draw_prompt;

/// `?`: the whole keymap, straight out of [`crate::app::KEYS`], each group under its name in
/// the accent. Up / Down scroll it when the terminal is too short for the whole list.
pub(super) fn draw_help(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let keys = crate::app::KEYS;
    let key_w = keys.iter().map(|(k, _, _)| k.len()).max().unwrap_or(0);
    let bold = base.add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line> = Vec::new();
    let mut group = "";
    for (key, action, g) in keys {
        if *g != group {
            group = g;
            lines.push(Line::styled(format!(" {group}"), bold.fg(theme.accent)));
        }
        lines.push(Line::from(vec![
            Span::styled(format!("   {key:key_w$}  "), bold),
            Span::styled(*action, base.fg(theme.ghost_fg)),
        ]));
    }
    let w = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 1;
    let [area] = Layout::horizontal([Constraint::Length((w + 4).min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(
        (lines.len() as u16 + 2).min(area.height),
    )])
    .flex(Flex::Center)
    .areas(area);

    let inner = Block::bordered().inner(area);
    let max_top = lines.len().saturating_sub(inner.height as usize);
    app.help_top = app.help_top.min(max_top);
    let title = if max_top > 0 {
        "merl — keys (Up / Down to scroll)"
    } else {
        "merl — keys"
    };
    let block = Block::bordered().title(title).style(base);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(lines)
            .style(base)
            .scroll((app.help_top as u16, 0)),
        inner,
    );
}

pub(super) fn draw_tree(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let accent = base.fg(theme.accent);
    let title = match &app.review {
        Some(r) => format!("{} \u{2190} {}", r.branch, r.base),
        None => app.root_name(),
    };
    let block = Block::bordered()
        .title(title)
        .border_style(accent)
        .title_style(accent.add_modifier(Modifier::BOLD))
        .style(base);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let focused = app.focus == Focus::Tree;
    let visible = app.tree.visible();
    let height = inner.height as usize;
    let at = visible
        .iter()
        .position(|&i| i == app.tree.cursor)
        .unwrap_or(0);
    // Scroll the minimum amount that keeps the cursor row on screen.
    app.tree_top = app.tree_top.min(at).max((at + 1).saturating_sub(height));

    let width = inner.width as usize;
    let rows: Vec<Line> = visible
        .iter()
        .skip(app.tree_top)
        .take(height)
        .map(|&i| {
            let n = &app.tree.nodes[i];
            let marker = match (n.is_dir, n.expanded) {
                (true, true) => "\u{25be} ",
                (true, false) => "\u{25b8} ",
                (false, _) => "  ",
            };
            let mut text = format!("{}{marker}{}", "  ".repeat(n.depth), n.name());
            // Review: a column of ticks for the viewed files, before every row.
            let tick = match (&app.review, app.viewed.contains_key(&n.path)) {
                (None, _) => "",
                (Some(_), false) => "  ",
                (Some(_), true) => "\u{2713} ",
            };
            let width = width.saturating_sub(wrap::width(tick));
            // Review: `M name  +6 -2`, the status in place of the marker.
            if let Some(f) = app.review.as_ref().and_then(|r| r.file(&n.path)) {
                let counts = if f.binary {
                    "bin".to_string()
                } else {
                    format!("+{} \u{2212}{}", f.added, f.deleted)
                };
                // The counts stay; a long name gives way, with the cut marked.
                let lead = format!("{}{} ", "  ".repeat(n.depth), f.status);
                let room = width.saturating_sub(wrap::width(&lead) + wrap::width(&counts) + 1);
                let name = n.name();
                let name = if wrap::width(&name) > room {
                    crate::app::clip(&name, room.saturating_sub(1))
                } else {
                    name.to_string()
                };
                text = format!("{lead}{name}");
                let gap = width.saturating_sub(wrap::width(&text) + wrap::width(&counts));
                text.push_str(&" ".repeat(gap));
                text.push_str(&counts);
            }
            // Directories carry the accent: they are what the eye scans the tree by. What
            // `.gitignore` leaves out is dim, directory or not.
            let row = if n.ignored {
                base.fg(theme.ghost_fg)
            } else if n.is_dir {
                accent
            } else {
                base
            };
            let style = if i != app.tree.cursor {
                row
            } else if focused {
                row.bg(theme.line_hl)
            } else {
                // The tree keeps its place while the code pane has the keys. Only the
                // background fades: the file name must stay readable (issue #8).
                row.bg(theme.line_hl_dim)
            };
            let pad = width.saturating_sub(wrap::width(&text));
            Line::from(vec![
                Span::styled(tick, style.fg(theme.accent)),
                Span::styled(text, style),
                Span::styled(" ".repeat(pad), style),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(rows).style(base), inner);
}

pub(super) fn draw_picker(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    base: Style,
) {
    let pending = app.search_pending();
    // `s> ` is the prompt of `s`; `D` greps for the query too, and stays `D`'s picker.
    let search = app.mode == Mode::Picker(crate::app::PickerKind::Search);
    let Some(picker) = &mut app.picker else {
        return;
    };
    let [area] = Layout::horizontal([Constraint::Percentage(80)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(60)])
        .flex(Flex::Center)
        .areas(area);

    let (matched, total) = picker.counts();
    let accent = base.fg(theme.accent);
    let block = Block::bordered()
        .title(if picker.live && pending {
            // The rows answer an older query: `0 hits` only ever means "grepped, found nothing".
            format!("{} (…)", picker.title)
        } else if picker.live && picker.query.is_empty() && total > 0 {
            // `D` past the cap: the rows are what the walk found before the cut, not the list,
            // and the query greps the project instead of filtering them. (`s` has no rows
            // without a query, so it never reads this way.)
            format!("{} (first {total}, type to search all)", picker.title)
        } else if picker.live {
            // Nothing filters the hits, and the grep stops at MAX_HITS: that many is a floor.
            let more = if total as usize >= MAX_HITS { "+" } else { "" };
            let s = crate::app::plural(total as usize);
            format!("{} ({total}{more} hit{s})", picker.title)
        } else {
            format!("{} ({matched}/{total})", picker.title)
        })
        .border_style(accent)
        .title_style(accent.add_modifier(Modifier::BOLD))
        .style(base);
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let [prompt, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    let prefix = if search { "s> " } else { "> " };
    draw_prompt(frame, prefix, &picker.query, base, prompt);

    let (rows, selected) = picker.window(list.height as usize);
    let width = list.width as usize;
    let files = app.mode == Mode::Picker(PickerKind::Files);
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            // An ignored file is dim, as in the tree.
            let base = if files && app.ignored.contains(&row.item.path) {
                base.fg(theme.ghost_fg)
            } else {
                base
            };
            let style = if i == selected {
                base.bg(theme.line_hl)
            } else {
                base
            };
            let label = &row.item.label;
            let code = code_hl(&mut picker.bufs, &app.root, &row.item, theme);
            // A span per cluster: ratatui measures each span apart, so an emoji split from its
            // selector would be drawn one column narrower than `wrap::width` counts it.
            let mut c = 0;
            let mut spans: Vec<Span> = label
                .grapheme_indices(true)
                .map(|(b, g)| {
                    // The matcher numbers chars; a cluster is bold when any of its chars matched.
                    let chars = c..c + g.chars().count() as u32;
                    c = chars.end;
                    let mut style = style;
                    if let (Some((hl, off)), Some(at)) = (&code, row.item.code_at)
                        && b >= at
                        && let Some((s, _)) = hl.iter().find(|(_, r)| r.contains(&(b - at + off)))
                    {
                        style = style.patch(*s);
                    }
                    if chars
                        .into_iter()
                        .any(|k| row.matched.binary_search(&k).is_ok())
                    {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    // Quoted code keeps its tabs, as the buffer does; `expand` draws them.
                    Span::styled(expand(g).into_owned(), style)
                })
                .collect();
            spans.push(Span::styled(
                " ".repeat(width.saturating_sub(wrap::width(label))),
                style,
            ));
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).style(base), list);
}

/// The highlighting of the file line an item quotes, plus the byte offset of the quoted text
/// in that line, so the picker row gets the same colours as the code view. `None` when the
/// item has no code text or the file no longer contains what the label shows.
fn code_hl<'a>(
    bufs: &'a mut HashMap<PathBuf, Buffer>,
    root: &Path,
    item: &PickItem,
    theme: &Theme,
) -> Option<(&'a Spans, usize)> {
    let quoted = item.label[item.code_at?..].trim_end_matches('\u{2026}');
    let buf = bufs.entry(item.path.clone()).or_insert_with(|| {
        Buffer::load(&root.join(&item.path)).unwrap_or_else(|_| Buffer::empty())
    });
    let idx = item.line.checked_sub(1)?;
    let line = buf.lines.get(idx)?;
    let off = line.len() - line.trim_start().len();
    if line.get(off..off + quoted.len()) != Some(quoted) {
        return None;
    }
    buf.highlight_to(idx, theme);
    Some((buf.hl.get(idx)?, off))
}

/// `--tutor`'s current lesson, or `--drill`'s task, three rows above the status bar.
pub(super) fn draw_lesson(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, base: Style) {
    let Some(tutor) = &app.tutor else { return };
    let (title, text) = match (&tutor.drill, crate::tutor::lesson(tutor.step)) {
        (Some(drill), _) => drill.panel(),
        (None, Some(t)) => (
            format!(
                " Tutor {}/{} \u{00b7} {}",
                tutor.step + 1,
                crate::tutor::TUTOR.len(),
                t.title
            ),
            t.tutor,
        ),
        (None, None) => (" Tutor \u{2713} done".to_string(), crate::tutor::DONE),
    };
    // The title row is a bar, so it is padded to the full width.
    let pad = (area.width as usize).saturating_sub(wrap::width(&title));
    let head = Style::new()
        .bg(theme.status_bg)
        .fg(theme.status_fg)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(Span::styled(format!("{title}{}", " ".repeat(pad)), head)),
        Line::from(Span::styled(format!(" {text}"), base)),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).style(base),
        area,
    );
}
