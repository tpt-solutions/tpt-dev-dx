use crate::scan::{delete_target, TargetEntry};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use humansize::{format_size, BINARY};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;

struct App {
    entries: Vec<TargetEntry>,
    selected: std::collections::HashSet<usize>,
    list_state: ListState,
    dry_run: bool,
    status: String,
}

impl App {
    fn new(entries: Vec<TargetEntry>, dry_run: bool) -> Self {
        let mut list_state = ListState::default();
        if !entries.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            entries,
            selected: std::collections::HashSet::new(),
            list_state,
            dry_run,
            status: String::new(),
        }
    }

    fn current_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    fn move_up(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i > 0 {
            self.list_state.select(Some(i - 1));
        }
    }

    fn move_down(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i + 1 < self.entries.len() {
            self.list_state.select(Some(i + 1));
        }
    }

    fn toggle_selected(&mut self) {
        if let Some(i) = self.current_index() {
            if self.selected.contains(&i) {
                self.selected.remove(&i);
            } else {
                self.selected.insert(i);
            }
        }
    }

    fn select_all(&mut self) {
        if self.selected.len() == self.entries.len() {
            self.selected.clear();
        } else {
            self.selected = (0..self.entries.len()).collect();
        }
    }

    fn delete_selected(&mut self) {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort_unstable_by(|a, b| b.cmp(a)); // remove from back first

        let mut deleted = 0u64;
        let mut count = 0;
        for i in &indices {
            let entry = &self.entries[*i];
            let ok = self.dry_run || delete_target(entry).is_ok();
            if ok {
                count += 1;
                deleted += entry.size_bytes;
            }
        }
        for i in indices {
            self.entries.remove(i);
        }
        self.selected.clear();
        // Clamp list cursor.
        if let Some(cur) = self.list_state.selected() {
            if cur >= self.entries.len() && !self.entries.is_empty() {
                self.list_state.select(Some(self.entries.len() - 1));
            }
        }
        let dry = if self.dry_run { " (dry-run)" } else { "" };
        self.status = format!(
            "Deleted{dry} {count} director{} — freed {}",
            if count == 1 { "y" } else { "ies" },
            format_size(deleted, BINARY)
        );
    }
}

pub fn run(entries: Vec<TargetEntry>, dry_run: bool) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, entries, dry_run);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    entries: Vec<TargetEntry>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let mut app = App::new(entries, dry_run);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                    KeyCode::Char(' ') => app.toggle_selected(),
                    KeyCode::Char('a') => app.select_all(),
                    KeyCode::Char('d') if !app.selected.is_empty() => app.delete_selected(),
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn size_color(bytes: u64) -> Color {
    if bytes >= 1_073_741_824 {
        Color::Red
    } else if bytes >= 104_857_600 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn draw(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[0]);

    // ── Left panel: directory list ────────────────────────────────────────
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let selected_marker = if app.selected.contains(&i) { "✓ " } else { "  " };
            let size_str = format_size(e.size_bytes, BINARY);
            let color = size_color(e.size_bytes);
            let path_str = e.path.display().to_string();
            let line = Line::from(vec![
                Span::styled(selected_marker, Style::default().fg(Color::Green)),
                Span::styled(size_str, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::raw(path_str),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" target/ directories "))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, main_chunks[0], &mut app.list_state);

    // ── Right panel: details ──────────────────────────────────────────────
    let detail_text = if let Some(i) = app.current_index() {
        let e = &app.entries[i];
        format!(
            "Path:\n  {}\n\nSize:\n  {}\n\nLast modified:\n  {}\n\nSelected: {}",
            e.path.display(),
            format_size(e.size_bytes, BINARY),
            e.last_modified,
            if app.selected.contains(&i) { "yes" } else { "no" }
        )
    } else {
        "No entries found.".to_string()
    };

    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(detail, main_chunks[1]);

    // ── Bottom bar: status + keybindings ─────────────────────────────────
    let total: u64 = app.entries.iter().map(|e| e.size_bytes).sum();
    let selected_size: u64 = app.selected.iter().map(|i| app.entries[*i].size_bytes).sum();
    let status = if app.status.is_empty() {
        format!(
            " Total: {}  |  Selected: {} ({})  |  [space] toggle  [a] all  [d] delete  [q] quit",
            format_size(total, BINARY),
            app.selected.len(),
            format_size(selected_size, BINARY),
        )
    } else {
        format!(" {}  |  [q] quit", app.status)
    };

    let status_bar = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(status_bar, chunks[1]);
}
