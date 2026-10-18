use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use unicode_segmentation::UnicodeSegmentation;

use crate::app::{App, Focus, Mode, PickerKind};
use crate::buffer::{Buffer, Spans};
use crate::git::{Mark, Review};
use crate::picker::{PickItem, Picker, Row};
use crate::search::MAX_HITS;
use crate::theme::Theme;
use crate::wrap;

use super::expand;
use super::status::draw_prompt;

#[expect(
    clippy::string_slice,
    reason = "row is a row of wrap_line over action, on cluster boundaries"
)]
pub(super) fn draw_help(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect, base: Style) {
    let keys = crate::app::KEYS;
    let key_w = keys.iter().map(|(k, _, _)| k.len()).max().unwrap_or(0);
    let lead = 3 + key_w + 2;
    let widest = keys
        .iter()
        .map(|(_, a, g)| (lead + wrap::width(a)).max(1 + g.len()))
        .max()
        .unwrap_or(0);
    let w = u16::try_from(widest).unwrap_or(u16::MAX) + 1;
    let [area] = Layout::horizontal([Constraint::Length((w + 4).min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let room = (area.width as usize).saturating_sub(2 + lead + 1);
    let bold = base.add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line> = Vec::new();
    let mut group = "";
    for (key, action, g) in keys {
        if *g != group {
            group = g;
            lines.push(Line::styled(format!(" {group}"), bold.fg(theme.accent)));
        }
        for (i, row) in wrap::wrap_line(action, room).into_iter().enumerate() {
            let key = if i == 0 { *key } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("   {key:key_w$}  "), bold),
                Span::styled(action[row].trim_end(), base.fg(theme.ghost_fg)),
            ]));
        }
    }
    let [area] = Layout::vertical([Constraint::Length(
        (u16::try_from(lines.len()).unwrap_or(u16::MAX) + 2).min(area.height),
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
            .scroll((u16::try_from(app.help_top).unwrap_or(u16::MAX), 0)),
        inner,
    );
}

#[expect(
    clippy::string_slice,
    reason = "fits is the range wrap::cut returns, on cluster boundaries"
)]
pub(super) fn draw_tree(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    base: Style,
) -> Option<usize> {
    let accent = base.fg(theme.accent);
    let block = tree_block(app, theme, area, base);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let focused = app.focus == Focus::Tree;
    let width = inner.width as usize;
    let review = app.review.as_ref();
    app.tree.fold(|n, depth| {
        let need = wrap::width(&n.name()).min(NAME_ROOM);
        (need + used(review, n, depth)).saturating_sub(width)
    });
    let visible = app.tree.visible();
    let height = inner.height as usize;
    let shown = app.tree.shown();
    let at = visible.iter().position(|&i| i == shown).unwrap_or(0);
    app.tree_top = app.tree_top.min(at).max((at + 1).saturating_sub(height));

    let mut cut_any = false;
    let mut need = 0;
    let rows: Vec<Line> = visible
        .iter()
        .skip(app.tree_top)
        .take(height)
        .map(|&i| {
            let n = &app.tree.nodes[i];
            let depth = app.tree.drawn_depth(i);
            let full = app.tree.row_name(i);
            let room = width.saturating_sub(used(app.review.as_ref(), n, depth));
            let cut = wrap::width(&full) > room;
            let joined = full.contains('/');
            cut_any |= cut;
            for j in app.tree.row_nodes(i) {
                let m = &app.tree.nodes[j];
                need = need.max(used(app.review.as_ref(), m, m.depth) + wrap::width(&m.name()));
            }
            let skipped = (review.and_then(|r| r.file(&n.path)))
                .is_some_and(|f| f.agent_skipped && !app.viewed.contains_key(&n.path));
            let look = std::env::var("MERL_SKIP_LOOK").unwrap_or_default();
            let row = if n.ignored || skipped && look != "robot" {
                base.fg(theme.ghost_fg)
            } else if n.is_dir {
                accent
            } else {
                base
            };
            let style = if i != shown {
                row
            } else if focused || review.is_some() {
                theme.selected(row)
            } else {
                row.bg(theme.line_hl_dim)
            };
            let noted = !n.is_dir && app.notes_in(&n.path) > 0;
            let tick = match look.as_str() {
                _ if !skipped || noted => tree_tick(app, &n.path, noted),
                "robot" | "robot-dim" => "\u{1f916}",
                "tick" => "\u{2713} ",
                _ => "  ",
            };
            let tick_fg = if skipped { theme.ghost_fg } else { theme.accent };
            let width = width.saturating_sub(wrap::width(tick));
            let mut spans = vec![
                Span::styled(tick, style.fg(tick_fg)),
                Span::styled("  ".repeat(depth), style),
            ];
            if let Some(f) = app.review.as_ref().and_then(|r| r.file(&n.path)) {
                let dim = if app.review_panel_colours {
                    style.fg(theme.ghost_fg)
                } else {
                    style
                };
                let counts = counts(f);
                let used = 2 * depth + 2;
                let mut name = full;
                if cut {
                    let fits = wrap::cut(&name, 0, room.saturating_sub(1)).0;
                    name = format!("{}\u{2026}", &name[fits]);
                }
                let gap = width.saturating_sub(used + wrap::width(&name) + wrap::width(&counts));
                spans.extend([
                    Span::styled(f.status.to_string(), style),
                    Span::styled(format!(" {name}{}", " ".repeat(gap)), style),
                    Span::styled(counts, dim),
                ]);
            } else {
                let marker = match (n.is_dir, n.expanded, app.marks.get(&n.path)) {
                    (true, true, _) => '\u{25be}',
                    (true, false, _) => '\u{25b8}',
                    (false, _, _) if noted => '\u{270e}',
                    (false, _, Some(&mark)) => mark,
                    (false, _, None) => ' ',
                };
                let name = if cut && joined {
                    cut_at_slash(&full, room)
                } else {
                    full
                };
                spans.push(Span::styled(format!("{marker} {name}"), style));
            }
            let used: usize = spans[1..].iter().map(|s| wrap::width(&s.content)).sum();
            spans.push(Span::styled(" ".repeat(width.saturating_sub(used)), style));
            Line::from(spans)
        })
        .collect();
    frame.render_widget(Paragraph::new(rows).style(base), inner);
    cut_any.then_some(need)
}

