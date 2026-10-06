use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use ratatui::style::{Color, Modifier, Style};
use syntect::highlighting::Highlighter;
use syntect::parsing::Scope;
use unicase::UniCase;

use crate::theme::Theme;
use crate::wrap;

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

/// What a piece of a row is drawn as: a colour of the theme and the modifiers over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub ink: Ink,
    pub mods: Modifier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Text,
    Heading,
    /// Inline code and code blocks, on a tint.
    Code,
    Link,
    Quote,
    /// What is shown as it is written: front matter, HTML, an image's alt text.
    Dim,
    /// Bullets, numbers and checkboxes.
    Bullet,
    /// Rules, table borders and the bar left of a quote.
    Line,
    /// A GitHub alert's bar and title.
    Alert(BlockQuoteKind),
}

impl Ink {
    fn plain(self) -> Look {
        Look {
            ink: self,
            mods: Modifier::empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub text: String,
    /// Looks over byte ranges of `text`, in order and disjoint.
    pub looks: Vec<(Look, Range<usize>)>,
    /// Where the row starts in the source: 0-based line and byte column. A column past the end
    /// of the line (a rule under a heading, the blank row after a block) is the line's end.
    pub src: (usize, usize),
    /// The source lines the row shows, which its git marks come from and a position is found by:
    /// a reflowed row can hold the end of one line and the start of the next, and a code block's
    /// rows its fences. Empty for a row drawn for no line of its own (a table's border, the rule
    /// under a heading).
    pub lines: Range<usize>,
    /// Lines no row shows that go with this one, for the same: a reference definition, a second
    /// blank line, a setext underline.
    pub owns: Vec<usize>,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    /// The blank row between two blocks: where `{` and `}` stop.
    Gap,
    /// A row of a code block: which block of [`Doc::code`], which of its lines, where in that
    /// line the row starts, and where in `text` it is drawn. The tint runs to the pane's edge.
    Code {
        block: usize,
        line: usize,
        from: usize,
        at: usize,
    },
}

/// A code block's info string and its lines, tabs drawn as spaces: highlighted as it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Code {
    pub lang: String,
    pub lines: Vec<String>,
}

/// A Markdown file laid out for one width.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    /// Never empty: an empty file is one blank row.
    pub rows: Vec<Row>,
    pub code: Vec<Code>,
}

impl Doc {
    /// The row a source position is shown on: of the rows that show its line, the last that
    /// starts at or before its column (the first of those that start together), or the first of
    /// them when the column comes before the text, on a list item's or a heading's marker; else
    /// the row that owns the line, which every line has.
    pub fn row_at(&self, pos: (usize, usize)) -> usize {
        let mut first = None;
        let mut best: Option<usize> = None;
        for (i, r) in self.rows.iter().enumerate() {
            if !r.lines.contains(&pos.0) {
                continue;
            }
            first.get_or_insert(i);
            if r.src <= pos && best.is_none_or(|b| r.src > self.rows[b].src) {
                best = Some(i);
            }
        }
        best.or(first)
            .or_else(|| self.rows.iter().position(|r| r.owns.contains(&pos.0)))
            .unwrap_or(self.rows.len() - 1)
    }

    /// The row a laid out anew doc shows `row` of this one on: the same row where rows share a
    /// position (a table's border and header, a heading and its rule), else the row of `pos`.
    pub fn same_row(&self, old: &Doc, row: usize, pos: (usize, usize)) -> usize {
        let src = old.rows[row].src;
        let rank = old.rows[..row].iter().filter(|r| r.src == src).count();
        let same: Vec<usize> = (0..self.rows.len())
            .filter(|&i| self.rows[i].src == src)
            .collect();
        match same.get(rank).or(same.last()) {
            Some(&i) => i,
            None => self.row_at(pos),
        }
    }
}

pub fn layout(lines: &[String], width: usize) -> Doc {
    let src = lines.join("\n");
    let mut starts = vec![0];
    starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
    let mut lay = Lay {
        src: &src,
        starts,
        width: width.max(1),
        ..Default::default()
    };
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    for (ev, r) in Parser::new_ext(&src, opts).into_offset_iter() {
        lay.event(ev, r);
    }
    lay.flush();
    let mut rows = lay.rows;
    if rows.is_empty() {
        rows.push(Row {
            text: String::new(),
            looks: Vec::new(),
            src: (0, 0),
            lines: 0..0,
            owns: Vec::new(),
            kind: Kind::Gap,
        });
    }
    cover(&mut rows, lines.len());
    Doc {
        rows,
        code: lay.code,
    }
}

