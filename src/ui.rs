use std::io::{stdout, Write};

extern crate crossterm;
use crossterm::{
    execute, queue,
    style,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
    terminal::{EnterAlternateScreen, ClearType},
    ExecutableCommand, QueueableCommand, Result,
    event,
    cursor,
    cursor::{MoveTo},
    event::{read, Event, KeyCode, KeyEvent}
};

extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

extern crate unicode_width;
use unicode_width::UnicodeWidthStr;

extern crate regex;
use regex::Regex;

enum Command {
    Go(String),
    Quit,
    Unknown(String)
}

#[derive(Clone)]
struct PrintableLine {
    s: String,
    wrapped: bool
}

#[derive(Clone)]
struct ContentContainer {
    lines: Vec<PrintableLine>,
    rendered: Vec<String>,
    content_width: usize,
    width: usize,
    height: usize,
    top_margin: usize,
    bottom_margin: usize,
    left_margin: usize,
    right_margin: usize,
    scroll_row: usize,
    scroll_column: usize
}

impl ContentContainer {
    pub fn new() -> ContentContainer {
        let size = terminal::size().unwrap();

        let new_container = ContentContainer {
            lines: Vec::<PrintableLine>::new(),
            rendered: Vec::<String>::new(),
            content_width: 0,
            width: size.0 as usize,
            height: size.1 as usize - 2,
            top_margin: 1,
            bottom_margin: 1,
            left_margin: 0,
            right_margin: 0,
            scroll_row: 0,
            scroll_column: 0
        };

        new_container
    }

    pub fn from_lines(c: Vec<PrintableLine>) -> ContentContainer {
        let size = terminal::size().unwrap();
        let top_margin = 1;
        let bottom_margin = 1;
        let left_margin = 0;
        let right_margin = 0;

        let mut new_container = ContentContainer {
            lines: c,
            rendered: Vec::<String>::new(),
            content_width: 0,
            width: size.0 as usize - (left_margin + right_margin),
            height: size.1 as usize - (top_margin + bottom_margin),
            top_margin: top_margin,
            bottom_margin: bottom_margin,
            left_margin: left_margin,
            right_margin: right_margin,
            scroll_row: 0,
            scroll_column: 0
        };

        new_container.render();

        new_container
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width as usize - (self.left_margin + self.right_margin);
        self.height = height as usize - (self.top_margin + self.bottom_margin);
        self.render();
    }

    pub fn print(&self) -> Result<()> {
        let mut cur_row = self.top_margin as u16;
        let mut skipped_rows = 0;
        for line in &self.rendered {
            if skipped_rows < self.scroll_row {
                skipped_rows += 1;
                continue;
            }
            let mut printable = String::new();
            let mut actual_width = 0;
            let mut skipped_width = 0;
            for c in line.graphemes(true) {
                if skipped_width < self.scroll_column {
                    skipped_width += UnicodeWidthStr::width(c);
                    continue;
                }
                if actual_width < self.width {
                    let char_len = UnicodeWidthStr::width(c);
                    if actual_width + char_len <= self.width {
                        actual_width += char_len;
                        printable.push_str(c);
                    }
                }
            } 

            queue!(
                stdout(),
                MoveTo(self.left_margin as u16, cur_row),
                Print(printable)
            )?;

            cur_row += 1;
            if cur_row as usize >= self.height + self.top_margin  {
                break;
            }
        }

        stdout().flush()?;

        Ok(())
    }

    pub fn scroll_pos(&self) -> (usize, usize) {
        (self.scroll_row, self.scroll_column)
    }

    pub fn set_margins(&mut self, top: usize, bottom: usize, left: usize, right: usize) {
        self.top_margin = top;
        self.bottom_margin = bottom;
        self.left_margin = left;
        self.right_margin = right;

        let size = terminal::size().unwrap();
        self.width = size.0 as usize - (self.left_margin + self.right_margin);
        self.height = size.1 as usize - (self.top_margin + self.bottom_margin);
    }

