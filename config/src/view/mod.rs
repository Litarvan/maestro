/// This module implements the rendering of the menus on in the terminal.

mod button;

use std::cmp::min;
use std::io::stdout;

use crossterm::Result;
use crossterm::cursor;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::event;
use crossterm::execute;
use crossterm::style::Color;
use crossterm::style::SetBackgroundColor;
use crossterm::style::SetForegroundColor;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal;

use crate::option::MenuOption;

use button::BackButton;
use button::Button;
use button::EnterButton;
use button::ExitButton;
use button::SaveButton;

/// Minimum display width.
pub const DISPLAY_MIN_WIDTH: u16 = 80;
/// Minimum display height.
pub const DISPLAY_MIN_HEIGHT: u16 = 25;

/// Renders the `screen too small` error.
fn render_screen_error() -> Result<()> {
    execute!(stdout(),
        SetForegroundColor(Color::Black),
        SetBackgroundColor(Color::Red))?;
    execute!(stdout(), Clear(ClearType::All))?;
    execute!(stdout(), cursor::MoveTo(0, 0))?;
    println!(concat!("Display is too small! (minimum 80x25)"));
    execute!(stdout(), cursor::MoveTo(0, 1))
}

/// Resets the terminal before quitting.
fn reset() -> Result<()> {
    terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)
}

/// Represents a menu.
struct MenuView {
	/// The identifier of the menu.
	menu_id: String,
	/// The X position of the cursor.
	cursor_y: usize,
}

/// Represents the terminal's view.
pub struct View {
	/// The Y position of the cursor.
	cursor_x: usize,
	/// Stores the current menu view. The last element is the view being shown.
	current_menu_view: Vec<MenuView>,
	/// The list of buttons on the interface.
	buttons: Vec<Box<dyn Button>>,

    /// Whether the modifications are saved.
    saved: bool,
}

impl View {
    /// Creates a new instance.
    pub fn new(new_file: bool) -> Self {
        Self {
			cursor_x: 0,
			current_menu_view: vec! {
				MenuView {
					menu_id: "".to_string(),
					cursor_y: 0,
				}
			},
			buttons: vec!{
				Box::new(EnterButton {}),
				Box::new(BackButton {}),
				Box::new(SaveButton {}),
				Box::new(ExitButton {}),
			},

            saved: !new_file,
        }
    }

    /// Renders the menu's background and x/y/width/height for the options.
    /// `width` is the width of the terminal.
    /// `height` is the height of the terminal.
    fn render_background(&self, width: u16, height: u16) -> Result<(u16, u16, u16, u16)> {
        execute!(stdout(),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::Blue))?;
        execute!(stdout(), Clear(ClearType::All))?;

        execute!(stdout(), cursor::MoveTo(1, 0))?;
        print!("Maestro kernel configuration utility");

        let menu_x = 2;
        let menu_y = 2;
        let menu_width = width - (menu_x * 2);
        let menu_height = height - (menu_y * 2);
        execute!(stdout(),
            SetForegroundColor(Color::Black),
            SetBackgroundColor(Color::Grey))?;
        for i in 0..menu_height {
            execute!(stdout(), cursor::MoveTo(menu_x, menu_y + i))?;
            for j in 0..menu_width {
                if j == 0 || j == menu_width - 1 {
                    print!("|");
                } else if i == 0 || i == menu_height - 1 {
                    print!("-");
                } else {
                    print!(" ");
                }
            }
        }

        execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 1))?;
        print!("<Up>/<Down>/<Left>/<Right>: Move around");
        execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 2))?;
        print!("<Space>: Toggle");
        execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 3))?;
        print!("<Enter>: Go to menu");
        execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 4))?;
        print!("<Backspace>: Go to parent menu");

        let options_x = menu_x + 6;
        let options_y = menu_y + 6;
        let options_width = menu_width - options_x;
        let options_height = menu_height - options_y;
        Ok((options_x, options_y, options_width, options_height))
    }

	/// Returns the current menu.
	fn get_current_menu(&self) -> Option<&MenuOption> {
		let mut option: Option<&MenuOption> = None;

		for i in 1..self.current_menu_view.len() {
			option = {
				let id = self.current_menu_view[i].menu_id.clone();

				if let Some(o) = option {
					o.get_suboption(&id)
				} else {
					self.get_root_option(&id)
				}
			};

			if option.is_none() {
				panic!("Internal error");
			}
		}

		option
	}

	/// Returns the number of options in the current menu.
	fn get_current_options(&self) -> &Vec<MenuOption> {
		if let Some(menu) = &self.get_current_menu() {
			&menu.suboptions
		} else {
			&self.options
		}
	}

	/// Returns the current menu view.
	fn get_current_view(&mut self) -> &mut MenuView {
		let last = self.current_menu_view.len() - 1;
		&mut self.current_menu_view[last]
	}

	/// Renders the options. The arguments define the frame in which the options will be rendered.
	fn render_options(&mut self, opt_x: u16, opt_y: u16, opt_width: u16, opt_height: u16)
		-> Result<()> {
        execute!(stdout(), cursor::MoveTo(opt_x, opt_y))?;
        print!("Current menu:");
		for m in &self.current_menu_view {
            print!("{}", m.menu_id); // TODO Print display name
            print!(" / ");
        }
        println!();

		// TODO Limit rendering and add scrolling
		let options_count = self.get_current_options().len();
		for i in 0..options_count {
			execute!(stdout(), cursor::MoveTo(opt_x, opt_y + 2 + i as u16))?;
			if i == self.get_current_view().cursor_y {
				execute!(stdout(),
					SetForegroundColor(Color::Grey),
					SetBackgroundColor(Color::Black))?;
			} else {
				execute!(stdout(),
					SetForegroundColor(Color::Black),
					SetBackgroundColor(Color::Grey))?;
			}

			let option = &self.get_current_options()[i];
			option.print("TODO"); // TODO Get value
		}

		// TODO Print current option description

		execute!(stdout(),
			SetForegroundColor(Color::Black),
			SetBackgroundColor(Color::Grey))?;
		// TODO Scrolling

		Ok(())
	}

	/// Renders the options. `x` and `y` are the coordinates of the buttons.
	fn render_buttons(&mut self, x: u16, y: u16) -> Result<()> {
		execute!(stdout(), cursor::MoveTo(x, y))?;
		for i in 0..self.buttons.len() {
			if i == self.cursor_x {
				execute!(stdout(),
					SetForegroundColor(Color::Black),
					SetBackgroundColor(Color::Red))?;
			}
			print!("<{}>", self.buttons[i].get_name());

			execute!(stdout(),
				SetForegroundColor(Color::Black),
				SetBackgroundColor(Color::Grey))?;
			if i < self.buttons.len() - 1 {
				print!(" ");
			}
		}
		println!();

		Ok(())
	}

	/// Renders the menu.
	pub fn render(&mut self) -> Result<()> {
		let (width, height) = terminal::size()?;

		if width < DISPLAY_MIN_WIDTH || height < DISPLAY_MIN_HEIGHT {
			render_screen_error()
		} else {
			let (opt_x, opt_y, opt_width, opt_height) = self.render_background(width, height)?;
			self.render_options(opt_x, opt_y, opt_width, opt_height);

			let buttons_x = opt_x;
			let buttons_y = opt_y + opt_height;
			self.render_buttons(buttons_x, buttons_y);

			execute!(stdout(), cursor::MoveTo(opt_x,
				opt_y + self.get_current_view().cursor_y as u16))
		}
	}

	/// Moves the cursor up. `n` is the number of lines to move up.
	fn move_up(&mut self, mut n: usize) {
		let curr = self.get_current_view().cursor_y;
        if curr > 0 {
            self.get_current_view().cursor_y -= min(n, curr);
            self.render();
        }
	}

	/// Moves the cursor down. `n` is the number of lines to move down.
	fn move_down(&mut self, n: usize) {
		let max = self.get_current_options().len() - 1;
		let curr = self.get_current_view().cursor_y;
        if curr < max {
            self.get_current_view().cursor_y = min(curr + n, max);
            self.render();
        }
	}

	/// Moves the cursor left.
	fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
            self.render();
        }
	}

	/// Moves the cursor right.
	fn move_right(&mut self) {
        if self.cursor_x < self.buttons.len() - 1 {
            self.cursor_x += 1;
            self.render();
        }
	}

	/// Moves the cursor to the top.
	fn move_top(&mut self) {
		let curr = self.get_current_view().cursor_y;
		if curr > 0 {
            self.get_current_view().cursor_y = 0;
            self.render();
		}
	}

	/// Moves the cursor to the bottom.
	fn move_bottom(&mut self) {
		let max = self.get_current_options().len() - 1;
		let curr = self.get_current_view().cursor_y;
		if curr < max {
            self.get_current_view().cursor_y = max;
            self.render();
		}
	}

	/// Toggles the selected option.
	fn toggle(&mut self) {
		// TODO Mutate the environement
	}

	/// Performs the action of the select button.
	fn press_button(&mut self) {
		let button_index = self.cursor_x;
		// TODO Clean
		unsafe {
			let button = &mut *(&mut *self.buttons[button_index] as *mut dyn Button);
			(*button).on_action(self);
		}
	}

	/// Enters the selected menu.
	pub fn push_menu(&mut self) {
		let menu_index = self.get_current_view().cursor_y;
		let menu_type = self.get_current_options()[menu_index].option_type.clone();
		if menu_type == "menu" {
			let menu_id = self.get_current_options()[menu_index].name.clone();
			self.current_menu_view.push(MenuView {
				menu_id: menu_id,
				cursor_y: 0,
			});
			self.render();
		}
	}

	/// Goes back to the parent menu.
	pub fn pop_menu(&mut self) {
		if self.current_menu_view.len() > 1 {
			self.current_menu_view.pop();
			self.render();
		}
	}

    /// Waits for a keyboard or resize event.
    pub fn wait_for_event(&self) -> Result<()> {
        loop {
            match event::read()? {
                Event::Key(event) => {
                    match event.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.move_up(1);
                        },
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.move_down(1);
                        },
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.move_left();
                        },
                        KeyCode::Right => {
                            self.move_right();
                        },

                        KeyCode::Char('l') => {
                            if event.modifiers == KeyModifiers::CONTROL {
                                self.render();
                            } else {
                                self.move_right();
                            }
                        },

                        KeyCode::PageUp => {
                            self.move_up(10);
                        },
                        KeyCode::PageDown => {
                            self.move_down(10);
                        },
                        KeyCode::Home => {
                            self.move_top();
                        },
                        KeyCode::End => {
                            self.move_bottom();
                        },

                        KeyCode::Char(' ') => {
                            self.toggle();
                        },
                        KeyCode::Enter => {
                            self.press_button();
                        },
                        KeyCode::Backspace => {
                            self.pop_menu();
                        },
                        KeyCode::Esc => {
                            self.pop_menu();
                        },

                        _ => {},
                    }
                },

                Event::Resize(_, _) => {
                    self.render();
                },

                _ => {},
            }
        }
    }

    pub fn exit(&self) -> Result<()> {
        if !self.saved {
            // TODO Ask for confirmation
        }

        reset()
    }
}

impl Drop for View {
    fn drop(&mut self) {
        reset();
    }
}
