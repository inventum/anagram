//! ratatui rendering for `GameState`.

mod theme;

use crate::affix_hints;
use crate::app::{GameState, InputFocus, Phase, Status};
use crate::game::Round;
use crate::workbench::LetterWorkbench;
pub use theme::{parse_color_name, UiTheme};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

const TITLE: &str = " Anagram ";
const SIZE_MIN: u8 = 4;
const SIZE_MAX: u8 = 10;
/// Side-by-side layout: enough height to show grid rows under the pool line.
const WORKBENCH_FULL_MIN_HEIGHT: u16 = 14;
/// Minimum width reserved for the workbench column (fullwidth glyphs + gaps).
const WORKBENCH_PANEL_PAD: u16 = 6;
/// Fixed terminal columns between vertical grid lines (ASCII cell content — stays aligned).
const WORKBENCH_GRID_CELL: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkbenchDrawMode {
    Full,
    Collapsed,
    Skip,
}

fn workbench_min_panel_width(cols: u8) -> u16 {
    // ~2 terminal cols per fullwidth letter + "  " between cells, plus block borders.
    cols as u16 * 4 + WORKBENCH_PANEL_PAD
}

fn workbench_draw_mode(terminal_width: u16, terminal_height: u16, cols: u8) -> WorkbenchDrawMode {
    let wb_min = workbench_min_panel_width(cols);
    let answer_min = 26u16;
    if terminal_width < wb_min.saturating_add(answer_min) || terminal_height < 10 {
        return WorkbenchDrawMode::Skip;
    }
    if terminal_height >= WORKBENCH_FULL_MIN_HEIGHT
        && terminal_width >= wb_min.saturating_add(30)
    {
        WorkbenchDrawMode::Full
    } else {
        WorkbenchDrawMode::Collapsed
    }
}

pub fn draw(frame: &mut Frame, state: &GameState, theme: &UiTheme) {
    let area = frame.area();
    match state.phase {
        Phase::Menu => draw_menu(frame, area, state),
        Phase::SizeSelect => draw_size_select(frame, area, state),
        Phase::CustomWordInput => draw_custom_word_input(frame, area, state),
        Phase::CustomWordPick => draw_custom_word_pick(frame, area, state),
        Phase::Playing | Phase::Help | Phase::Won => {
            draw_play_screen(frame, area, state, theme);
            if state.help_open || state.phase == Phase::Help {
                draw_help_overlay(frame, area);
            }
        }
    }
}

