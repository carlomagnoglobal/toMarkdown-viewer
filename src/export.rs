//! Markdown → DOCX / RTF export for the desktop app. Text-level fidelity:
//! headings, paragraphs, bold/italic/code, lists (rendered as prefixed
//! paragraphs), code blocks, blockquotes, and tables. Images are omitted.

use docx_rs::{
    AlignmentType, Docx, Paragraph, Run, RunFonts, Table, TableCell, TableRow,
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

#[derive(Default, Clone)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
}

/// One exported block with inline spans.
enum Block {
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    Code(String),
    Quote(Vec<Span>),
    Grid(Vec<Vec<String>>),
}

/// Walk the markdown into a flat block list both exporters share.
fn collect_blocks(md: &str) -> Vec<Block> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut blocks: Vec<Block> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let mut bold = 0usize;
    let mut italic = 0usize;
    let mut heading: Option<u8> = None;
    let mut in_quote = 0usize;
    let mut code_block: Option<String> = None;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut item_prefix: Option<String> = None;
    // Table assembly
    let mut table: Option<Vec<Vec<String>>> = None;
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_cell = false;

    let push_span = |spans: &mut Vec<Span>, text: &str, bold: usize, italic: usize, code: bool| {
        if text.is_empty() {
            return;
        }
        spans.push(Span {
            text: text.to_string(),
            bold: bold > 0,
            italic: italic > 0,
            code,
        });
    };

    let flush = |blocks: &mut Vec<Block>, spans: &mut Vec<Span>, heading: &mut Option<u8>, in_quote: usize| {
        if spans.is_empty() {
            return;
        }
        let s = std::mem::take(spans);
        match heading.take() {
            Some(l) => blocks.push(Block::Heading(l, s)),
            None if in_quote > 0 => blocks.push(Block::Quote(s)),
            None => blocks.push(Block::Paragraph(s)),
        }
    };

    for event in Parser::new_ext(md, options) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut blocks, &mut spans, &mut heading, in_quote);
                heading = Some(level as u8);
            }
            Event::End(TagEnd::Heading(_)) | Event::End(TagEnd::Paragraph) => {
                flush(&mut blocks, &mut spans, &mut heading, in_quote);
            }
            Event::Start(Tag::BlockQuote(_)) => in_quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => in_quote = in_quote.saturating_sub(1),
            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),
            Event::Start(Tag::CodeBlock(kind)) => {
                flush(&mut blocks, &mut spans, &mut heading, in_quote);
                let _ = kind; // language not represented in DOCX/RTF output
                code_block = Some(String::new());
                if let CodeBlockKind::Fenced(_) = kind {}
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = code_block.take() {
                    blocks.push(Block::Code(code));
                }
            }
            Event::Start(Tag::List(start)) => list_stack.push(start),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                flush(&mut blocks, &mut spans, &mut heading, in_quote);
                let depth = list_stack.len().saturating_sub(1);
                let indent = "    ".repeat(depth);
                let marker = match list_stack.last().copied().flatten() {
                    Some(_) => "1. ".to_string(),
                    None => "• ".to_string(),
                };
                item_prefix = Some(format!("{}{}", indent, marker));
            }
            Event::End(TagEnd::Item) => {
                flush(&mut blocks, &mut spans, &mut heading, in_quote);
                item_prefix = None;
            }
            Event::TaskListMarker(done) => {
                let mark = if done { "☑ " } else { "☐ " };
                push_span(&mut spans, mark, 0, 0, false);
            }
            // Tables
            Event::Start(Tag::Table(_)) => table = Some(Vec::new()),
            Event::End(TagEnd::Table) => {
                if let Some(t) = table.take() {
                    blocks.push(Block::Grid(t));
                }
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => row = Vec::new(),
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                if let Some(t) = &mut table {
                    t.push(std::mem::take(&mut row));
                }
            }
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_cell = false;
                row.push(std::mem::take(&mut cell));
            }
            Event::Text(t) | Event::Code(t) if in_cell => cell.push_str(&t),
            Event::Text(t) => {
                if let Some(code) = &mut code_block {
                    code.push_str(&t);
                } else {
                    if let Some(prefix) = item_prefix.take() {
                        push_span(&mut spans, &prefix, 0, 0, false);
                    }
                    push_span(&mut spans, &t, bold, italic, false);
                }
            }
            Event::Code(t) => push_span(&mut spans, &t, bold, italic, true),
            Event::SoftBreak | Event::HardBreak => push_span(&mut spans, " ", 0, 0, false),
            _ => {}
        }
    }
    flush(&mut blocks, &mut spans, &mut heading, in_quote);
    blocks
}

// ---- DOCX ----

fn docx_runs(spans: &[Span]) -> Vec<Run> {
    spans
        .iter()
        .map(|s| {
            let mut run = Run::new().add_text(&s.text);
            if s.bold {
                run = run.bold();
            }
            if s.italic {
                run = run.italic();
            }
            if s.code {
                run = run.fonts(RunFonts::new().ascii("Courier New"));
            }
            run
        })
        .collect()
}