/// Gives every source line no row shows to a row, for its marks and for finding the row of a
/// position: to the row that shows or owns the line above it, and above every row to the row
/// that shows the next line. With no row that shows a line, the first row takes them all.
fn cover(rows: &mut [Row], n: usize) {
    let (mut first, mut last) = (vec![None; n], vec![None; n]);
    for (i, r) in rows.iter().enumerate() {
        for l in r.lines.clone().filter(|&l| l < n) {
            first[l].get_or_insert(i);
            last[l] = Some(i);
        }
    }
    let (mut above, mut pending) = (None, Vec::new());
    for l in 0..n {
        if let Some(i) = first[l] {
            rows[i].owns.append(&mut pending);
            above = last[l];
        } else if let Some(i) = above {
            rows[i].owns.push(l);
        } else {
            pending.push(l);
        }
    }
    rows[0].owns.append(&mut pending);
}

/// A container the rows inside it are drawn in.
enum Frame {
    Quote(Option<BlockQuoteKind>),
    /// The next number of an ordered list, and whether its items are paragraphs apart.
    List(Option<u64>, bool),
    /// A list item or a footnote: the marker its first row still has to show, and how wide the
    /// marker is, which the rows after the first are indented by.
    Item(Option<(String, Look)>, usize),
}

/// A paragraph, heading or table cell being read: its text, the looks over it, and where each
/// piece came from in the source.
#[derive(Default)]
struct Inline {
    text: String,
    looks: Vec<(Look, Range<usize>)>,
    /// (byte in `text`, source bytes) of each piece.
    map: Vec<(usize, Range<usize>)>,
}

impl Inline {
    /// Adds the text `s` of an event whose source is `raw`, starting at byte `at`. Text written
    /// as it is (code between its backticks too) maps byte for byte, its tabs to themselves;
    /// text the parser changed (an escape, an entity) maps to the event as a whole.
    fn push(&mut self, s: &str, look: Look, raw: &str, at: usize) {
        let start = self.text.len();
        let literal = raw.find(s).map(|i| at + i);
        let mut o = 0;
        for (k, part) in s.split('\t').enumerate() {
            if k > 0 {
                let tab = literal.map_or(at..at + raw.len(), |b| b + o - 1..b + o);
                self.map.push((self.text.len(), tab));
                self.text.push_str(crate::buffer::TAB);
            }
            if !part.is_empty() {
                let src = literal.map_or(at..at + raw.len(), |b| b + o..b + o + part.len());
                self.map.push((self.text.len(), src));
                self.text.push_str(part);
            }
            o += part.len() + 1;
        }
        let end = self.text.len();
        match self.looks.last_mut() {
            Some((l, r)) if *l == look && r.end == start => r.end = end,
            _ => self.looks.push((look, start..end)),
        }
    }

    /// The source byte text byte `i` came from, as near as the piece holding it says.
    fn src_at(&self, i: usize) -> usize {
        let k = self.map.partition_point(|(t, _)| *t <= i).saturating_sub(1);
        self.map
            .get(k)
            .map_or(0, |(t, r)| (r.start + (i - t)).min(r.end))
    }
}

/// A code block being read.
struct Block {
    /// The first word of the info string.
    lang: String,
    /// Its lines as written, each with the source position it starts at and how many bytes in
    /// front of it the parser added, the spaces of a tab a container took part of.
    lines: Vec<(String, (usize, usize), usize)>,
    /// The last line has not had its `\n` yet.
    open: bool,
    /// The source lines of the block, its fences included.
    span: Range<usize>,
}

struct Table {
    aligns: Vec<Alignment>,
    /// The source line of each row, the header first, and its cells.
    rows: Vec<(usize, Vec<Inline>)>,
}

#[derive(Default)]
struct Lay<'a> {
    src: &'a str,
    /// The byte each source line starts at.
    starts: Vec<usize>,
    width: usize,
    rows: Vec<Row>,
    code: Vec<Code>,
    stack: Vec<Frame>,
    /// A blank row goes before the next block.
    gap: bool,
    inline: Option<Inline>,
    /// `inline` is a tight list item's text, which comes with no paragraph around it.
    implicit: bool,
    bold: usize,
    italic: usize,
    strike: usize,
    link: usize,
    image: usize,
    heading: Option<HeadingLevel>,
    table: Option<Table>,
    block: Option<Block>,
    /// Inside front matter, which is drawn from its source lines.
    meta: bool,
    /// Footnote labels, numbered as they are first met, and matched as the parser matches a
    /// reference to its note: case folded.
    numbers: HashMap<UniCase<String>, usize>,
}