fn tree_block(app: &App, theme: &Theme, area: Rect, base: Style) -> Block<'static> {
    let accent = base.fg(theme.accent);
    let title = match &app.review {
        Some(r) => format!("{} \u{2190} {}", r.branch, r.base),
        None => app.root_name(),
    };
    let mut block = Block::bordered()
        .title(Span::styled(title, accent.add_modifier(Modifier::BOLD)))
        .border_style(accent)
        .style(base);
    if let Some(r) = app.review.as_ref().filter(|_| app.review_panel_colours) {
        let (added, deleted) = r
            .files
            .iter()
            .fold((0, 0), |(a, d), f| (a + f.added, d + f.deleted));
        let n = r.files.len();
        let s = crate::app::plural(n);
        let notes = app.notes.list.len();
        let notes = match notes {
            0 => String::new(),
            1 => " \u{b7} 1 note".to_owned(),
            k => format!(" \u{b7} {k} notes"),
        };
        let totals = format!(" {n} file{s} \u{b7} +{added} \u{2212}{deleted}{notes} ");
        if wrap::width(&totals) <= area.width.saturating_sub(2) as usize {
            block = block.title_bottom(Span::styled(totals, base.fg(theme.ghost_fg)));
        }
    }
    block
}

fn tree_tick(app: &App, path: &Path, noted: bool) -> &'static str {
    match (&app.review, app.viewed.contains_key(path)) {
        (Some(_), _) if noted => "\u{270e} ",
        (None, _) => "",
        (Some(_), false) => "  ",
        (Some(_), true) => "\u{2713} ",
    }
}

