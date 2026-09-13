//! Drawing. Reads `App`, writes only the viewport size back into it.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use regex::Regex;

use crate::app::{App, Focus, Mode};
use crate::buffer::{Buffer, Spans};
use crate::picker::PickItem;
use crate::theme::Theme;
use crate::wrap;

/// Width of the file tree pane.
const TREE_W: u16 = 30;
/// ponytail: a single line longer than this is truncated for rendering only.
const MAX_RENDER_BYTES: usize = 20_000;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme) {
    let area = frame.area();
    let base = Style::new().bg(theme.bg).fg(theme.fg);
    frame.render_widget(Block::new().style(base), area);

    // The lesson panel is 0 rows tall outside `--tutor`, so nothing else moves.
    let lesson_h = if app.tutor.is_some() { 3 } else { 0 };
    let [main, lesson, status] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(lesson_h),
        Constraint::Length(1),
    ])
    .areas(area);
    let tree_w = if app.show_tree { TREE_W } else { 0 };
    let [tree, code] =
        Layout::horizontal([Constraint::Length(tree_w), Constraint::Min(1)]).areas(main);

    if app.show_tree {
        draw_tree(frame, app, theme, tree, base);
    }
    if app.buf.path.is_some() {
        draw_code(frame, app, theme, code, base);
    } else {
        let hint = Line::from(Span::styled(
            " o: open file   ?: help",
            base.fg(theme.gutter_fg),
        ));
        frame.render_widget(Paragraph::new(hint).style(base), code);
    }
    if app.tutor.is_some() {
        draw_lesson(frame, app, theme, lesson, base);
    }
    draw_status(frame, app, theme, status);
    if app.picker.is_some() {
        draw_picker(frame, app, theme, area, base);
    }
    if app.mode == Mode::Help {
        draw_help(frame, theme, area, base);
    }
}

