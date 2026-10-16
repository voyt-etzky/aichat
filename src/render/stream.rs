use super::{MarkdownRender, SseEvent};

use crate::utils::{poll_abort_signal, spawn_spinner, AbortSignal};

use anyhow::Result;
use crossterm::{
    cursor, queue, style,
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self, stdout, Stdout, Write},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedReceiver;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub async fn markdown_stream(
    rx: UnboundedReceiver<SseEvent>,
    render: &mut MarkdownRender,
    abort_signal: &AbortSignal,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    let ret = markdown_stream_inner(rx, render, abort_signal, &mut stdout).await;

    disable_raw_mode()?;

    if ret.is_err() {
        println!();
    }
    ret
}

pub async fn raw_stream(
    mut rx: UnboundedReceiver<SseEvent>,
    abort_signal: &AbortSignal,
) -> Result<()> {
    let mut spinner = Some(spawn_spinner("Generating"));

    loop {
        if abort_signal.aborted() {
            break;
        }
        if let Some(evt) = rx.recv().await {
            if let Some(spinner) = spinner.take() {
                spinner.stop();
            }

            match evt {
                SseEvent::Text(text) => {
                    print!("{text}");
                    stdout().flush()?;
                }
                SseEvent::Done => {
                    break;
                }
            }
        } else {
            break;
        }
    }
    if let Some(spinner) = spinner.take() {
        spinner.stop();
    }
    Ok(())
}

async fn markdown_stream_inner(
    mut rx: UnboundedReceiver<SseEvent>,
    render: &mut MarkdownRender,
    abort_signal: &AbortSignal,
    writer: &mut Stdout,
) -> Result<()> {
    let mut buffer = String::new();
    let mut buffer_rows: u16 = 1;
    let mut last_line_full = false;

    let mut columns = terminal::size()?.0.max(1);

    let mut spinner = Some(spawn_spinner("Generating"));

    'outer: loop {
        if abort_signal.aborted() {
            break;
        }
        for reply_event in gather_events(&mut rx).await {
            if let Some(spinner) = spinner.take() {
                spinner.stop();
            }

            columns = terminal::size().map(|(w, _)| w).unwrap_or(columns).max(1);

            match reply_event {
                SseEvent::Text(mut text) => {
                    if text.contains('\r') {
                        text = text.replace("\r\n", "\n").replace('\r', "");
                    }
                    // tab width hacking
                    text = text.replace('\t', "    ");

                    let mut attempts = 0;
                    let (col, row) = loop {
                        match cursor::position() {
                            Ok(pos) => break pos,
                            Err(_) if attempts < 3 => attempts += 1,
                            Err(_) => break (0, 0),
                        }
                    };

                    let mut up_rows = buffer_rows.saturating_sub(1);
                    // Fix unexpected duplicate lines on kitty, see https://github.com/sigoden/aichat/issues/105
                    if last_line_full && col == 0 && row > 0 {
                        up_rows += 1;
                    }

                    if up_rows > 0 {
                        queue!(writer, cursor::MoveUp(up_rows))?;
                    }
                    // No guarantee that text returned by render will not be re-layouted, so it is better to clear it.
                    queue!(
                        writer,
                        cursor::MoveToColumn(0),
                        terminal::Clear(terminal::ClearType::FromCursorDown),
                    )?;

                    if text.contains('\n') {
                        let text = format!("{buffer}{text}");
                        let (head, tail) = split_line_tail(&text);
                        let output = render.render(head);
                        print_block(writer, &output, columns)?;
                        buffer = tail.to_string();
                    } else {
                        buffer = format!("{buffer}{text}");
                    }

                    let output = render.render_line(&buffer);
                    if output.contains('\n') {
                        let (head, tail) = split_line_tail(&output);
                        buffer_rows = print_block(writer, head, columns)?;
                        queue!(writer, style::Print(tail))?;

                        // No guarantee the buffer width of the buffer will not exceed the number of columns.
                        // So we calculate the number of rows needed, rather than setting it directly to 1.
                        let (tail_rows, tail_col) = line_layout(tail, columns);
                        buffer_rows += tail_rows;
                        last_line_full = tail_col == columns;
                    } else {
                        queue!(writer, style::Print(&output))?;
                        let (rows, line_col) = line_layout(&output, columns);
                        buffer_rows = rows;
                        last_line_full = line_col == columns;
                    }

                    writer.flush()?;
                }
                SseEvent::Done => {
                    break 'outer;
                }
            }
        }

        if poll_abort_signal(abort_signal)? {
            break;
        }
    }

    if let Some(spinner) = spinner.take() {
        spinner.stop();
    }
    Ok(())
}