const NAME_ROOM: usize = 12;

fn counts(f: &crate::git::ReviewFile) -> String {
    if f.binary {
        "bin".to_owned()
    } else {
        format!("+{} \u{2212}{}", f.added, f.deleted)
    }
}

fn used(review: Option<&Review>, n: &crate::tree::Node, depth: usize) -> usize {
    match review {
        None => 2 * depth + 2,
        Some(r) => 2 + 2 * depth + 2 + r.file(&n.path).map_or(0, |f| wrap::width(&counts(f)) + 1),
    }
}

fn cut_at_slash(path: &str, room: usize) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    (1..parts.len())
        .map(|k| format!("\u{2026}/{}", parts[k..].join("/")))
        .find(|t| wrap::width(t) <= room)
        .unwrap_or_else(|| parts[parts.len() - 1].to_owned())
}

pub(super) fn draw_picker(
    frame: &mut Frame,
    app: &mut App,
    theme: &Theme,
    area: Rect,
    share: u16,
    base: Style,
) {
    let pending = app.search_pending();
    let search = app.mode == Mode::Picker(PickerKind::Search);
    let Some(picker) = &mut app.picker else {
        return;
    };
    let [area] = Layout::horizontal([Constraint::Percentage(share)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(60)])
        .flex(Flex::Center)
        .areas(area);

    let accent = base.fg(theme.accent);
    let block = Block::bordered()
        .title(picker_title(picker, pending))
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
    app.cursor_cell = Some(draw_prompt(frame, prefix, &picker.query, base, prompt));

    let width = list.width as usize;
    let files = app.mode == Mode::Picker(PickerKind::Files);
    let review = app.review.as_ref();
    let letters = review.filter(|_| files && app.review_open_files_first);
    let marks = review.filter(|_| app.review_list_marks && lists_marks(app.mode));
    let grouped = matches!(
        app.mode,
        Mode::Picker(PickerKind::Usages | PickerKind::Search | PickerKind::Definitions)
    );
    let height = list.height as usize;
    let prefix_w = usize::from(marks.is_some());
    let (rows, places, selected) = if grouped && height > 1 {
        group_window(picker, height, width, prefix_w)
    } else {
        let (rows, selected) = picker.window(height);
        let places = rows.iter().map(|_| None).collect();
        (rows, places, selected)
    };
    let mut lines: Vec<Line> = Vec::new();
    let mut headers = Vec::new();
    for (i, (row, place)) in rows.iter().zip(&places).enumerate() {
        let base = if files && app.ignored.contains(&row.item.path) {
            base.fg(theme.ghost_fg)
        } else {
            base
        };
        let style = if i == selected {
            theme.selected(base)
        } else {
            base
        };
        let prefix = row_prefix(
            (letters, marks),
            &mut picker.marks,
            &app.root,
            &row.item,
            style,
        );
        let code = code_hl(&mut picker.bufs, &app.root, review, &row.item, theme);
        let lit = !(i == selected && theme.selection_fg.is_some());
        let cells = row_cells(row, code, (lit, i == selected), style);
        let Some(place) = place else {
            let at = picker.selected + i - selected;
            lines.push(tagged_line(
                &picker.labels,
                at,
                prefix,
                cells,
                (style, theme),
                width,
            ));
            continue;
        };
        if let Some(header) = place_header(place, &cells, base, theme, (width, prefix_w)) {
            lines.push(header);
            headers.push(lines.len());
        }
        lines.extend(place_rows(
            place,
            &cells,
            prefix,
            row.item.line,
            (style, theme),
            width,
        ));
    }
    lines.truncate(height);
    if headers.contains(&lines.len()) {
        lines.pop();
    }
    frame.render_widget(Paragraph::new(lines).style(base), list);
}