    pub fn scroll_left(&mut self) {
        if self.scroll_column > 0 {
            self.scroll_column -= 1;
        }
    }
    pub fn scroll_right(&mut self) {
        if self.content_width > self.width && self.scroll_column < self.content_width - self.width {
            self.scroll_column += 1;
        }
    }
    pub fn scroll_up(&mut self) {
        if self.scroll_row > 0 {
            self.scroll_row -= 1;
        }
    }
    pub fn scroll_down(&mut self) {
        let content_height = self.rendered.len();

        if content_height > self.height && self.scroll_row < content_height - self.height {
            self.scroll_row += 1;
        }
    }

    fn render(&mut self) {
        self.rendered = Vec::<String>::new();
        self.content_width = 0;

        for line in &self.lines {
            if line.wrapped == true {
                let wrapped_lines = pretty_wrap(&line.s, self.width);
                for wline in wrapped_lines {
                    let length = UnicodeWidthStr::width(&wline[..]);
                    if length > self.content_width {
                        self.content_width = length;
                    }
                    self.rendered.push(wline);
                }
            }
            else {
                let length = UnicodeWidthStr::width(&line.s[..]);
                if length > self.content_width {
                    self.content_width = length;
                }
                self.rendered.push(line.s.clone());
            }
        }
    }
}

pub struct TextUI {
    top_line: String,
    container: ContentContainer,
    bottom_line: String
}

impl Drop for TextUI {
    fn drop(&mut self) {
        execute!(
            stdout(),
            style::ResetColor,
            cursor::Show,
            terminal::LeaveAlternateScreen
        ).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

impl TextUI {
    pub fn init() -> Result<Self> {
        execute!(stdout(), EnterAlternateScreen);
        terminal::enable_raw_mode()?;
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::Hide)?;

        let container = ContentContainer::new();

        Ok(TextUI {
            top_line: String::new(),
            container: container,
            bottom_line: String::new()
        })
    }

    pub fn main_loop(&mut self) {
        loop {
            match read().unwrap() {
                Event::Resize(width, height) => {
                    self.container.resize(width, height);
    
                    let scroll = self.container.scroll_pos();
                    self.bottom_line = format!("Scroll: {}, {}", scroll.0, scroll.1);
    
                    self.redraw_window();
                },
                Event::Key(event) => {
                    match event.code {
                        KeyCode::Char('h') => {
                            self.container.scroll_left();
                            let scroll = self.container.scroll_pos();
                            self.bottom_line = format!("Scroll: {}, {}", scroll.0, scroll.1);
                            self.redraw_window();
                        },
                        KeyCode::Char('l') => {
                            self.container.scroll_right();
                            let scroll = self.container.scroll_pos();
                            self.bottom_line = format!("Scroll: {}, {}", scroll.0, scroll.1);
                            self.redraw_window();
                        },
                        KeyCode::Char('j') => {
                            self.container.scroll_down();
                            let scroll = self.container.scroll_pos();
                            self.bottom_line = format!("Scroll: {}, {}", scroll.0, scroll.1);
                            self.redraw_window();
                        },
                        KeyCode::Char('k') => {
                            self.container.scroll_up();
                            let scroll = self.container.scroll_pos();
                            self.bottom_line = format!("Scroll: {}, {}", scroll.0, scroll.1);
                            self.redraw_window();
                        },
                        KeyCode::Char(' ') => {
                            let raw_command = get_command_from_user().unwrap();
                            let mut print_error = false;
                            let mut error_msg = String::new();

                            match parse_command(&raw_command) {
                                Some(Command::Go(url)) => {

                                }
                                Some(Command::Quit) => {
                                    break;
                                }
                                Some(Command::Unknown(c)) => {
                                    print_error = true;
                                    error_msg = format!("Unknown command: {}", c);
                                }
                                None => {

                                }
                            }

                            //self.bottom_line = format!("Received command: {}", raw_command);
                            if print_error == true {
                                self.bottom_line = error_msg;
                            }
                            self.redraw_window().unwrap();
                        },
                        KeyCode::Esc => {
                            break;
                        },
                        _ => {}
                    }
                },
                _ => {
                    break;
                }
            }
        }
    }

    fn redraw_window(&self) -> Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        self.print_top_row()?;
        self.container.print()?;
        self.print_bottom_row()?;
        Ok(())
    }

