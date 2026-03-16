use dialoguer::theme::ColorfulTheme;
use owo_colors::OwoColorize;

pub struct Ui {
    color: bool,
    verbose: bool,
}

impl Ui {
    pub fn new(color: bool, verbose: bool) -> Self {
        Self { color, verbose }
    }

    pub fn theme(&self) -> ColorfulTheme {
        ColorfulTheme::default()
    }

    pub fn heading(&self, text: &str) {
        if self.color {
            println!("{}", text.bold().cyan());
        } else {
            println!("{text}");
        }
    }

    pub fn status(&self, label: &str, text: &str) {
        if self.color {
            println!("{} {}", format!("[{label}]").bold().yellow(), text);
        } else {
            println!("[{label}] {text}");
        }
    }

    pub fn success(&self, text: &str) {
        if self.color {
            println!("{}", text.green());
        } else {
            println!("{text}");
        }
    }

    pub fn warn(&self, text: &str) {
        if self.color {
            eprintln!("{}", text.yellow());
        } else {
            eprintln!("{text}");
        }
    }

    pub fn info(&self, text: &str) {
        println!("{text}");
    }

    pub fn debug(&self, text: &str) {
        if self.verbose {
            self.status("debug", text);
        }
    }
}