fn draw_menu(frame: &mut Frame, area: Rect, state: &GameState) {
    let block = main_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    let header = Paragraph::new(Line::from(vec![
        Span::raw("Length: "),
        Span::styled(
            format!("{}", state.size),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("          Clues: 0/3 used"),
    ]));
    frame.render_widget(header, chunks[0]);

    let body = Paragraph::new(vec![
        Line::from("Welcome to Anagram."),
        Line::from(""),
        Line::from(vec![
            Span::styled("[n]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" New puzzle  "),
            Span::styled("[i]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Your word  "),
            Span::styled("[s]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Size  "),
            Span::styled("[?]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Help  "),
            Span::raw("⌘Q / Ctrl+C Quit"),
        ]),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(body, chunks[1]);

    render_status(chunks[2], frame, state);
    frame.render_widget(footer_line(state), chunks[3]);
}

fn draw_size_select(frame: &mut Frame, area: Rect, state: &GameState) {
    let block = Block::default()
        .title(" Size ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let presets: Vec<Span<'_>> = (SIZE_MIN..=SIZE_MAX)
        .map(|n| {
            let label = format!(" {n} ");
            if n == state.size {
                Span::styled(
                    label,
                    Style::default()
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                )
            } else {
                Span::raw(label)
            }
        })
        .collect();

    let text = vec![
        Line::from("Choose puzzle length (4–10):"),
        Line::from(""),
        Line::from(presets),
        Line::from(""),
        Line::from("Type a digit 4–10 to select, Enter to confirm."),
        Line::from(vec![
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" quit"),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_custom_word_input(frame: &mut Frame, area: Rect, state: &GameState) {
    let block = Block::default()
        .title(" Your word ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let len = state.custom_word_input.len();
    let min = state.min_word_len;
    let max = state.max_word_len;
    let word_line = if len == 0 {
        Line::from(vec![
            Span::styled(
                "_",
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::raw("  (solution or scrambled letters, any order)"),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                state.custom_word_input.to_ascii_uppercase(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("  ({len} letters)")),
        ])
    };

    let text = vec![
        Line::from(format!(
            "Enter letters ({min}–{max}, A–Z): unscramble in the puzzle"
        )),
        Line::from(""),
        word_line,
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" start puzzle  "),
            Span::styled("Backspace", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" delete  "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" menu  "),
            Span::styled("⌘Q", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" quit"),
        ]),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }),
        chunks[0],
    );
    render_status(chunks[1], frame, state);
    frame.render_widget(footer_line(state), chunks[2]);
}

fn draw_custom_word_pick(frame: &mut Frame, area: Rect, state: &GameState) {
    let block = Block::default()
        .title(" Pick a word ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(format!(
            "Scramble: {}",
            state.custom_word_input.to_ascii_uppercase()
        )),
        Line::from(""),
    ];
    for (i, entry) in state.custom_word_choices.iter().enumerate() {
        let marker = if i == state.custom_word_pick { ">" } else { " " };
        let def = entry
            .definition
            .as_deref()
            .unwrap_or("…");
        let short = if def.len() > 60 {
            format!("{}…", &def[..60])
        } else {
            def.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{marker} {} ", entry.word.to_ascii_uppercase()),
                if i == state.custom_word_pick {
                    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default()
                },
            ),
            Span::raw(short),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" move  "),
        Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" play  "),
        Span::styled("u", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" menu"),
    ]));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);
    render_status(chunks[1], frame, state);
    frame.render_widget(footer_line(state), chunks[2]);
}

fn draw_play_screen(frame: &mut Frame, area: Rect, state: &GameState, theme: &UiTheme) {
    let block = main_block();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut constraints = vec![
        Constraint::Length(1),
        Constraint::Min(18),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ];
    if show_banner(state) {
        constraints.insert(3, Constraint::Length(1));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let len = state
        .round
        .as_ref()
        .map(|r| r.answer.len())
        .unwrap_or(state.size as usize);

    let header = Paragraph::new(Line::from(vec![
        Span::raw("Length: "),
        Span::styled(format!("{len}"), Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(
            "          Clues: {}/{} used",
            state.clues_used,
            state
                .current_word
                .as_ref()
                .map(|entry| {
                    crate::dictionary::Dictionary::clue_budget_round(
                        entry,
                        &state.anagram_alternates,
                    )
                })
                .unwrap_or(3)
        )),
    ]));
    frame.render_widget(header, chunks[0]);

    if let Some(round) = state.round.as_ref() {
        if let Some(wb) = state.workbench.as_ref() {
            let wb_mode = workbench_draw_mode(area.width, area.height, wb.cols);
            match wb_mode {
                WorkbenchDrawMode::Skip => {
                    draw_game_board(
                        frame,
                        chunks[1],
                        round,
                        theme,
                        state.highlight_correct_positions,
                    );
                }
                WorkbenchDrawMode::Full | WorkbenchDrawMode::Collapsed => {
                    let wb_min = workbench_min_panel_width(wb.cols);
                    let h_constraints = match wb_mode {
                        WorkbenchDrawMode::Full => [
                            Constraint::Percentage(50),
                            Constraint::Percentage(50),
                        ],
                        WorkbenchDrawMode::Collapsed => [
                            Constraint::Min(wb_min),
                            Constraint::Min(24),
                        ],
                        WorkbenchDrawMode::Skip => unreachable!(),
                    };
                    let areas = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(h_constraints)
                        .split(chunks[1]);
                    draw_workbench(
                        frame,
                        areas[0],
                        wb,
                        round,
                        state.input_focus,
                        theme,
                        wb_mode,
                    );
                    draw_game_board(
                        frame,
                        areas[1],
                        round,
                        theme,
                        state.highlight_correct_positions,
                    );
                }
            }
        } else {
            draw_game_board(
                frame,
                chunks[1],
                round,
                theme,
                state.highlight_correct_positions,
            );
        }
    } else {
        frame.render_widget(
            Paragraph::new("Start a puzzle with [n] from the menu.")
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(" Play ")
                        .borders(Borders::ALL),
                ),
            chunks[1],
        );
    }

    let clue_label = state
        .active_clue_text
        .as_deref()
        .unwrap_or("—");
    let clue = Paragraph::new(format!("Clue: {clue_label}"))
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true });
    frame.render_widget(clue, chunks[2]);

    let mut idx = 3;
    if show_banner(state) {
        frame.render_widget(banner_paragraph(state), chunks[idx]);
        idx += 1;
    }

    render_status(chunks[idx], frame, state);
    frame.render_widget(footer_line(state), chunks[idx + 1]);
}

fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    let width = area.width.min(60).max(40);
    let height = 22u16.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    let help = Paragraph::new(vec![
        Line::from("Keys"),
        Line::from(""),
        Line::from("← →        move cursor between slots"),
        Line::from("a-z        place letter in highlighted slot"),
        Line::from("Backspace  clear slot / move left"),
        Line::from("Tab        cycle clues (wraps to first)"),
        Line::from("Ctrl+R     reset shuffle (same word)"),
        Line::from("Esc / ⌘Q   quit (Ctrl+C also)"),
        Line::from("?          toggle this help"),
        Line::from(""),
        Line::from("Menu: n new puzzle, i your word, s size"),
        Line::from("After winning: n new puzzle, s change size"),
        Line::from(""),
        Line::from("Workbench (cue only — does not affect win)"),
        Line::from("Ctrl+g     toggle Answer / Grid focus"),
        Line::from("← → ↑ ↓    move grid cursor (Grid focus)"),
        Line::from("a-z        type into grid cell from grid pool"),
        Line::from("Space      pick / place on grid"),
        Line::from("Backspace  clear grid cell (Grid focus)"),
        Line::from("Ctrl+Shift+G  reset grid (new shuffle)"),
    ])
    .block(block)
    .wrap(Wrap { trim: false });
    frame.render_widget(help, popup);
}

fn main_block() -> Block<'static> {
    Block::default().title(TITLE).borders(Borders::ALL)
}

/// Fullwidth capitals (2 terminal columns, reads larger than ASCII).
fn display_glyph(ch: char) -> String {
    if ch == '_' {
        return "＿".to_string();
    }
    let upper = ch.to_ascii_uppercase();
    if upper.is_ascii_alphabetic() {
        let code = 0xFF21u32 + (upper as u32 - 'A' as u32);
        return char::from_u32(code).unwrap_or(upper).to_string();
    }
    upper.to_string()
}

