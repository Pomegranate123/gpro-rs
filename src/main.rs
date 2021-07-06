#![feature(iter_intersperse)]
mod app;
mod conf;
mod parser;
mod ui;
mod util;

use crate::{
    app::App,
    conf::{Config, Theme},
    parser::Song,
    util::event::{self, Event, Events},
};
use getopts::Options;
use std::{env, error::Error, fs, io, time::Duration};
use termion::{event::Key, raw::IntoRawMode};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Terminal,
};

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse commandline arguments
    let args: Vec<String> = std::env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optopt("c", "config", "set config file", "PATH");
    opts.optopt("", "default-config", "write the default config", "PATH");
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("d", "debug", "");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f)
        }
    };

    if matches.opt_present("h") {
        print_usage(program, opts);
        return Ok(());
    }

    if let Some(arg) = matches.opt_str("default-config") {
        let path = std::path::PathBuf::from(&arg);
        Config::write_default(&path)?;
        println!("Default config has been written to {}", path.display());
        return Ok(());
    }

    let config = match matches.opt_str("c") {
        Some(arg) => {
            let path = std::path::PathBuf::from(&arg);
            if !path.exists() {
                panic!("Path '{}' doesn't exist", arg)
            }
            Config::load(&path)?
        }
        None => {
            let config_path = env::var("GPRO_CONFIG")
                .unwrap_or_else(|_| String::from("/home/pomegranate/git/gpro-rs/conf.yml"));
            Config::load(&std::path::PathBuf::from(config_path)).unwrap_or_default()
        }
    };

    let mut app = App::new(config.clone());

    if matches.opt_present("d") {
        let song = Song::from(
            fs::read_to_string("/home/pomegranate/Dropbox/Songbook/NL Selectie/Opw785.txt")
                .unwrap(),
        );
        let wrapped = ui::wrap_lines(&song.content, Rect::new(0, 0, 50, 50), 15);
        println!("{:#?}", wrapped.get(0).unwrap().to_spans(&Theme::default()));
        return Ok(());
    }

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);

    let mut term = Terminal::new(backend)?;
    let events = Events::with_config(event::Config {
        exit_key: *config.keybinds.quit,
        tick_rate: Duration::from_millis(250),
    });

    term.clear().unwrap();
    loop {
        term.draw(|f| {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Length(20), Constraint::Min(80)].as_ref())
                .split(f.size());

            ui::draw_search_list(f, &mut app, layout[0]);
            ui::draw_song_block(f, &app, layout[1]);
        })?;

        match events.next()? {
            Event::Input(key) => {
                if app.searching {
                    match key {
                        Key::Char(c) => match c {
                            '\n' => (),
                            _ => {
                                app.input.push(c);
                                app.files.items = app.search(&app.input);
                                app.files.select(None);
                            }
                        },
                        Key::Backspace => {
                            app.input.pop();
                            app.files.items = app.search(&app.input);
                        }
                        Key::Esc => app.searching = false,
                        _ => (),
                    }
                } else if key == *app.config.keybinds.down {
                    app.files.forward(1);
                    if app.config.auto_select_song {
                        app.load_selected()
                    }
                } else if key == *app.config.keybinds.up {
                    app.files.back(1);
                    if app.config.auto_select_song {
                        app.load_selected()
                    }
                } else if key == *app.config.keybinds.jump_down {
                    app.files.forward(20);
                    if app.config.auto_select_song {
                        app.load_selected()
                    }
                } else if key == *app.config.keybinds.jump_up {
                    app.files.back(20);
                    if app.config.auto_select_song {
                        app.load_selected()
                    }
                } else if key == *app.config.keybinds.next {
                    app.load_selected()
                } else if key == *app.config.keybinds.back {
                    app.path_back()
                } else if key == *app.config.keybinds.col_size_inc {
                    app.extra_column_size += 1;
                } else if key == *app.config.keybinds.col_size_dec {
                    if app.extra_column_size > 0 {
                        app.extra_column_size -= 1;
                    }
                } else if key == *app.config.keybinds.search {
                    app.searching = true
                } else if key == *app.config.keybinds.quit {
                    break;
                };
            }
            Event::Tick => (),
        }
    }
    Ok(())
}