/// `?`: the whole keymap, straight out of [`crate::app::KEYS`].
fn draw_help(frame: &mut Frame, theme: &Theme, area: Rect, base: Style) {
    let keys = crate::app::KEYS;
    let key_w = keys.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let w = keys
        .iter()
        .map(|(_, a)| key_w + 4 + a.len())
        .max()
        .unwrap_or(0) as u16;
    let [area] = Layout::horizontal([Constraint::Length((w + 4).min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length((keys.len() as u16 + 2).min(area.height))])
        .flex(Flex::Center)
        .areas(area);

    let block = Block::bordered().title("merl — keys").style(base);
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    let lines: Vec<Line> = keys
        .iter()
        .map(|(key, action)| {
            Line::from(vec![
                Span::styled(
                    format!(" {key:key_w$}  "),
                    base.add_modifier(Modifier::BOLD),
                ),
                Span::styled(*action, base.fg(theme.gutter_fg)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).style(base), inner);
}

fn draw_tree(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let block = Block::bordered().title(app.root_name()).style(base);
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
            let text = format!("{}{marker}{}", "  ".repeat(n.depth), n.name());
            let style = if i != app.tree.cursor {
                base
            } else if focused {
                base.bg(theme.line_hl)
            } else {
                // The tree keeps its place while the code pane has the keys. Only the
                // background fades: the file name must stay readable (issue #8).
                base.bg(theme.line_hl_dim)
            };
            let pad = width.saturating_sub(wrap::width(&text));
            Line::from(vec![
                Span::styled(text, style),
                Span::styled(" ".repeat(pad), style),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(rows).style(base), inner);
}

fn draw_picker(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
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
    let block = Block::bordered()
        .title(format!("{} ({matched}/{total})", picker.title))
        .style(base);
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let [prompt, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    frame.render_widget(
        Paragraph::new(format!("> {}", picker.query)).style(base),
        prompt,
    );
    frame.set_cursor_position((prompt.x + 2 + wrap::width(&picker.query) as u16, prompt.y));

    let (rows, selected) = picker.window(list.height as usize);
    let width = list.width as usize;
    let lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let style = if i == selected {
                base.bg(theme.line_hl)
            } else {
                base
            };
            let label = &row.item.label;
            let code = code_hl(&mut picker.bufs, &app.root, &row.item, theme);
            let mut spans: Vec<Span> = label
                .char_indices()
                .enumerate()
                .map(|(c, (b, ch))| {
                    let mut style = style;
                    if let (Some((hl, off)), Some(at)) = (&code, row.item.code_at)
                        && b >= at
                        && let Some((s, _)) = hl.iter().find(|(_, r)| r.contains(&(b - at + off)))
                    {
                        style = style.patch(*s);
                    }
                    if row.matched.binary_search(&(c as u32)).is_ok() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    Span::styled(ch.to_string(), style)
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

fn draw_code(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let gutter_w = digits(app.buf.lines.len()) + 1;
    app.view_w = (area.width as usize).saturating_sub(gutter_w).max(1);
    app.view_h = area.height as usize;
    app.clamp_scroll();
    // Wrapping only ever costs rows, so this covers every visible line.
    app.buf.highlight_to(app.top_line + app.view_h, theme);

    let gutter_style = base.fg(theme.gutter_fg);
    let find_style = Style::new().bg(theme.find_bg).fg(theme.find_fg);
    let hl = base.bg(theme.line_hl);
    let hl_gutter = gutter_style.bg(theme.line_hl);
    let sel = base.bg(theme.selection);
    // Selected lines above the selection's last line are selected through their newline, so
    // their background runs to the right edge like VS Code's.
    let sel_lines = app.selection().map(|(start, end)| (start.0, end.0));

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let mut l = app.top_line;
    let mut skip = app.top_row;
    while lines.len() < area.height as usize && l < app.buf.lines.len() {
        let text = &app.buf.lines[l];
        let clipped = &text[..floor_boundary(text, MAX_RENDER_BYTES)];
        let spans = app.buf.hl.get(l).map_or(&[][..], Vec::as_slice);
        // Find matches paint over the syntax colours, so they are merged into the span list.
        let merged;
        let spans = match app.find_re.as_ref().map(|re| matches(re, clipped)) {
            Some(f) if !f.is_empty() => {
                merged = with_find(spans, &f, find_style);
                &merged[..]
            }
            _ => spans,
        };
        let cursor_line = l == app.line;
        let (g, t) = if cursor_line {
            (hl_gutter, hl)
        } else {
            (gutter_style, base)
        };
        let selected = app
            .selected_bytes(l)
            .map(|r| r.start..r.end.min(clipped.len()));
        let pad_selected = sel_lines.is_some_and(|(first, last)| first <= l && l < last);
        for (i, r) in wrap::wrap_line(clipped, app.view_w).into_iter().enumerate() {
            if i < skip {
                continue;
            }
            if lines.len() == area.height as usize {
                break;
            }
            let num = if i == 0 {
                format!("{:>w$} ", l + 1, w = gutter_w - 1)
            } else {
                " ".repeat(gutter_w)
            };
            let mut row = vec![Span::styled(num, g)];
            let pad = app.view_w.saturating_sub(wrap::width(&clipped[r.clone()]));
            // The selected part of the row keeps its syntax colours on the selection background.
            let (lo, hi) = match &selected {
                Some(s) => (s.start.clamp(r.start, r.end), s.end.clamp(r.start, r.end)),
                None => (r.end, r.end),
            };
            for (piece, style) in [(r.start..lo, t), (lo..hi, sel), (hi..r.end, t)] {
                if !piece.is_empty() {
                    row.extend(row_spans(clipped, spans, &piece, style));
                }
            }
            if cursor_line || pad_selected {
                // Pad so the line background reaches the right edge of the pane.
                let style = if pad_selected { sel } else { t };
                row.push(Span::styled(" ".repeat(pad), style));
            }
            lines.push(Line::from(row));
        }
        skip = 0;
        l += 1;
    }
    frame.render_widget(Paragraph::new(lines).style(base), area);

    let screen_row = rows_between(
        app,
        (app.top_line, app.top_row),
        (app.line, app.cursor_row()),
    );
    if let Some(y) = screen_row.filter(|y| *y < area.height as usize)
        && app.mode == Mode::Normal
    {
        let x = area.x + (gutter_w + app.cursor_x()) as u16;
        frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y + y as u16));
    }
}

/// `--tutor`: the current lesson, three rows above the status bar.
fn draw_lesson(frame: &mut Frame, app: &App, theme: &Theme, area: Rect, base: Style) {
    let Some(tutor) = &app.tutor else { return };
    let lessons = crate::tutor::LESSONS;
    let (title, text) = match lessons.get(tutor.step) {
        Some(l) => (
            format!(
                " Tutor {}/{} \u{00b7} {}",
                tutor.step + 1,
                lessons.len(),
                l.title
            ),
            l.text,
        ),
        None => (" Tutor \u{2713} done".to_string(), crate::tutor::DONE),
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

fn draw_status(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let style = Style::new().bg(theme.status_bg).fg(theme.status_fg);
    let prefix = match app.mode {
        Mode::Goto => ":",
        Mode::Find => "/",
        Mode::Search => "s>",
        _ => "",
    };
    if !prefix.is_empty() {
        let text = format!("{prefix}{}", app.prompt);
        frame.set_cursor_position((area.x + wrap::width(&text) as u16, area.y));
        frame.render_widget(Paragraph::new(text).style(style), area);
        return;
    }
    let mut spans = vec![
        Span::styled(app.rel_path(), style.add_modifier(Modifier::BOLD)),
        Span::styled(
            format!(
                "  {}:{}  [{}]",
                app.line + 1,
                app.display_col(),
                if app.focus == Focus::Tree {
                    "tree"
                } else {
                    "code"
                }
            ),
            style,
        ),
    ];
    if !app.message.is_empty() {
        spans.push(Span::styled(format!("  {}", app.message), style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
    // With nothing to report, the right edge keeps `?` discoverable.
    const HINT: &str = "? help ";
    if app.message.is_empty() && area.width as usize > HINT.len() {
        let hint = Rect {
            x: area.right() - HINT.len() as u16,
            width: HINT.len() as u16,
            ..area
        };
        frame.render_widget(Paragraph::new(HINT).style(style), hint);
    }
}

/// Cuts one wrapped row `r` of `text` into spans, taking colours from the line's highlighting
/// and everything else from `base` (which carries the pane or cursor-line background).
fn row_spans<'a>(
    text: &'a str,
    hl: &[(Style, std::ops::Range<usize>)],
    r: &std::ops::Range<usize>,
    base: Style,
) -> Vec<Span<'a>> {
    let mut out = Vec::new();
    let mut pos = r.start;
    for (style, span) in hl {
        if span.end <= r.start {
            continue;
        }
        if span.start >= r.end {
            break;
        }
        let (start, end) = (span.start.max(pos), span.end.min(r.end));
        if pos < start {
            out.push(Span::styled(&text[pos..start], base));
        }
        out.push(Span::styled(&text[start..end], base.patch(*style)));
        pos = end;
    }
    if pos < r.end || out.is_empty() {
        out.push(Span::styled(&text[pos..r.end], base));
    }
    out
}

/// Non-empty match ranges of `re` in one file line.
fn matches(re: &Regex, text: &str) -> Vec<Range<usize>> {
    re.find_iter(text)
        .map(|m| m.range())
        .filter(|r| !r.is_empty())
        .collect()
}

/// Overlays `finds` (sorted, disjoint) on the syntax spans of one line: the syntax spans are
/// cut around them, and the matches are added in `style`. The result stays sorted and disjoint,
/// which is all [`row_spans`] needs.
fn with_find(
    hl: &[(Style, Range<usize>)],
    finds: &[Range<usize>],
    style: Style,
) -> Vec<(Style, Range<usize>)> {
    let mut out: Vec<(Style, Range<usize>)> = Vec::with_capacity(hl.len() + finds.len() * 2);
    for (st, r) in hl {
        let mut pos = r.start;
        for f in finds.iter().filter(|f| f.end > r.start && f.start < r.end) {
            if pos < f.start {
                out.push((*st, pos..f.start));
            }
            pos = pos.max(f.end.min(r.end));
        }
        if pos < r.end {
            out.push((*st, pos..r.end));
        }
    }
    out.extend(finds.iter().map(|f| (style, f.clone())));
    out.sort_by_key(|(_, r)| r.start);
    out
}

fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

/// Largest byte index <= `max` that is a char boundary of `s`.
fn floor_boundary(s: &str, max: usize) -> usize {
    if s.len() <= max {
        return s.len();
    }
    let mut i = max;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Number of wrapped rows from `from` to `to`, or `None` when `to` is above `from`.
fn rows_between(app: &App, from: (usize, usize), to: (usize, usize)) -> Option<usize> {
    if to < from {
        return None;
    }
    let mut n = 0usize;
    let (mut l, mut r) = from;
    while (l, r) < to {
        r += 1;
        if r >= app.rows(l).len() {
            l += 1;
            r = 0;
        }
        n += 1;
        if n > 10_000 {
            return None;
        }
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::app::App;
    use crate::buffer::Buffer;
    use crate::tree::Tree;

    #[test]
    fn picker_overlay_renders_border_prompt_and_items() {
        let files = ["src/app.rs", "src/wrap.rs"].map(PathBuf::from).to_vec();
        let mut app = App::new(
            PathBuf::from("/demo"),
            Tree::default(),
            files,
            Buffer::empty(),
            None,
        );
        app.open_files_picker();
        app.picker.as_mut().unwrap().settle();

        let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

        let buf = terminal.backend().buffer();
        let text: Vec<String> = (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect();
        assert_eq!(text, SNAPSHOT);
    }

    /// A usages row is drawn with the colours of the file line it quotes: the `//!` comment
    /// in the row must not be painted like the `path:line:` prefix in front of it.
    #[test]
    fn hit_picker_rows_keep_the_syntax_colours_of_their_line() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut app = App::new(root, Tree::default(), Vec::new(), Buffer::empty(), None);
        let hits = vec![crate::search::Hit {
            path: PathBuf::from("src/wrap.rs"),
            line: 1,
            text: std::fs::read_to_string(app.root.join("src/wrap.rs"))
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .to_string(),
        }];
        assert!(hits[0].text.starts_with("//!"), "{:?}", hits[0].text);
        app.show_picker(crate::app::PickerKind::Usages, App::hit_items(hits));
        app.picker.as_mut().unwrap().settle();

        let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();

        let buf = terminal.backend().buffer();
        let row = 4;
        let x0 = (0..buf.area.width)
            .find(|&x| buf[(x, row)].symbol() == "s")
            .expect("row starts with src/wrap.rs");
        let prefix = "src/wrap.rs:1: ";
        let code_x = x0 + prefix.len() as u16;
        assert_eq!(buf[(code_x, row)].symbol(), "/");
        assert_ne!(buf[(code_x, row)].fg, buf[(x0, row)].fg);
    }

    /// The tree pane and the status bar frame the overlay; the overlay itself is the border,
    /// the `>` prompt and the two items.
    #[rustfmt::skip]
    const SNAPSHOT: [&str; 12] = [
        "┌demo────────────────────────┐ o: open file   ?: help",
        "│                            │",
        "│     ┌Files (2/2)───────────────────────────────────┐",
        "│     │>                                             │",
        "│     │src/app.rs                                    │",
        "│     │src/wrap.rs                                   │",
        "│     │                                              │",
        "│     │                                              │",
        "│     │                                              │",
        "│     └──────────────────────────────────────────────┘",
        "└────────────────────────────┘",
        "demo/  1:1  [tree]                                   ? help",
    ];

    #[test]
    fn selection_background_covers_partial_edges_and_pads_inner_lines() {
        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::new(
            PathBuf::from("/demo"),
            Tree::default(),
            Vec::new(),
            Buffer::from_bytes(PathBuf::from("/demo/f.txt"), b"zz\nabcd\nef\nghij\n"),
            None,
        );
        app.show_tree = false;
        for (code, m) in [
            (KeyCode::Down, KeyModifiers::NONE),
            (KeyCode::Right, KeyModifiers::NONE),
            (KeyCode::Right, KeyModifiers::NONE),
            (KeyCode::Down, KeyModifiers::SHIFT),
            (KeyCode::Down, KeyModifiers::SHIFT),
        ] {
            app.key(KeyEvent::new(code, m));
        }
        let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(12, 5)).unwrap();
        terminal.draw(|f| super::draw(f, &mut app, &theme)).unwrap();
        let buf = terminal.backend().buffer();
        // Gutter is two cells; `#` marks a cell with the selection background. The line above
        // the selection stays clean.
        let rows: Vec<String> = (0..4)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| {
                        if buf[(x, y)].bg == theme.selection {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect();
        assert_eq!(
            rows,
            [
                "............",
                "....########",
                "..##########",
                "..##........"
            ]
        );
    }

    #[test]
    fn find_spans_cut_the_syntax_spans_they_cover() {
        use ratatui::style::{Color, Style};

        let syntax = Style::new().fg(Color::Blue);
        let find = Style::new().bg(Color::Yellow);
        // "abcdefgh": one syntax span over 0..4, matches at 2..3 (inside it) and 5..6 (in the
        // gap the highlighter left behind).
        let out = super::with_find(&[(syntax, 0..4)], &[2..3, 5..6], find);
        assert_eq!(
            out,
            [(syntax, 0..2), (find, 2..3), (syntax, 3..4), (find, 5..6),]
        );
        // Matches covering a whole span replace it.
        assert_eq!(
            super::with_find(&[(syntax, 0..4)], &[0..2, 2..6], find),
            [(find, 0..2), (find, 2..6)]
        );
        assert_eq!(
            super::with_find(&[(syntax, 0..4)], &[], find),
            [(syntax, 0..4)]
        );
    }

    #[test]
    fn gutter_width() {
        assert_eq!(super::digits(0), 1);
        assert_eq!(super::digits(9), 1);
        assert_eq!(super::digits(10), 2);
        assert_eq!(super::digits(999), 3);
        assert_eq!(super::digits(1000), 4);
    }
}