impl Lay<'_> {
    fn event(&mut self, ev: Event, r: Range<usize>) {
        match ev {
            Event::Start(tag) => self.start(tag, r),
            Event::End(tag) => self.end(tag, r),
            Event::Text(_) if self.meta => {}
            Event::Text(t) if self.block.is_some() => {
                // Code comes as text that ends its lines with `\n`: a block at the top in one
                // event, one inside a container an event a line, and the spaces of a tab the
                // container took part of as an event of their own in front of the line.
                let mut off = 0;
                for piece in t.split_inclusive('\n') {
                    let at = self.pos((r.start + off).min(r.end));
                    off += piece.len();
                    let Some(b) = &mut self.block else {
                        return;
                    };
                    let text = piece.strip_suffix('\n').unwrap_or(piece);
                    // Text with no source bytes is the parser's own.
                    let added = if r.is_empty() { text.len() } else { 0 };
                    match b.lines.last_mut() {
                        Some((line, _, n)) if b.open => {
                            *n += if line.len() == *n { added } else { 0 };
                            line.push_str(text);
                        }
                        _ => b.lines.push((text.to_string(), at, added)),
                    }
                    b.open = !piece.ends_with('\n');
                }
            }
            Event::Text(t) => self.text(&t, self.look(), r),
            Event::Code(t) => {
                let look = Look {
                    ink: Ink::Code,
                    ..self.look()
                };
                self.text(&t, look, r);
            }
            Event::InlineMath(t) | Event::DisplayMath(t) => self.text(&t, self.look(), r),
            Event::Html(t) => {
                // A raw HTML block, a line an event: shown as it is written.
                let at = self.pos(r.start);
                let line = t.trim_end_matches(['\n', '\r']);
                self.lines(line, Ink::Dim.plain(), at);
            }
            Event::InlineHtml(t) => self.text(&t, Ink::Dim.plain(), r),
            Event::FootnoteReference(label) => {
                let n = self.number(&label);
                let look = Look {
                    ink: Ink::Link,
                    ..self.look()
                };
                self.text(&format!("[{n}]"), look, r);
            }
            Event::SoftBreak => self.text(" ", self.look(), r),
            Event::HardBreak => self.text("\n", self.look(), r),
            Event::Rule => {
                self.block_start(r.start);
                self.rule(self.pos(r.start));
                self.gap = true;
            }
            Event::TaskListMarker(done) => {
                if let Some(Frame::Item(marker, w)) = self.stack.last_mut() {
                    let bullet = if done { "\u{2611} " } else { "\u{2610} " };
                    *marker = Some((bullet.into(), Ink::Bullet.plain()));
                    *w = 2;
                }
            }
        }
    }

    fn start(&mut self, tag: Tag, r: Range<usize>) {
        match tag {
            Tag::Paragraph => {
                self.block_start(r.start);
                // A list whose items hold paragraphs is loose: blank rows between its items.
                if let [.., Frame::List(_, loose), Frame::Item(..)] = &mut self.stack[..] {
                    *loose = true;
                }
                self.inline = Some(Inline::default());
            }
            Tag::Heading { level, .. } => {
                self.block_start(r.start);
                self.heading = Some(level);
                self.inline = Some(Inline::default());
            }
            Tag::BlockQuote(kind) => {
                self.block_start(r.start);
                self.stack.push(Frame::Quote(kind));
                if let Some(kind) = kind {
                    let title = match kind {
                        BlockQuoteKind::Note => "Note",
                        BlockQuoteKind::Tip => "Tip",
                        BlockQuoteKind::Important => "Important",
                        BlockQuoteKind::Warning => "Warning",
                        BlockQuoteKind::Caution => "Caution",
                    };
                    let look = Look {
                        ink: Ink::Alert(kind),
                        mods: Modifier::BOLD,
                    };
                    let at = self.pos(r.start);
                    self.push(title.into(), vec![(look, 0..title.len())], at, Kind::Text);
                }
            }
            Tag::CodeBlock(kind) => {
                self.block_start(r.start);
                // `rust,ignore` and `py title=x` name the language by their first word.
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .next()
                        .unwrap_or("")
                        .into(),
                    CodeBlockKind::Indented => String::new(),
                };
                let span = self.pos(r.start).0..self.pos(r.end.saturating_sub(1)).0 + 1;
                self.block = Some(Block {
                    lang,
                    lines: Vec::new(),
                    open: false,
                    span,
                });
            }
            Tag::HtmlBlock => self.block_start(r.start),
            Tag::List(first) => {
                self.block_start(r.start);
                self.stack.push(Frame::List(first, false));
            }
            Tag::Item => {
                self.flush_implicit();
                // Items of a tight list follow each other; a loose list's are paragraphs apart.
                let loose = matches!(self.stack.last(), Some(Frame::List(_, true)));
                self.gap &= loose;
                self.block_start(r.start);
                let depth = self
                    .stack
                    .iter()
                    .filter(|f| matches!(f, Frame::List(..)))
                    .count();
                let marker = match self.stack.last_mut() {
                    Some(Frame::List(Some(n), _)) => {
                        *n += 1;
                        format!("{}. ", *n - 1)
                    }
                    _ => match depth {
                        1 => "\u{2022} ",
                        2 => "\u{25e6} ",
                        _ => "\u{25aa} ",
                    }
                    .into(),
                };
                let w = wrap::width(&marker);
                self.stack
                    .push(Frame::Item(Some((marker, Ink::Bullet.plain())), w));
            }
            // A footnote is drawn where it is written, a block of its own under its label.
            Tag::FootnoteDefinition(label) => {
                self.block_start(r.start);
                let n = self.number(&label);
                let marker = format!("[{n}] ");
                let w = wrap::width(&marker);
                self.stack
                    .push(Frame::Item(Some((marker, Ink::Link.plain())), w));
            }
            Tag::Table(aligns) => {
                self.block_start(r.start);
                self.table = Some(Table {
                    aligns,
                    rows: Vec::new(),
                });
            }
            Tag::TableHead | Tag::TableRow => {
                let line = self.pos(r.start).0;
                if let Some(t) = &mut self.table {
                    t.rows.push((line, Vec::new()));
                }
            }
            Tag::TableCell => self.inline = Some(Inline::default()),
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { .. } => self.link += 1,
            Tag::Image { .. } => {
                // The picture cannot be drawn: its alt text stands in, marked.
                self.image += 1;
                self.text("\u{25a3} ", self.look(), r.start..r.start);
            }
            Tag::MetadataBlock(_) => {
                self.block_start(r.start);
                self.meta = true;
                let (first, last) = (self.pos(r.start).0, self.pos(r.end.saturating_sub(1)).0);
                for l in first..=last {
                    let text = self.line(l).to_string();
                    self.lines(&text, Ink::Dim.plain(), (l, 0));
                }
            }
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => {}
        }
    }

    fn end(&mut self, tag: TagEnd, r: Range<usize>) {
        match tag {
            TagEnd::Paragraph => {
                self.flush();
                self.gap = true;
            }
            TagEnd::Heading(level) => {
                let shown = self.flush();
                self.heading = None;
                // H1 and H2 are set apart by a rule, as GitHub underlines them.
                let rule = match level {
                    HeadingLevel::H1 => "\u{2501}",
                    HeadingLevel::H2 => "\u{2500}",
                    _ => "",
                };
                if shown && !rule.is_empty() {
                    let at = self.rows.last().map_or((0, 0), |r| r.src);
                    let text = rule.repeat(self.avail());
                    let n = text.len();
                    self.push(text, vec![(Ink::Line.plain(), 0..n)], at, Kind::Text);
                    self.shows_nothing();
                }
                // What a heading heads follows it without a blank row: the terminal has few.
                self.gap = false;
            }
            TagEnd::BlockQuote(_) | TagEnd::List(_) => {
                self.flush_implicit();
                self.stack.pop();
                self.gap = true;
            }
            TagEnd::Item => {
                self.flush_implicit();
                self.stack.pop();
            }
            TagEnd::FootnoteDefinition => {
                self.flush_implicit();
                // An empty note still shows its label.
                if let Some(Frame::Item(Some(_), _)) = self.stack.last() {
                    let at = self.pos(r.start);
                    self.push(String::new(), Vec::new(), at, Kind::Text);
                }
                self.stack.pop();
                self.gap = true;
            }
            TagEnd::CodeBlock => {
                if let Some(block) = self.block.take() {
                    self.code_block(block);
                }
                self.gap = true;
            }
            TagEnd::HtmlBlock => self.gap = true,
            TagEnd::MetadataBlock(_) => {
                self.meta = false;
                self.gap = true;
            }
            TagEnd::Table => {
                if let Some(t) = self.table.take() {
                    self.table(t);
                }
                self.gap = true;
            }
            TagEnd::TableCell => {
                let cell = self.inline.take().unwrap_or_default();
                if let Some((_, cells)) = self.table.as_mut().and_then(|t| t.rows.last_mut()) {
                    cells.push(cell);
                }
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => self.link = self.link.saturating_sub(1),
            TagEnd::Image => self.image = self.image.saturating_sub(1),
            TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::Superscript
            | TagEnd::Subscript => {}
        }
    }

    /// The look text read now gets from the tags around it.
    fn look(&self) -> Look {
        let plain_quote = self
            .stack
            .iter()
            .rev()
            .find_map(|f| match f {
                Frame::Quote(kind) => Some(kind.is_none()),
                _ => None,
            })
            .unwrap_or(false);
        let ink = if self.image > 0 {
            Ink::Dim
        } else if self.link > 0 {
            Ink::Link
        } else if self.heading.is_some() {
            Ink::Heading
        } else if plain_quote {
            Ink::Quote
        } else {
            Ink::Text
        };
        let mut mods = Modifier::empty();
        for (on, m) in [
            (self.bold > 0 || self.heading.is_some(), Modifier::BOLD),
            (self.italic > 0, Modifier::ITALIC),
            (self.strike > 0, Modifier::CROSSED_OUT),
            (self.link > 0 && self.image == 0, Modifier::UNDERLINED),
        ] {
            if on {
                mods |= m;
            }
        }
        Look { ink, mods }
    }

    fn text(&mut self, s: &str, look: Look, r: Range<usize>) {
        let raw = &self.src[r.clone()];
        let inline = self.inline.get_or_insert_with(|| {
            self.implicit = true;
            Inline::default()
        });
        inline.push(s, look, raw, r.start);
    }

    /// 0-based line and byte column of source byte `i`.
    fn pos(&self, i: usize) -> (usize, usize) {
        let l = self.starts.partition_point(|&s| s <= i).saturating_sub(1);
        (l, i - self.starts[l])
    }

    fn line(&self, l: usize) -> &str {
        let end = self.starts.get(l + 1).map_or(self.src.len(), |&s| s - 1);
        &self.src[self.starts[l]..end]
    }

    fn number(&mut self, label: &str) -> usize {
        let next = self.numbers.len() + 1;
        *self
            .numbers
            .entry(UniCase::new(label.into()))
            .or_insert(next)
    }

    /// Where a row that stands for no text of its own goes back to: the end of the line the row
    /// above it came from.
    fn after(&self) -> (usize, usize) {
        (self.rows.last().map_or(0, |r| r.src.0), usize::MAX)
    }

    /// What goes in front of a row: a bar per quote, the indent of each list item and, on the
    /// first row of an item, its marker (taken by that row when `first`).
    fn prefix(&mut self, first: bool) -> (String, Vec<(Look, Range<usize>)>) {
        let mut s = String::new();
        let mut looks = Vec::new();
        for f in &mut self.stack {
            let at = s.len();
            match f {
                Frame::Quote(kind) => {
                    s.push_str("\u{2502} ");
                    let ink = kind.map_or(Ink::Line, Ink::Alert);
                    looks.push((ink.plain(), at..at + "\u{2502}".len()));
                }
                Frame::Item(marker, w) => match marker.take_if(|_| first) {
                    Some((m, look)) => {
                        s.push_str(&m);
                        looks.push((look, at..s.len()));
                    }
                    None => s.push_str(&" ".repeat(*w)),
                },
                Frame::List(..) => {}
            }
        }
        (s, looks)
    }

    /// Columns left for text inside the containers open now.
    fn avail(&mut self) -> usize {
        let w = self
            .stack
            .iter()
            .map(|f| match f {
                Frame::Quote(_) => 2,
                Frame::Item(_, w) => *w,
                Frame::List(..) => 0,
            })
            .sum::<usize>();
        self.width.saturating_sub(w).max(1)
    }

    fn push(
        &mut self,
        text: String,
        looks: Vec<(Look, Range<usize>)>,
        src: (usize, usize),
        kind: Kind,
    ) {
        let (mut full, mut all) = self.prefix(kind != Kind::Gap);
        let at = full.len();
        full.push_str(&text);
        all.extend(
            looks
                .into_iter()
                .map(|(l, r)| (l, r.start + at..r.end + at)),
        );
        let kind = match kind {
            Kind::Code {
                block,
                line,
                from,
                at: a,
            } => Kind::Code {
                block,
                line,
                from,
                at: a + at,
            },
            k => k,
        };
        if kind == Kind::Gap {
            full.truncate(full.trim_end().len());
            all.retain(|(_, r)| r.end <= full.len());
        }
        let lines = match src {
            (l, usize::MAX) => l..l,
            (l, _) => l..l + 1,
        };
        self.rows.push(Row {
            text: full,
            looks: all,
            src,
            lines,
            owns: Vec::new(),
            kind,
        });
    }

    /// Before the block starting at source byte `next`: the text of a tight item is over, and
    /// the blank row a block before it asked for goes in. It stands for the blank line above the
    /// block, when the source has one.
    fn block_start(&mut self, next: usize) {
        self.flush_implicit();
        if std::mem::take(&mut self.gap) && !self.rows.is_empty() {
            let l = self.pos(next).0;
            let above = self.after();
            match l.checked_sub(1) {
                Some(b) if b > above.0 && self.line(b).trim().is_empty() => {
                    self.push(String::new(), Vec::new(), (b, 0), Kind::Gap);
                }
                // No blank line stands between the blocks: the blank row goes to the next one.
                _ => {
                    self.push(String::new(), Vec::new(), (l, 0), Kind::Gap);
                    self.shows_nothing();
                }
            }
        }
    }

    fn shows_nothing(&mut self) {
        if let Some(r) = self.rows.last_mut() {
            r.lines.end = r.lines.start;
        }
    }

    /// A tight item's text that made rows is followed by what the item holds next, with no
    /// blank row between; text of nothing but spaces makes no rows and changes nothing.
    fn flush_implicit(&mut self) {
        if self.implicit && self.flush() {
            self.gap = false;
        }
    }

    /// Lays the text read so far out: wrapped to the room the containers leave, a hard break
    /// starting a new row. Returns whether it made any rows.
    fn flush(&mut self) -> bool {
        self.implicit = false;
        let Some(inline) = self.inline.take() else {
            return false;
        };
        if inline.text.trim().is_empty() {
            return false;
        }
        let avail = self.avail();
        let mut from = 0;
        for seg in inline.text.split('\n') {
            for r in wrap::wrap_line(seg, avail) {
                let r = from + r.start..from + r.start + seg[r].trim_end_matches(' ').len();
                // A row of nothing but spaces, from a run wider than the pane, is no row.
                if r.is_empty() && !seg.is_empty() {
                    continue;
                }
                let (text, looks) = cut(&inline.text, &inline.looks, r.clone());
                let at = self.pos(inline.src_at(r.start));
                let last = self
                    .pos(inline.src_at(r.end.saturating_sub(1).max(r.start)))
                    .0;
                self.push(text, looks, at, Kind::Text);
                if let Some(row) = self.rows.last_mut() {
                    row.lines.end = row.lines.end.max(last + 1);
                }
            }
            from += seg.len() + 1;
        }
        true
    }

    /// Rows for a line shown as it is written, wrapped.
    fn lines(&mut self, raw: &str, look: Look, (l, c): (usize, usize)) {
        let line = raw.replace('\t', crate::buffer::TAB);
        let mut tabs = Tabs::new(raw);
        let avail = self.avail();
        for r in wrap::wrap_line(&line, avail) {
            let text = line[r.clone()].to_string();
            let n = text.len();
            let at = (l, c.saturating_add(tabs.raw(r.start)));
            self.push(text, vec![(look, 0..n)], at, Kind::Text);
        }
    }

    fn rule(&mut self, at: (usize, usize)) {
        let text = "\u{2500}".repeat(self.avail());
        let n = text.len();
        self.push(text, vec![(Ink::Line.plain(), 0..n)], at, Kind::Text);
    }

    /// A code block: every line on the tint, a column in from its edge, wrapped as the source
    /// wraps it, with the rows after the first under its indent. Its rows stand for its fences.
    fn code_block(&mut self, mut b: Block) {
        // An empty block's row goes back to its opening fence.
        if b.lines.is_empty() {
            b.lines.push((String::new(), (b.span.start, 0), 0));
        }
        let block = self.code.len();
        let first = self.rows.len();
        let room = self.avail().saturating_sub(2).max(1);
        let mut drawn = Vec::with_capacity(b.lines.len());
        for (k, (raw, (l, c), added)) in b.lines.iter().enumerate() {
            let line = raw.replace('\t', crate::buffer::TAB);
            let mut tabs = Tabs::new(raw);
            let indent = wrap::indent(&line, room);
            for (i, r) in wrap::wrap_line(&line, room).into_iter().enumerate() {
                let lead = if i == 0 { 1 } else { 1 + indent };
                let text = format!("{}{}", " ".repeat(lead), &line[r.clone()]);
                let n = text.len();
                let kind = Kind::Code {
                    block,
                    line: k,
                    from: r.start,
                    at: lead,
                };
                let col = tabs.raw(r.start).saturating_sub(*added);
                self.push(text, vec![(Ink::Code.plain(), 0..n)], (*l, c + col), kind);
            }
            drawn.push(line);
        }
        let rows = &mut self.rows;
        if let Some(r) = rows.get_mut(first) {
            r.lines.start = r.lines.start.min(b.span.start);
        }
        if let Some(r) = rows.last_mut() {
            r.lines.end = r.lines.end.max(b.span.end);
        }
        self.code.push(Code {
            lang: b.lang,
            lines: drawn,
        });
    }

    /// A table in box drawing, each column aligned as its `:---:` says. Wider than the room it
    /// has, the widest columns give up columns first and their cells wrap, so it stays whole.
    fn table(&mut self, t: Table) {
        let n = t
            .rows
            .iter()
            .map(|(_, cells)| cells.len())
            .max()
            .unwrap_or(0)
            .max(t.aligns.len());
        if n == 0 {
            return;
        }
        // Each column's widest cell, and its widest word: a path or a name reads best whole.
        let (mut natural, mut words) = (vec![1; n], vec![1; n]);
        for (_, cells) in &t.rows {
            for (j, c) in cells.iter().enumerate() {
                natural[j] = natural[j].max(wrap::width(&c.text));
                let word = c.text.split(' ').map(wrap::width).max().unwrap_or(0);
                words[j] = words[j].max(word);
            }
        }
        // `│ a │ b │`: a border and a space each side of every cell.
        let room = self.avail().saturating_sub(3 * n + 1);
        let widths = fit(&natural, &words, room);
        let border = |l: &str, m: &str, r: &str| {
            let cols: Vec<String> = widths.iter().map(|w| "\u{2500}".repeat(w + 2)).collect();
            format!("{l}{}{r}", cols.join(m))
        };
        let line = Ink::Line.plain();
        let first = t.rows.first().map_or(0, |(l, _)| *l);
        let top = border("\u{250c}", "\u{252c}", "\u{2510}");
        let len = top.len();
        self.push(top, vec![(line, 0..len)], (first, 0), Kind::Text);
        self.shows_nothing();
        let empty = Inline::default();
        for (i, (l, cells)) in t.rows.iter().enumerate() {
            // Every cell's rows: byte ranges of its text.
            let wrapped: Vec<Vec<Range<usize>>> = (0..n)
                .map(|j| {
                    let c = cells.get(j).unwrap_or(&empty);
                    wrap::wrap_line(&c.text, widths[j])
                        .into_iter()
                        .map(|r| r.start..r.start + c.text[r].trim_end_matches(' ').len())
                        .collect()
                })
                .collect();
            let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
            for k in 0..height {
                let mut text = String::new();
                let mut looks = Vec::new();
                for j in 0..n {
                    let bar = if j == 0 { "\u{2502} " } else { " \u{2502} " };
                    looks.push((
                        line,
                        text.len()..text.len() + "\u{2502}".len() + usize::from(j > 0),
                    ));
                    text.push_str(bar);
                    let c = cells.get(j).unwrap_or(&empty);
                    let r = wrapped[j].get(k).cloned().unwrap_or(0..0);
                    let (piece, mut piece_looks) = cut(&c.text, &c.looks, r);
                    if i == 0 {
                        for (look, _) in &mut piece_looks {
                            look.mods |= Modifier::BOLD;
                        }
                    }
                    let pad = widths[j].saturating_sub(wrap::width(&piece));
                    let before = match t.aligns.get(j) {
                        Some(Alignment::Right) => pad,
                        Some(Alignment::Center) => pad / 2,
                        _ => 0,
                    };
                    text.push_str(&" ".repeat(before));
                    let at = text.len();
                    text.push_str(&piece);
                    looks.extend(
                        piece_looks
                            .into_iter()
                            .map(|(lk, r)| (lk, r.start + at..r.end + at)),
                    );
                    text.push_str(&" ".repeat(pad - before));
                }
                let at = text.len();
                text.push_str(" \u{2502}");
                looks.push((line, at + 1..text.len()));
                self.push(text, looks, (*l, 0), Kind::Text);
            }
            if i == 0 {
                let mid = border("\u{251c}", "\u{253c}", "\u{2524}");
                let len = mid.len();
                self.push(mid, vec![(line, 0..len)], (l + 1, 0), Kind::Text);
            }
        }
        let bottom = border("\u{2514}", "\u{2534}", "\u{2518}");
        let len = bottom.len();
        let last = t.rows.last().map_or(0, |(l, _)| *l);
        self.push(bottom, vec![(line, 0..len)], (last, usize::MAX), Kind::Text);
    }
}