fn lists_marks(mode: Mode) -> bool {
    matches!(
        mode,
        Mode::Picker(
            PickerKind::Usages | PickerKind::Search | PickerKind::Definitions | PickerKind::Symbols
        )
    )
}

fn row_prefix(
    (letters, marks): (Option<&Review>, Option<&Review>),
    picker_marks: &mut HashMap<PathBuf, HashMap<usize, Mark>>,
    root: &Path,
    item: &PickItem,
    style: Style,
) -> Vec<Span<'static>> {
    let mut prefix = Vec::new();
    if let Some(r) = letters {
        prefix.push(match r.file(&item.path) {
            Some(f) => Span::styled(format!("{} ", f.status), style),
            None => Span::styled("  ", style),
        });
    }
    if let Some(r) = marks {
        let colour = if item.deleted {
            Some(Color::Red)
        } else {
            match review_mark(picker_marks, r, root, item) {
                Some(Mark::Added) => Some(Color::Green),
                Some(Mark::Changed) => Some(Color::Blue),
                _ => None,
            }
        };
        prefix.push(match colour {
            Some(c) => Span::styled("\u{258e}", style.fg(c)),
            None => Span::styled(" ", style),
        });
    }
    prefix
}

fn tagged_line<'a>(
    labels: &[(usize, &'static str)],
    at: usize,
    prefix: Vec<Span<'a>>,
    cells: Vec<(usize, Span<'a>)>,
    (style, theme): (Style, &Theme),
    width: usize,
) -> Line<'a> {
    let tag = labels.iter().find(|(row, _)| *row == at).map(|t| t.1);
    let tag_w = tag.map_or(0, |t| wrap::width(t) + 1);
    let mut line = padded(
        prefix.into_iter().chain(cells.into_iter().map(|(_, s)| s)),
        width.saturating_sub(tag_w),
        style,
    );
    if let Some(t) = tag {
        line.spans
            .push(Span::styled(format!("{t} "), style.fg(theme.ghost_fg)));
    }
    line
}

fn picker_title(picker: &Picker, pending: bool) -> String {
    let (matched, total) = picker.counts();
    if picker.live && pending {
        format!("{} (…)", picker.title)
    } else if picker.live && picker.query.is_empty() && total > 0 {
        format!("{} (first {total}, type to search all)", picker.title)
    } else if picker.live {
        let more = if total as usize >= MAX_HITS { "+" } else { "" };
        let s = crate::app::plural(total as usize);
        format!("{} ({total}{more} hit{s})", picker.title)
    } else {
        format!("{} ({matched}/{total})", picker.title)
    }
}

fn row_cells<'a>(
    row: &Row,
    code: Option<(&Spans, usize)>,
    (lit, selected): (bool, bool),
    style: Style,
) -> Vec<(usize, Span<'a>)> {
    let mut c = 0;
    row.item
        .label
        .grapheme_indices(true)
        .map(|(b, g)| {
            let chars = c..c + u32::try_from(g.chars().count()).unwrap_or(u32::MAX);
            c = chars.end;
            let mut style = style;
            if let (Some((hl, off)), Some(at)) = (&code, row.item.code_at)
                && lit
                && b >= at
                && let Some((s, _)) = hl.iter().find(|(_, r)| r.contains(&(b - at + off)))
            {
                style = style.patch(if selected {
                    Style { bg: None, ..*s }
                } else {
                    *s
                });
            }
            if chars
                .into_iter()
                .any(|k| row.matched.binary_search(&k).is_ok())
            {
                style = style.add_modifier(Modifier::BOLD);
            }
            (b, Span::styled(expand(g).into_owned(), style))
        })
        .collect()
}

fn bold(s: &Span) -> bool {
    s.style.add_modifier.contains(Modifier::BOLD)
}

