use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Paragraph},
    DefaultTerminal, Frame,
};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

/// The input mode of the application.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// The current input mode.
    input_mode: InputMode,
    /// The current input for the CSV path.
    input: String,
    /// The data for the chart.
    data: Vec<(String, u64)>,
    /// Error message to display.
    error_message: Option<String>,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(frame.area());

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    Span::raw("Press "),
                    Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to exit, "),
                    Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to start editing."),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    Span::raw("Press "),
                    Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to stop editing, "),
                    Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" to record the message"),
                ],
                Style::default(),
            ),
        };
        let text = Line::from(msg).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, chunks[0]);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("CSV Path"));
        frame.render_widget(input, chunks[1]);

        match self.input_mode {
            InputMode::Normal =>
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                {}

            InputMode::Editing => {
                // Make the cursor visible and ask ratatui to put it at the specified coordinates after rendering
                frame.set_cursor_position((
                    // Put cursor past the end of the input text
                    chunks[1].x + self.input.len() as u16 + 1,
                    // Move one line down, from the border to the input line
                    chunks[1].y + 1,
                ));
            }
        }

        let error_message = if let Some(err) = &self.error_message {
            Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red))
        } else {
            Paragraph::new("Enter a CSV path (e.g., test.csv) and press Enter")
        };
        frame.render_widget(error_message, chunks[2]);

        let bar_data: Vec<Bar> = self
            .data
            .iter()
            .enumerate()
            .map(|(i, (label, value))| {
                let color = match i % 6 {
                    0 => Color::Red,
                    1 => Color::Green,
                    2 => Color::Yellow,
                    3 => Color::Blue,
                    4 => Color::Magenta,
                    _ => Color::Cyan,
                };
                Bar::default()
                    .value(*value)
                    .label(Line::from(label.as_str()))
                    .style(Style::default().fg(color))
            })
            .collect();

        let barchart = BarChart::default()
            .block(
                Block::bordered().title(Span::styled(
                    "Data Chart",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            )
            .data(BarGroup::default().bars(&bar_data))
            .bar_width(9)
            .bar_gap(1);
        frame.render_widget(barchart, chunks[3]);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('e') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Char('q') => {
                    self.quit();
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    if let Err(e) = self.load_csv() {
                        self.error_message = Some(format!("Error: {}", e));
                    } else {
                        self.error_message = None;
                        self.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                _ => {}
            },
        }
    }

    fn load_csv(&mut self) -> color_eyre::Result<()> {
        let mut rdr = csv::Reader::from_path(&self.input)?;
        let mut new_data = Vec::new();
        for result in rdr.records() {
            let record = result?;
            if record.len() >= 2 {
                let label = record[0].to_string();
                let value: u64 = record[1].parse()?;
                new_data.push((label, value));
            }
        }
        if new_data.is_empty() {
            return Err(color_eyre::eyre::eyre!("No valid data found in CSV"));
        }
        self.data = new_data;
        Ok(())
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