async fn gather_events(rx: &mut UnboundedReceiver<SseEvent>) -> Vec<SseEvent> {
    let mut texts = vec![];
    let mut done = false;
    tokio::select! {
        _ = async {
            while let Some(reply_event) = rx.recv().await {
                match reply_event {
                    SseEvent::Text(v) => texts.push(v),
                    SseEvent::Done => {
                        break;
                    }
                }
            }
            done = true;
        } => {}
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    };
    let mut events = vec![];
    if !texts.is_empty() {
        events.push(SseEvent::Text(texts.join("")))
    }
    if done {
        events.push(SseEvent::Done)
    }
    events
}

fn print_block(writer: &mut Stdout, text: &str, columns: u16) -> Result<u16> {
    let mut num = 0;
    for line in text.split('\n') {
        queue!(
            writer,
            style::Print(line),
            style::Print("\n"),
            cursor::MoveToColumn(0),
        )?;
        num += need_rows(line, columns);
    }
    Ok(num)
}

fn split_line_tail(text: &str) -> (&str, &str) {
    if let Some((head, tail)) = text.rsplit_once('\n') {
        (head, tail)
    } else {
        ("", text)
    }
}

fn grapheme_width(grapheme: &str) -> u16 {
    UnicodeWidthStr::width(grapheme).min(2) as u16
}

fn line_layout(text: &str, columns: u16) -> (u16, u16) {
    let columns = columns.max(1);
    let stripped = strip_ansi_escapes::strip_str(text);
    let mut rows: u16 = 1;
    let mut col: u16 = 0;

    for grapheme in stripped.graphemes(true) {
        let width = grapheme_width(grapheme);
        if width == 0 {
            continue;
        }
        if col + width > columns {
            if col > 0 {
                rows += 1;
                col = 0;
            }
            if width > columns {
                rows += width - 1;
                col = columns;
                continue;
            }
        }
        col += width;
    }

    (rows, col)
}

fn need_rows(text: &str, columns: u16) -> u16 {
    line_layout(text, columns).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_need_rows_ansi_escapes() {
        let plain = "hello world";
        let colored = "\x1b[31mhello \x1b[32mworld\x1b[0m";
        assert_eq!(need_rows(plain, 80), 1);
        assert_eq!(need_rows(colored, 80), 1);

        let long_plain = "a".repeat(100);
        let long_colored = format!("\x1b[34m{}\x1b[0m", "a".repeat(100));
        assert_eq!(need_rows(&long_plain, 80), 2);
        assert_eq!(need_rows(&long_colored, 80), 2);
    }

    #[test]
    fn test_line_layout_emojis_and_wide_wrapping() {
        // Emoji presentation sequences (U+FE0F) and ZWJ sequences have width 2
        assert_eq!(line_layout("⚠️", 80), (1, 2));
        assert_eq!(line_layout("👨‍💻", 80), (1, 2));
        assert_eq!(line_layout("👍🏻", 80), (1, 2));

        // 2-column character at column 9 of 10-column terminal wraps early to next row
        let line = "123456789🚀123456789";
        assert_eq!(line_layout(line, 10), (3, 1));
        assert_eq!(need_rows(line, 10), 3);

        // Exact column boundary detection for 1x and 2x columns
        assert_eq!(line_layout(&"a".repeat(80), 80), (1, 80));
        assert_eq!(line_layout(&"a".repeat(160), 80), (2, 80));
    }
}