fn place_header<'a>(
    place: &Place,
    cells: &[(usize, Span<'a>)],
    base: Style,
    theme: &Theme,
    (width, prefix_w): (usize, usize),
) -> Option<Line<'a>> {
    if !place.header {
        return None;
    }
    let lead = Span::styled(" ".repeat(prefix_w + place.head_w), base);
    let path = cells
        .iter()
        .filter(|(b, _)| place.path.contains(b))
        .map(|(_, s)| {
            let accent = base.fg(theme.accent);
            if bold(s) {
                Span::styled(s.content.clone(), accent.add_modifier(Modifier::BOLD))
            } else {
                Span::styled(s.content.clone(), accent)
            }
        });
    Some(padded(std::iter::once(lead).chain(path), width, base))
}

fn place_rows<'a>(
    place: &Place,
    cells: &[(usize, Span<'a>)],
    prefix: Vec<Span<'a>>,
    line: usize,
    (style, theme): (Style, &Theme),
    width: usize,
) -> Vec<Line<'a>> {
    let code_at = place.code_at;
    let matched_num = cells
        .iter()
        .any(|(b, s)| (place.path.end..code_at).contains(b) && bold(s));
    let num_style = if matched_num {
        style.fg(theme.ghost_fg).add_modifier(Modifier::BOLD)
    } else {
        style.fg(theme.ghost_fg)
    };
    let num = Span::styled(format!("  {:>w$}  ", line, w = place.num_w), num_style);
    let in_row = |r: &std::ops::Range<usize>| {
        cells
            .iter()
            .filter(|(b, _)| *b >= code_at + r.start && *b < code_at + r.end)
            .map(|(_, s)| s.clone())
            .collect::<Vec<_>>()
    };
    let head = cells
        .iter()
        .filter(|(b, _)| *b < place.path.start)
        .map(|(_, s)| s.clone());
    let first = prefix
        .into_iter()
        .chain(head)
        .chain([num])
        .chain(in_row(&place.code_rows[0]));
    let mut lines = vec![padded(first, width, style)];
    for r in &place.code_rows[1..] {
        let lead = Span::styled(" ".repeat(place.code_col), style);
        lines.push(padded(std::iter::once(lead).chain(in_row(r)), width, style));
    }
    lines
}

fn padded<'a>(spans: impl IntoIterator<Item = Span<'a>>, width: usize, style: Style) -> Line<'a> {
    let mut spans: Vec<Span> = spans.into_iter().collect();
    let used: usize = spans.iter().map(|s| wrap::width(&s.content)).sum();
    spans.push(Span::styled(" ".repeat(width.saturating_sub(used)), style));
    Line::from(spans)
}

struct Place {
    header: bool,
    head_w: usize,
    path: std::ops::Range<usize>,
    code_at: usize,
    num_w: usize,
    code_col: usize,
    code_rows: Vec<std::ops::Range<usize>>,
}

impl Place {
    fn lines(&self) -> usize {
        usize::from(self.header) + self.code_rows.len()
    }
}

fn group_window(
    picker: &mut Picker,
    height: usize,
    width: usize,
    prefix_w: usize,
) -> (Vec<Row>, Vec<Option<Place>>, usize) {
    if picker.clamp_selected() == 0 {
        picker.first = 0;
        return (Vec::new(), Vec::new(), 0);
    }
    let height = height.max(1);
    picker.first = picker.first.clamp(
        (picker.selected + 1).saturating_sub(height),
        picker.selected,
    );
    loop {
        let rows = picker.rows(picker.first, height);
        let places = places(&rows, width, prefix_w);
        let selected = picker.selected - picker.first;
        let lines = |n: usize| -> usize {
            places[..n]
                .iter()
                .map(|p| p.as_ref().map_or(1, Place::lines))
                .sum()
        };
        if lines(selected + 1) <= height || picker.first == picker.selected {
            let shown = (0..=rows.len())
                .take_while(|&n| lines(n) <= height)
                .last()
                .unwrap_or(0);
            picker.set_page(shown);
            return (rows, places, selected);
        }
        picker.first += 1;
    }
}