pub fn markdown_to_docx(md: &str, title: &str) -> Result<Vec<u8>, String> {
    let mut doc = Docx::new();
    if !title.is_empty() {
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(title).bold().size(40))
                .align(AlignmentType::Left),
        );
    }
    for block in collect_blocks(md) {
        match block {
            Block::Heading(level, spans) => {
                let size = match level {
                    1 => 36,
                    2 => 30,
                    3 => 26,
                    _ => 24,
                };
                let mut p = Paragraph::new();
                for run in docx_runs(&spans) {
                    p = p.add_run(run.bold().size(size));
                }
                doc = doc.add_paragraph(p);
            }
            Block::Paragraph(spans) => {
                let mut p = Paragraph::new();
                for run in docx_runs(&spans) {
                    p = p.add_run(run);
                }
                doc = doc.add_paragraph(p);
            }
            Block::Quote(spans) => {
                let mut p = Paragraph::new().indent(Some(720), None, None, None);
                for run in docx_runs(&spans) {
                    p = p.add_run(run.italic());
                }
                doc = doc.add_paragraph(p);
            }
            Block::Code(code) => {
                for line in code.lines() {
                    doc = doc.add_paragraph(Paragraph::new().add_run(
                        Run::new().add_text(line).fonts(RunFonts::new().ascii("Courier New")).size(18),
                    ));
                }
            }
            Block::Grid(rows) => {
                let t_rows: Vec<TableRow> = rows
                    .iter()
                    .map(|r| {
                        TableRow::new(
                            r.iter()
                                .map(|c| {
                                    TableCell::new().add_paragraph(
                                        Paragraph::new().add_run(Run::new().add_text(c)),
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect();
                doc = doc.add_table(Table::new(t_rows));
            }
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    doc.build().pack(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

// ---- RTF ----

fn rtf_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            c if (c as u32) > 127 => out.push_str(&format!("\\u{}?", c as i32)),
            c => out.push(c),
        }
    }
    out
}

fn rtf_spans(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|s| {
            let mut open = String::new();
            let mut close = String::new();
            if s.bold {
                open.push_str("\\b ");
                close.push_str("\\b0 ");
            }
            if s.italic {
                open.push_str("\\i ");
                close.push_str("\\i0 ");
            }
            if s.code {
                open.push_str("\\f1 ");
                close.push_str("\\f0 ");
            }
            format!("{}{}{}", open, rtf_escape(&s.text), close)
        })
        .collect()
}

pub fn markdown_to_rtf(md: &str, title: &str) -> String {
    let mut out = String::from(
        "{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Helvetica;}{\\f1 Courier New;}}\\fs24\n",
    );
    if !title.is_empty() {
        out.push_str(&format!("{{\\fs40\\b {}}}\\par\\par\n", rtf_escape(title)));
    }
    for block in collect_blocks(md) {
        match block {
            Block::Heading(level, spans) => {
                let size = match level {
                    1 => 36,
                    2 => 30,
                    _ => 26,
                };
                out.push_str(&format!("{{\\fs{}\\b {}}}\\par\\par\n", size, rtf_spans(&spans)));
            }
            Block::Paragraph(spans) => out.push_str(&format!("{}\\par\\par\n", rtf_spans(&spans))),
            Block::Quote(spans) => {
                out.push_str(&format!("{{\\li720\\i {}}}\\par\\par\n", rtf_spans(&spans)))
            }
            Block::Code(code) => {
                out.push_str("{\\f1\\fs18 ");
                for line in code.lines() {
                    out.push_str(&rtf_escape(line));
                    out.push_str("\\line ");
                }
                out.push_str("}\\par\\par\n");
            }
            Block::Grid(rows) => {
                // RTF tables are gnarly; emit tab-separated rows.
                for r in rows {
                    out.push_str(&rtf_escape(&r.join("\t")));
                    out.push_str("\\par\n");
                }
                out.push_str("\\par\n");
            }
        }
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Title\n\nSome **bold** and *italic* and `code`.\n\n- item one\n- item two\n\n> quoted\n\n```rust\nfn main() {}\n```\n\n| A | B |\n| - | - |\n| 1 | 2 |\n";

    #[test]
    fn docx_export_produces_valid_zip() {
        let bytes = markdown_to_docx(SAMPLE, "Doc").unwrap();
        // DOCX is a ZIP: PK magic + non-trivial size.
        assert_eq!(&bytes[..2], b"PK");
        assert!(bytes.len() > 1000);
    }

    #[test]
    fn rtf_export_contains_styled_content() {
        let rtf = markdown_to_rtf(SAMPLE, "Doc");
        assert!(rtf.starts_with("{\\rtf1"));
        assert!(rtf.contains("\\b bold"));
        assert!(rtf.contains("\\i italic"));
        assert!(rtf.contains("fn main()"));
        // The bullet is non-ASCII, so it appears as an RTF unicode escape.
        assert!(rtf.contains("\\u8226? item one"), "rtf: {}", &rtf[..600.min(rtf.len())]);
        assert!(rtf.ends_with('}'));
    }
}