fn spaced_glyphs<I>(letters: I, gap: &'static str) -> Vec<Span<'static>>
where
    I: Iterator<Item = (String, Style)>,
{
    let mut spans = Vec::new();
    for (i, (glyph, style)) in letters.enumerate() {
        if i > 0 {
            spans.push(Span::raw(gap));
        }
        spans.push(Span::styled(glyph, style));
    }
    spans
}

fn draw_game_board(
    frame: &mut Frame,
    area: Rect,
    round: &crate::game::Round,
    theme: &UiTheme,
    highlight_correct_positions: bool,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(area);

    let pool_block = Block::default()
        .title(" Scrambled — pick from these letters ")
        .borders(Borders::ALL);
    let pool_inner = pool_block.inner(rows[0]);
    frame.render_widget(pool_block, rows[0]);

    let pool_body = if round.pool.is_empty() {
        Paragraph::new("(empty)")
    } else {
        let pool_style = Style::default()
            .fg(theme.pool)
            .add_modifier(Modifier::BOLD);
        let letters = round
            .pool
            .iter()
            .map(|&ch| (display_glyph(ch), pool_style));
        Paragraph::new(vec![
            Line::from(""),
            Line::from(spaced_glyphs(letters, "   ")),
        ])
    };
    frame.render_widget(pool_body.alignment(Alignment::Center), pool_inner);

    let answer_block = Block::default()
        .title(" Your answer ")
        .borders(Borders::ALL);
    let answer_inner = answer_block.inner(rows[1]);
    frame.render_widget(answer_block, rows[1]);

    let answer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(answer_inner);

    let answer_letters = round.answer.iter().enumerate().map(|(i, slot)| {
        let at_cursor = i == round.cursor;
        let glyph = match slot {
            Some(ch) => display_glyph(*ch),
            None => "_".to_string(),
        };
        let highlight = Modifier::BOLD | Modifier::UNDERLINED;
        let correct = highlight_correct_positions && round.slot_is_correct(i);
        let style = match slot {
            Some(_) if correct && at_cursor => Style::default()
                .add_modifier(Modifier::BOLD | Modifier::REVERSED | Modifier::UNDERLINED),
            Some(_) if correct => Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            Some(_) if at_cursor => Style::default()
                .fg(theme.filled_slot)
                .add_modifier(highlight),
            Some(_) => Style::default()
                .fg(theme.filled_slot)
                .add_modifier(Modifier::BOLD),
            None if at_cursor => Style::default()
                .fg(theme.empty_slot)
                .bg(Color::DarkGray)
                .add_modifier(highlight),
            None => Style::default()
                .fg(theme.empty_slot)
                .add_modifier(Modifier::BOLD),
        };
        (glyph, style)
    });
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(spaced_glyphs(answer_letters, "   ")),
        ])
        .alignment(Alignment::Center),
        answer_chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("← →", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" move slot  "),
            Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" clue"),
        ]))
        .alignment(Alignment::Center),
        answer_chunks[1],
    );
    frame.render_widget(
        Paragraph::new("Type a pool letter into the underlined slot")
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM)),
        answer_chunks[2],
    );
}

fn draw_affix_hints_panel(frame: &mut Frame, area: Rect, round: &Round) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let block = Block::default()
        .title(" Patterns (cue) ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let hints = affix_hints::affix_hints(round);
    let starts = if hints.starts.is_empty() {
        "—".to_string()
    } else {
        hints.starts.join(", ")
    };
    let ends = if hints.ends.is_empty() {
        "—".to_string()
    } else {
        hints.ends.join(", ")
    };
    let dim = Style::default().add_modifier(Modifier::DIM);
    let body = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Starts: ", dim),
            Span::raw(starts),
        ]),
        Line::from(vec![
            Span::styled("Ends:   ", dim),
            Span::raw(ends),
        ]),
    ])
    .wrap(Wrap { trim: true });
    frame.render_widget(body, inner);
}