    fn print_top_row(&self) -> Result<()> {
        queue!(
            stdout(),
            MoveTo(0, 0),
            Print(&self.top_line)
        )?;
        stdout().flush()?;

        Ok(())
    }

    fn print_bottom_row(&self) -> Result<()> {
        let size = terminal::size()?;
        queue!(
            stdout(),
            MoveTo(0, size.1),
            Print(&self.bottom_line)
        )?;
        stdout().flush()?;
    
        Ok(())
    }
}

fn get_command_from_user() -> Result<String> {
    let size = terminal::size()?;

    queue!(
        stdout(),
        MoveTo(0, size.1),
        terminal::Clear(ClearType::CurrentLine),
        Print("> "),
        cursor::Show
    ).unwrap();
    stdout().flush()?;

    //terminal::disable_raw_mode()?;

    let mut command = String::new();
    loop {
        match read()? {
            Event::Key(event) => {
                match event.code {
                    KeyCode::Esc => {
                        command.clear();
                        break;
                    },

                    KeyCode::Backspace => {
                        if let Some(c) = command.pop() {
                            let s = c.to_string();
                            let l = UnicodeWidthStr::width(&s[..]) as u16;
                            queue!(
                                stdout(),
                                cursor::MoveLeft(l)
                            )?;
                            for _ in 0..l {
                                queue!(
                                    stdout(),
                                    Print(' ')
                                )?;
                            }
                            queue!(
                                stdout(),
                                cursor::MoveLeft(l)
                            )?;
                            stdout().flush()?;
                        }
                    },

                    KeyCode::Enter => {
                        break;
                    },
                    
                    KeyCode::Char(c) => {
                        command.push(c);
                        execute!(
                            stdout(),
                            Print(c),
                        )?;
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    //terminal::enable_raw_mode()?;
    execute!(
        stdout(),
        cursor::Hide
    )?;

    Ok(command)
}

fn pretty_wrap(line: &str, width: usize) -> Vec::<String> {
    let mut results = Vec::<String>::new();

    if UnicodeWidthStr::width(line) <= width {
        results.push(line.to_string());
        return results;
    }

    let split = line.split_word_bounds().collect::<Vec<&str>>();
    let mut current_line = String::new();

    for word in split {
        let word_width = UnicodeWidthStr::width(word);
        
        if word_width > width {
            for c in word.graphemes(true) {
                let current_line_width = UnicodeWidthStr::width(&current_line[..]);
                let grapheme_width = UnicodeWidthStr::width(c);

                if current_line_width + grapheme_width > width {
                    results.push(current_line);
                    current_line = String::new();
                    current_line.push_str(c);
                }
                else {
                    current_line.push_str(c);
                }
            }
        }
        else {
            let current_line_width = UnicodeWidthStr::width(&current_line[..]);

            if current_line_width + word_width > width {
                results.push(current_line);
                current_line = String::new();
                current_line.push_str(word);
            }
            else {
                current_line.push_str(word);
            }
        }
    }
    if current_line.len() > 0 {
        results.push(current_line);
    }

    results
}

fn parse_command(s: &str) -> Option<Command> {
    if s.len() == 0 {
        return None;
    }

    let go_re = Regex::new(r"^\s*go? +(.+)").unwrap();
    let quit_re = Regex::new(r"^\s*qu?i?t?.*").unwrap();
    let generic_re = Regex::new(r"^\s*(\S+)").unwrap();

    if go_re.is_match(s) {
        let groups = go_re.captures(s).unwrap();
        let location = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());
        return Some(Command::Go(location));
    }
    else if quit_re.is_match(s) {
        return Some(Command::Quit);
    }
    else {
        if generic_re.is_match(s) {
            let groups = generic_re.captures(s).unwrap();
            let command = groups.get(1).map_or("".to_string(), |u| u.as_str().to_string());
            return Some(Command::Unknown(command.to_string()));
        }
        else {
            return None;
        }
    }
}