/// The columns of `natural` widths made to fit `room`: as they are when they fit. Else the
/// widest give way: every column is cut down to one width, as wide as leaves them all within
/// `room`, but none below its widest word while all the words fit, and what is left is handed back
/// a column each. At one column each they may still not fit.
fn fit(natural: &[usize], words: &[usize], room: usize) -> Vec<usize> {
    if natural.iter().sum::<usize>() <= room {
        return natural.to_vec();
    }
    let floor: Vec<usize> = match words.iter().sum::<usize>() <= room {
        true => words.to_vec(),
        false => vec![1; natural.len()],
    };
    let at = |c: usize| -> Vec<usize> {
        natural
            .iter()
            .zip(&floor)
            .map(|(&w, &f)| w.min(c).max(f.min(w)))
            .collect()
    };
    let max = natural.iter().copied().max().unwrap_or(1);
    let cap = (1..=max)
        .rev()
        .find(|&c| at(c).iter().sum::<usize>() <= room)
        .unwrap_or(1);
    let mut widths = at(cap);
    let mut left = room.saturating_sub(widths.iter().sum());
    for (w, &nat) in widths.iter_mut().zip(natural) {
        if left > 0 && nat > *w {
            *w += 1;
            left -= 1;
        }
    }
    widths
}

/// Maps bytes of a line with its tabs drawn as spaces back to the line as written, for the rows
/// of the line in order: each tab is passed once, however many rows the line wraps into.
struct Tabs {
    at: Vec<usize>,
    next: usize,
    shift: usize,
}