fn draw_workbench(
    frame: &mut Frame,
    area: Rect,
    wb: &LetterWorkbench,
    round: &Round,
    input_focus: InputFocus,
    theme: &UiTheme,
    mode: WorkbenchDrawMode,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let grid_focused = input_focus == InputFocus::Grid;
    let block = Block::default()
        .title(" Workbench (cue only) ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    if mode == WorkbenchDrawMode::Collapsed {
        let focus = if grid_focused { "Grid" } else { "Answer" };
        let held = wb
            .held
            .map(|c| format!(" hand={}", display_glyph(c)))
            .unwrap_or_default();
        let line = Line::from(vec![
            Span::raw("Grid cue — Focus: "),
            Span::styled(focus, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("  pool {}  grid {}×{}", wb.pool.len(), wb.rows, wb.cols)),
            Span::raw(held),
            Span::raw("  "),
            Span::styled("Ctrl+g", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" toggle"),
        ]);
        frame.render_widget(Paragraph::new(line), inner);
        return;
    }

    let pool_rows = if wb.held.is_some() { 2u16 } else { 1 };
    let affix_rows = if inner.height >= pool_rows + 14 { 6u16 } else { 0 };
    let constraints: Vec<Constraint> = if affix_rows > 0 {
        vec![
            Constraint::Length(pool_rows),
            Constraint::Min(5),
            Constraint::Length(affix_rows),
        ]
    } else {
        vec![Constraint::Length(pool_rows), Constraint::Min(1)]
    };
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let pool_label = Span::styled(
        "Grid pool (cue only): ",
        Style::default().add_modifier(Modifier::BOLD),
    );
    let pool_spans = if wb.pool.is_empty() {
        vec![
            pool_label,
            Span::styled("(empty)", Style::default().add_modifier(Modifier::DIM)),
        ]
    } else {
        let pool_style = Style::default()
            .fg(theme.pool)
            .add_modifier(Modifier::BOLD);
        let letters = wb
            .pool
            .iter()
            .map(|&ch| (display_glyph(ch), pool_style));
        let mut spans = vec![pool_label];
        spans.extend(spaced_glyphs(letters, "   "));
        spans
    };
    let mut pool_lines = vec![Line::from(pool_spans)];
    if let Some(h) = wb.held {
        pool_lines.push(Line::from(vec![
            Span::raw("In hand: "),
            Span::styled(
                display_glyph(h),
                Style::default()
                    .fg(theme.pool)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(pool_lines), inner_chunks[0]);
    draw_workbench_grid(frame, inner_chunks[1], wb, grid_focused, theme);
    if affix_rows > 0 {
        draw_affix_hints_panel(frame, inner_chunks[2], round);
    }
}

fn workbench_cell_style(
    at_cursor: bool,
    filled: bool,
    theme: &UiTheme,
) -> Style {
    if filled {
        if at_cursor {
            Style::default()
                .fg(theme.pool)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
                .fg(theme.pool)
                .add_modifier(Modifier::BOLD)
        }
    } else if at_cursor {
        Style::default()
            .fg(theme.empty_slot)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED | Modifier::UNDERLINED)
    } else {
        Style::default()
            .fg(theme.empty_slot)
            .add_modifier(Modifier::BOLD)
    }
}

fn workbench_grid_line_width(cols: usize) -> usize {
    if cols == 0 {
        return 2;
    }
    1 + cols * (WORKBENCH_GRID_CELL + 1)
}

fn workbench_grid_h_fill() -> String {
    "─".repeat(WORKBENCH_GRID_CELL)
}

fn workbench_grid_border_line(kind: u8, cols: usize) -> Line<'static> {
    let (left, junction, right) = match kind {
        0 => ('┌', '┬', '┐'),
        1 => ('├', '┼', '┤'),
        _ => ('└', '┴', '┘'),
    };
    let border = Style::default().add_modifier(Modifier::DIM);
    let seg = workbench_grid_h_fill();
    let mut s = String::new();
    s.push(left);
    for c in 0..cols {
        if c > 0 {
            s.push(junction);
        }
        s.push_str(&seg);
    }
    s.push(right);
    Line::from(Span::styled(s, border))
}

fn workbench_grid_cell_text(ch: Option<char>) -> String {
    let c = ch.map(|x| x.to_ascii_uppercase()).unwrap_or('_');
    format!("{:^width$}", c, width = WORKBENCH_GRID_CELL)
}

fn workbench_grid_data_line(
    wb: &LetterWorkbench,
    row: usize,
    grid_focused: bool,
    theme: &UiTheme,
) -> Line<'static> {
    let cols = wb.cols as usize;
    let border = Style::default().add_modifier(Modifier::DIM);
    let mut spans = vec![Span::styled("│".to_string(), border)];
    for col in 0..cols {
        let idx = row * cols + col;
        let at_cursor = grid_focused && idx == wb.cursor;
        let ch = wb.cells.get(idx).and_then(|c| *c);
        let filled = ch.is_some();
        let text = workbench_grid_cell_text(ch);
        let style = workbench_cell_style(at_cursor, filled, theme);
        spans.push(Span::styled(text, style));
        spans.push(Span::styled("│".to_string(), border));
    }
    Line::from(spans)
}

fn workbench_boxed_grid_lines(
    wb: &LetterWorkbench,
    grid_focused: bool,
    theme: &UiTheme,
) -> Vec<Line<'static>> {
    let rows = wb.rows as usize;
    let cols = wb.cols as usize;
    let mut lines = Vec::with_capacity(rows * 2 + 1);
    lines.push(workbench_grid_border_line(0, cols));
    for row in 0..rows {
        lines.push(workbench_grid_data_line(wb, row, grid_focused, theme));
        if row + 1 < rows {
            lines.push(workbench_grid_border_line(1, cols));
        }
    }
    lines.push(workbench_grid_border_line(2, cols));
    lines
}

