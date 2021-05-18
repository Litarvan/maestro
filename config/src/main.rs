/// The configuration utility allows to easily create configuration files for the kernel's
/// compilation.

mod option;
mod view;

use std::fs::File;
use std::io::Write;
use std::io::stdout;
use std::io;
use std::process;

use crossterm::Result;
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal;
use crossterm::tty::IsTty;

use option::MenuOption;

/// The path to the file containing configuration file options.
const CONFIG_OPTIONS_FILE: &str = "config_options.json";
/// The path to the output configuration file.
const CONFIG_FILE: &str = ".config";

/// Structure representing the configuration environment, storage data for rendering and
/// configuration itself.
struct ConfigEnv {
	/// The list of available options in the root menu.
	options: Vec<MenuOption>,

    /// The terminal's view.
    view: view::View,
}

impl ConfigEnv {
	/// Creates a new instance.
	/// `options` is the list of options.
	/// `values` is the values for all options.
	pub fn new(options: Vec<MenuOption>) -> Self {
		Self {
			options: options,

            view: view::View::new(true), // TODO
		}
	}

	/// Returns the option with name `name` within the root menu.
	fn get_root_option(&self, name: &String) -> Option<&MenuOption> {
		for m in &self.options {
			if m.name == *name {
				return Some(&m);
			}
		}

		None
	}

	/// Saves configuration to file.
	pub fn save(&self) -> io::Result<()> {
		let mut data = String::new();
		for o in &self.options {
			o.serialize(&mut data);
		}

		let mut file = File::create(CONFIG_FILE)?;
		file.write_all(data.as_str().as_bytes())
	}
}

/// Displays the configuration utility.
fn display(options: Vec<MenuOption>) -> Result<()> {
	execute!(stdout(), EnterAlternateScreen)?;
	terminal::enable_raw_mode()?;

	let mut env = ConfigEnv::new(options);
	env.view.render();
    env.view.wait_for_event()
}

fn main() {
	let s = stdout();

	if !s.is_tty() {
		eprintln!("Standard output must be a terminal!");
		process::exit(1);
	}

	let size = terminal::size();
	if size.is_err() {
		eprintln!("Cannot retrieve terminal size!");
		process::exit(1);
	}
	let (width, height) = size.unwrap();

	if width < view::DISPLAY_MIN_WIDTH || height < view::DISPLAY_MIN_HEIGHT {
		eprintln!(concat!("The terminal must be at least 80x25 characters in size to run the
configuration tool"));
		process::exit(1);
	}

	let options_results = option::from_file(CONFIG_OPTIONS_FILE);
	if let Err(err) = options_results {
		eprintln!("{}", err);
		process::exit(1);
	}
	let options = options_results.unwrap();

	if display(options).is_err() {
		eprintln!("Terminal error!");
		process::exit(1);
	}
}