impl Tabs {
    fn new(raw: &str) -> Self {
        let at = raw.match_indices('\t').map(|(t, _)| t).collect();
        Self {
            at,
            next: 0,
            shift: 0,
        }
    }

    /// The byte of the line that byte `i` of it drawn came from; a byte inside a tab's spaces
    /// came from the tab. `i` never goes back.
    fn raw(&mut self, i: usize) -> usize {
        let tab = crate::buffer::TAB.len();
        while let Some(&t) = self.at.get(self.next) {
            let drawn = t + self.shift;
            if drawn >= i {
                break;
            }
            if i < drawn + tab {
                return t;
            }
            self.shift += tab - 1;
            self.next += 1;
        }
        i - self.shift
    }
}

/// `text[r]` with the looks over it, moved to start at 0.
fn cut(
    text: &str,
    looks: &[(Look, Range<usize>)],
    r: Range<usize>,
) -> (String, Vec<(Look, Range<usize>)>) {
    let out = looks
        .iter()
        .filter(|(_, l)| l.end > r.start && l.start < r.end)
        .map(|(look, l)| {
            (
                *look,
                l.start.max(r.start) - r.start..l.end.min(r.end) - r.start,
            )
        })
        .collect();
    (text[r].to_string(), out)
}

/// The colours a [`Look`] is drawn in, from the theme: the ones it gives Markdown's own scopes
/// where it gives them one, the editor's chrome otherwise.
pub struct Palette {
    text: Color,
    heading: Color,
    code: Color,
    link: Color,
    quote: Color,
    bullet: Color,
    line: Color,
    /// Note, Tip, Important, Warning, Caution.
    alerts: [Color; 5],
    pub code_bg: Color,
}