fn pad_grid_line(line: Line<'static>, pad_left: usize) -> Line<'static> {
    if pad_left == 0 {
        return line;
    }
    let pad = Span::raw(" ".repeat(pad_left));
    let mut spans = vec![pad];
    spans.extend(line.spans.into_iter());
    Line::from(spans)
}

fn draw_workbench_grid(
    frame: &mut Frame,
    area: Rect,
    wb: &LetterWorkbench,
    grid_focused: bool,
    theme: &UiTheme,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let cols = wb.cols as usize;
    let grid_w = workbench_grid_line_width(cols);
    let pad_left = (area.width as usize).saturating_sub(grid_w) / 2;
    let raw = workbench_boxed_grid_lines(wb, grid_focused, theme);
    let lines: Vec<Line<'_>> = raw
        .into_iter()
        .map(|line| pad_grid_line(line, pad_left))
        .collect();
    let grid_h = lines.len() as u16;
    let top_pad = area.height.saturating_sub(grid_h) / 2;
    let grid_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(grid_h),
            Constraint::Min(0),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(lines), grid_chunks[1]);
}

fn workbench_focus_footer_spans(state: &GameState) -> Vec<Span<'static>> {
    if state.workbench.is_none()
        || !matches!(state.phase, Phase::Playing | Phase::Help | Phase::Won)
    {
        return Vec::new();
    }
    let (answer_style, grid_style) = match state.input_focus {
        InputFocus::Answer => (
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
            Style::default().add_modifier(Modifier::DIM),
        ),
        InputFocus::Grid => (
            Style::default().add_modifier(Modifier::DIM),
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ),
    };
    vec![
        Span::raw("Focus: "),
        Span::styled("Answer", answer_style),
        Span::raw(" | "),
        Span::styled("Grid", grid_style),
        Span::raw("  "),
        Span::styled("Ctrl+g", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" toggle  "),
    ]
}

