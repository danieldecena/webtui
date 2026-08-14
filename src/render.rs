use crate::extract::{Block, Document, Kind};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const DIM: Color = Color::DarkGray;

/// Pure: a document plus the currently focused element id becomes styled lines.
pub fn lines(doc: &Document, focus: Option<u32>) -> Vec<Line<'static>> {
    let mut out = Vec::with_capacity(doc.blocks.len() + 8);
    for block in &doc.blocks {
        let focused = block.id.is_some() && block.id == focus;
        match block.kind {
            Kind::Heading => {
                out.push(Line::default());
                out.push(Line::from(Span::styled(
                    block.text.clone(),
                    Style::default()
                        .fg(heading_color(block.level.unwrap_or(6)))
                        .add_modifier(Modifier::BOLD),
                )));
            }
            Kind::Para => out.push(Line::from(block.text.clone())),
            Kind::Listitem => out.push(Line::from(format!("  - {}", block.text))),
            Kind::Row => out.push(Line::from(Span::styled(
                block.text.clone(),
                Style::default().fg(Color::Gray),
            ))),
            Kind::Code => {
                for raw in block.text.lines() {
                    out.push(Line::from(Span::styled(
                        format!("    {raw}"),
                        Style::default().fg(Color::Green),
                    )));
                }
            }
            Kind::Link => out.push(interactive(block, focused, |t| t.to_string(), Color::Blue)),
            Kind::Input => out.push(interactive(
                block,
                focused,
                |t| format!("[ {t} ]"),
                Color::Magenta,
            )),
            Kind::Button => {
                out.push(interactive(block, focused, |t| format!("< {t} >"), Color::Yellow))
            }
        }
    }
    out
}

fn interactive(
    block: &Block,
    focused: bool,
    shape: impl Fn(&str) -> String,
    color: Color,
) -> Line<'static> {
    let mut style = Style::default().fg(color);
    if focused {
        style = style.add_modifier(Modifier::REVERSED);
    }
    Line::from(vec![
        Span::styled(
            format!("[{}] ", block.id.unwrap_or(0)),
            Style::default().fg(DIM),
        ),
        Span::styled(shape(&block.text), style),
    ])
}

fn heading_color(level: u8) -> Color {
    match level {
        1 => Color::Cyan,
        2 => Color::LightCyan,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(kind: Kind, text: &str, id: Option<u32>) -> Block {
        Block { kind, text: text.into(), id, href: None, level: Some(1) }
    }

    fn doc(blocks: Vec<Block>) -> Document {
        Document { url: "https://x.test/".into(), title: "x".into(), blocks }
    }

    #[test]
    fn heading_gets_a_leading_blank_line() {
        let out = lines(&doc(vec![block(Kind::Heading, "Title", None)]), None);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].to_string(), "Title");
    }

    #[test]
    fn link_is_addressed_by_its_id() {
        let out = lines(&doc(vec![block(Kind::Link, "Learn more", Some(7))]), None);
        assert_eq!(out[0].to_string(), "[7] Learn more");
    }

    #[test]
    fn only_the_focused_element_is_reversed() {
        let blocks = vec![
            block(Kind::Link, "one", Some(1)),
            block(Kind::Button, "two", Some(2)),
        ];
        let out = lines(&doc(blocks), Some(2));
        let reversed = |line: &Line| {
            line.spans.iter().any(|s| s.style.add_modifier.contains(Modifier::REVERSED))
        };
        assert!(!reversed(&out[0]));
        assert!(reversed(&out[1]));
    }

    #[test]
    fn multiline_code_becomes_one_line_each() {
        let out = lines(&doc(vec![block(Kind::Code, "a\nb\nc", None)]), None);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].to_string(), "    a");
    }

    #[test]
    fn empty_text_and_missing_level_do_not_panic() {
        let mut odd = block(Kind::Heading, "", None);
        odd.level = None;
        let out = lines(&doc(vec![odd, block(Kind::Para, "", None)]), None);
        assert_eq!(out.len(), 3);
    }
}