fn places(rows: &[Row], width: usize, prefix_w: usize) -> Vec<Option<Place>> {
    let paths: Vec<Option<(std::ops::Range<usize>, usize)>> = rows
        .iter()
        .map(|r| {
            let (path, code_at) = (r.item.path_at.clone()?, r.item.code_at?);
            r.item.label.get(path.clone())?;
            (!path.is_empty() && path.end <= code_at).then_some((path, code_at))
        })
        .collect();
    let num_w = rows
        .iter()
        .zip(&paths)
        .filter(|(_, p)| p.is_some())
        .map(|(r, _)| r.item.line.to_string().len())
        .max()
        .unwrap_or(0);
    let mut last: Option<&str> = None;
    rows.iter()
        .zip(paths)
        .map(|(r, split)| {
            let label = &r.item.label;
            let Some((path, code_at, text, head, code)) = split.and_then(|(path, code_at)| {
                let text = label.get(path.clone())?;
                let head = label.get(..path.start)?;
                let code = label.get(code_at..)?;
                Some((path, code_at, text, head, code))
            }) else {
                last = None;
                return None;
            };
            let header = last != Some(text);
            last = Some(text);
            let head_w = wrap::width(head);
            let code_col = prefix_w + head_w + 2 + num_w + 2;
            let room = width.saturating_sub(code_col);
            let code_rows = if wrap::width(code) > room && room >= MIN_WRAP {
                wrap::wrap_line(code, room)
            } else {
                std::iter::once(0..code.len()).collect()
            };
            Some(Place {
                header,
                head_w,
                path,
                code_at,
                num_w,
                code_col,
                code_rows,
            })
        })
        .collect()
}

const MIN_WRAP: usize = 20;

fn review_mark(
    marks: &mut HashMap<PathBuf, HashMap<usize, Mark>>,
    review: &Review,
    root: &Path,
    item: &PickItem,
) -> Option<Mark> {
    let file = review.file(&item.path)?;
    let marks = marks
        .entry(item.path.clone())
        .or_insert_with(|| review.diff(root, &root.join(&item.path), Some(file)).marks);
    marks.get(&item.line.checked_sub(1)?).copied()
}

fn code_hl<'a>(
    bufs: &'a mut HashMap<PathBuf, Buffer>,
    root: &Path,
    review: Option<&Review>,
    item: &PickItem,
    theme: &Theme,
) -> Option<(&'a Spans, usize)> {
    let quoted = item
        .label
        .get(item.code_at?..)?
        .trim_end_matches('\u{2026}');
    let key = if item.deleted {
        item.path.join("\0base")
    } else {
        item.path.clone()
    };
    let buf = bufs
        .entry(key)
        .or_insert_with(|| match (item.deleted, review) {
            (true, Some(r)) => {
                let old = r.file(&item.path).and_then(|f| f.old.clone());
                let bytes = r
                    .base_bytes(root, old.as_deref().unwrap_or(&item.path))
                    .unwrap_or_default();
                Buffer::from_bytes(root.join(&item.path), &bytes)
            }
            _ => Buffer::load(&root.join(&item.path)).unwrap_or_else(|_| Buffer::empty()),
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

pub(super) fn lesson_panel(
    app: &App,
    theme: &Theme,
    width: u16,
    base: Style,
) -> Option<Paragraph<'static>> {
    let tutor = app.tutor.as_ref()?;
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
        (None, None) => (
            " Tutor \u{2713} done".to_owned(),
            crate::tutor::ALL_LESSONS_DONE,
        ),
    };
    let pad = (width as usize).saturating_sub(wrap::width(&title));
    let head = Style::new()
        .bg(theme.status_bg)
        .fg(theme.status_fg)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(Span::styled(format!("{title}{}", " ".repeat(pad)), head)),
        Line::from(Span::styled(format!(" {text}"), base)),
    ];
    Some(Paragraph::new(lines).wrap(Wrap { trim: true }).style(base))
}