fn footer_line(state: &GameState) -> Paragraph<'static> {
    let focus = workbench_focus_footer_spans(state);
    let line = if matches!(state.phase, Phase::Playing | Phase::Help) {
        let mut spans = focus;
        spans.extend([
            Span::raw("Type letters • "),
            Span::styled("Backspace", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" undo  "),
            Span::styled("←→", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" move  "),
            Span::styled("[Tab]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Clue  "),
            Span::styled("Ctrl+R", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Reset  "),
            Span::styled("[?]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Help  "),
            Span::raw("Esc / ⌘Q Quit"),
        ]);
        Line::from(spans)
    } else if matches!(state.phase, Phase::Won) {
        let mut spans = focus;
        spans.extend([
            Span::styled("[n]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" New  "),
            Span::styled("[i]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Your letters  "),
            Span::styled("[s]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Size  "),
            Span::raw("Esc / ⌘Q Quit"),
        ]);
        Line::from(spans)
    } else {
        Line::from(vec![
            Span::styled("[n]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" New  "),
            Span::styled("[s]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Size  "),
            Span::styled("[?]", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(" Help  "),
            Span::raw("Esc / ⌘Q Quit"),
        ])
    };
    Paragraph::new(line).alignment(Alignment::Center)
}

fn show_banner(state: &GameState) -> bool {
    matches!(state.phase, Phase::Won) || matches!(state.status, Status::WrongAnswer)
}

fn banner_paragraph(state: &GameState) -> Paragraph<'static> {
    let (text, style) = if matches!(state.phase, Phase::Won) {
        (
            "★ Correct! Puzzle solved. ★",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "Not quite — check your letters.",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    };
    Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Center)
}

fn render_status(area: Rect, frame: &mut Frame, state: &GameState) {
    let line = status_line(state);
    if let Some(p) = line {
        frame.render_widget(p, area);
    }
}

fn status_line(state: &GameState) -> Option<Paragraph<'static>> {
    match &state.status {
        Status::None | Status::WrongAnswer => None,
        Status::Error(msg) => Some(
            Paragraph::new(Line::from(vec![
                Span::styled("Error: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(msg.clone()),
            ]))
            .style(Style::default().fg(Color::Red)),
        ),
        Status::Info(msg) => {
            if matches!(state.phase, Phase::Won) {
                return None;
            }
            Some(
                Paragraph::new(Line::from(vec![
                    Span::styled("Info: ", Style::default().add_modifier(Modifier::DIM)),
                    Span::raw(msg.clone()),
                ]))
                .style(Style::default().add_modifier(Modifier::DIM)),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::InputFocus;
    use crate::game::Round;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn playing_state() -> GameState {
        let mut rng = StdRng::seed_from_u64(42);
        let round = Round::new("crate", &mut rng);
        let mut state = GameState::new(5);
        state.phase = Phase::Playing;
        state.round = Some(round);
        state.active_clue_text = Some("A wooden box.".into());
        state.clues_used = 1;
        state
    }

    fn playing_state_with_workbench() -> GameState {
        let mut rng = StdRng::seed_from_u64(42);
        let mut state = playing_state();
        state.workbench = Some(LetterWorkbench::from_solution(
            "crate", 5, 5, &mut rng,
        ));
        state
    }

    #[test]
    fn draw_smoke_all_phases() {
        let theme = UiTheme::default();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");

        let mut menu = GameState::new(6);
        assert!(terminal.draw(|f| draw(f, &menu, &theme)).is_ok());

        menu.phase = Phase::SizeSelect;
        assert!(terminal.draw(|f| draw(f, &menu, &theme)).is_ok());

        let playing = playing_state();
        assert!(terminal.draw(|f| draw(f, &playing, &theme)).is_ok());

        let mut playing_wb = playing_state_with_workbench();
        assert!(terminal.draw(|f| draw(f, &playing_wb, &theme)).is_ok());

        playing_wb.input_focus = InputFocus::Grid;
        assert!(terminal.draw(|f| draw(f, &playing_wb, &theme)).is_ok());

        let backend_tall = TestBackend::new(80, 28);
        let mut terminal_tall = Terminal::new(backend_tall).expect("terminal");
        assert!(terminal_tall
            .draw(|f| draw(f, &playing_wb, &theme))
            .is_ok());

        let backend_short = TestBackend::new(80, 18);
        let mut terminal_short = Terminal::new(backend_short).expect("terminal");
        assert!(terminal_short
            .draw(|f| draw(f, &playing_wb, &theme))
            .is_ok());

        let mut won = playing.clone();
        won.phase = Phase::Won;
        won.status = Status::Info("Correct!".into());
        assert!(terminal.draw(|f| draw(f, &won, &theme)).is_ok());

        let mut won_wb = playing_state_with_workbench();
        won_wb.phase = Phase::Won;
        won_wb.status = Status::Info("Correct!".into());
        assert!(terminal.draw(|f| draw(f, &won_wb, &theme)).is_ok());

        let mut wrong = playing.clone();
        wrong.status = Status::WrongAnswer;
        assert!(terminal.draw(|f| draw(f, &wrong, &theme)).is_ok());

        let mut err = playing.clone();
        err.status = Status::Error("Letter not in pool.".into());
        assert!(terminal.draw(|f| draw(f, &err, &theme)).is_ok());

        let mut help = playing.clone();
        help.help_open = true;
        help.phase = Phase::Help;
        assert!(terminal.draw(|f| draw(f, &help, &theme)).is_ok());

        let mut help_wb = playing_state_with_workbench();
        help_wb.help_open = true;
        help_wb.phase = Phase::Help;
        assert!(terminal.draw(|f| draw(f, &help_wb, &theme)).is_ok());
    }
}
