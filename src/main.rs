use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Chart, Dataset, GraphType, Paragraph},
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
    data: Vec<(f64, f64)>,
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

        let datasets = vec![Dataset::default()
            .name("data")
            .marker(ratatui::symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&self.data)];

        let chart = Chart::new(datasets)
            .block(
                Block::bordered().title(Span::styled(
                    "Data Chart",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            )
            .x_axis(
                Axis::default()
                    .title("X")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(self.get_x_bounds()),
            )
            .y_axis(
                Axis::default()
                    .title("Y")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(self.get_y_bounds()),
            );
        frame.render_widget(chart, chunks[3]);
    }

    fn get_x_bounds(&self) -> [f64; 2] {
        if self.data.is_empty() {
            return [0.0, 10.0];
        }
        let min = self.data.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let max = self.data.iter().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max);
        [min, max]
    }

    fn get_y_bounds(&self) -> [f64; 2] {
        if self.data.is_empty() {
            return [0.0, 10.0];
        }
        let min = self.data.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let max = self.data.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);
        [min, max]
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
                let x: f64 = record[0].parse()?;
                let y: f64 = record[1].parse()?;
                new_data.push((x, y));
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