impl Palette {
    pub fn new(theme: &Theme) -> Self {
        let h = Highlighter::new(&theme.syntect);
        // The colour the theme paints `scopes` with in a Markdown file, when it is not the text's.
        let scoped = |scopes: &str, or: Color| {
            let stack: Vec<Scope> = scopes
                .split(' ')
                .filter_map(|s| Scope::new(s).ok())
                .collect();
            let style = h.style_for_stack(&stack);
            if Some(style.foreground) == theme.syntect.settings.foreground {
                return or;
            }
            crate::theme::style(style).fg.unwrap_or(or)
        };
        // GitHub's alert colours, Primer's dark or light ones as the theme is.
        let alerts = if theme.light {
            [0x0969da, 0x1a7f37, 0x8250df, 0x9a6700, 0xd1242f]
        } else {
            [0x4493f8, 0x3fb950, 0xab7df8, 0xd29922, 0xf85149]
        }
        .map(Color::from_u32);
        Self {
            text: theme.fg,
            heading: scoped(
                "text.html.markdown markup.heading.1.markdown entity.name.section.markdown",
                theme.accent,
            ),
            code: scoped("text.html.markdown markup.raw.inline.markdown", theme.fg),
            link: scoped(
                "text.html.markdown meta.link.inline.markdown string.other.link.title.markdown",
                theme.accent,
            ),
            quote: theme.ghost_fg,
            bullet: theme.accent,
            line: theme.own_gutter_fg,
            alerts,
            code_bg: theme.line_hl_dim,
        }
    }

    /// The style of `look`. Code is on its tint, unless the row it is on has a background of its
    /// own (`row_bg`: the cursor's, a review's).
    pub fn style(&self, look: Look, row_bg: bool) -> Style {
        let fg = match look.ink {
            Ink::Text => self.text,
            Ink::Heading => self.heading,
            Ink::Code => self.code,
            Ink::Link => self.link,
            Ink::Quote | Ink::Dim => self.quote,
            Ink::Bullet => self.bullet,
            Ink::Line => self.line,
            Ink::Alert(kind) => self.alerts[kind as usize],
        };
        let style = Style::new().fg(fg).add_modifier(look.mods);
        if look.ink == Ink::Code && !row_bg {
            style.bg(self.code_bg)
        } else {
            style
        }
    }
}

#[cfg(test)]
mod tests;
