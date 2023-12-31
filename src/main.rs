#![windows_subsystem = "windows"]

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use std::process::exit;

use nwd::NwgUi;
use nwg::NativeUi;

use log::{debug, error};
use simplelog;
use std::fs::OpenOptions;

use chrono;
use csv::Writer;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use ini::Ini;

#[derive(Default, NwgUi)]
pub struct Dialog {
    #[nwg_control(size: (300, 200), title: "Please Enter Your Username", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [Dialog::handle_close],
                  OnKeyEsc: [Dialog::handle_esc_key],
                  OnKeyEnter: [Dialog::handle_enter_key] )]
    window: nwg::Window,

    #[nwg_control(text: "Username: ", size: (280, 25), position: (10, 10))]
    label: nwg::Label,

    #[nwg_control(text: "", size: (200, 25), position: (90, 10), focus: true, limit: 32)]
    username_edit: nwg::TextInput,

    #[nwg_control(text: "Ok", size: (80, 30), position: (210, 160))]
    #[nwg_events( OnButtonClick: [Dialog::handle_ok_button] )]
    ok_button: nwg::Button,
}

impl Dialog {
    fn handle_close(&self) {
        nwg::stop_thread_dispatch();
        debug!("User terminated by closing window");
        exit(0);
    }

    fn handle_esc_key(&self) {
        nwg::stop_thread_dispatch();
        debug!("User terminated with ESC key");
        exit(0);
    }

    fn handle_enter_key(&self) {
        nwg::stop_thread_dispatch();
        debug!("User accepted with ENTER key");
    }

    fn handle_ok_button(&self) {
        nwg::stop_thread_dispatch();
        debug!("User accepted with OK button");
    }
}

fn get_username_dialog() -> String {
    let mut app;
    loop {
        app = Dialog::build_ui(Default::default()).unwrap_or_else(|_| {
            error!("Failed to build UI");
            exit(1);
        });
        nwg::dispatch_thread_events();

        if !app.username_edit.text().is_empty() {
            break;
        }
    }

    return app.username_edit.text();
}

#[derive(Serialize, Deserialize)]
struct Config {
    app_path: String,
    output_path: Box<Path>,
}

fn load_config(path: PathBuf) -> Config {
    let output_path = match home::home_dir() {
        Some(mut x) => {
            x.push("Desktop");
            x.push("AppTimer.csv");
            x
        }
        None => PathBuf::from("AppTimer.csv"),
    };

    if !path.is_file() {
        let mut conf = Ini::new();
        conf.with_section(Some("AppTimer"))
            .set(
                "app_path",
                format!("C:\\Windows\\system32\\notepad.exe {}", path.display()),
            )
            .set("output_path", output_path.to_str().unwrap());
        conf.write_to_file(&path).unwrap_or_else(|_| {
            error!("Failed to create default configuration file");
            exit(1)
        });
    }

    let config: Ini = Ini::load_from_file(path).unwrap_or_else(|_| {
        error!("Failed to read configuration file");
        exit(1);
    });

    for (sec, prop) in &config {
        if sec == Some("AppTimer") {
            let app_path = prop.get("app_path").unwrap();
            let output_path = prop.get("output_path").unwrap();
            return Config {
                app_path: app_path.to_string(),
                output_path: PathBuf::from(output_path).into(),
            };
        }
    }

    error!("Invalid configuration file");
    exit(1);
}

fn main() {
    const CONFIG_FILENAME: &str = "settings.ini";
    let conf_file_path = match home::home_dir() {
        Some(mut x) => {
            x.push("AppData");
            x.push("Roaming");
            x.push("AppTimer");
            x.push(CONFIG_FILENAME);
            x
        }
        None => PathBuf::from(CONFIG_FILENAME),
    };
    let conf_parent_dir = conf_file_path.parent().unwrap();
    if !conf_parent_dir.is_dir() {
        fs::create_dir(conf_parent_dir).expect("Error creating default configuration");
    }
    let config = load_config(conf_file_path);
    let log_file_path = match home::home_dir() {
        Some(mut x) => {
            x.push("AppData");
            x.push("Roaming");
            x.push("AppTimer");
            x.push("AppTimer.log");
            x
        }
        None => PathBuf::from("AppTimer.log"),
    };

    let log_file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(log_file_path)
        .unwrap();
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            simplelog::LevelFilter::Debug,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        simplelog::WriteLogger::new(
            simplelog::LevelFilter::Debug,
            simplelog::Config::default(),
            log_file,
        ),
    ])
    .unwrap();

    nwg::init().unwrap_or_else(|_| {
        error!("Unable to initalize GUI window.");
        exit(1);
    });
    nwg::Font::set_global_family("Segoe UI").unwrap_or_else(|_| {
        error!("Unable to set font");
        exit(1);
    });
    let username = get_username_dialog();

    let write_header = !config.output_path.is_file();
    let mut writer = Writer::from_writer(vec![]);
    if write_header {
        writer
            .write_record(&[
                "startTimestamp",
                "endTimestamp",
                "elapsedMinutes",
                "username",
            ])
            .unwrap();
    }

    let start = chrono::offset::Local::now();
    let app_path = config.app_path.clone();
    println!("{:?}", app_path);
    Command::new(app_path)
        .output()
        .unwrap_or_else(|err| {
            let path = config.app_path;
            error!("Failed to spawn subprocess. ({})\n\t{}", path, err);
            exit(1);
        });

    let end = chrono::offset::Local::now();
    let elapsed = end - start;
    let start_str = start.to_rfc3339();
    let end_str = end.to_rfc3339();

    writer
        .write_record(&[
            start_str.to_string(),
            end_str.to_string(),
            elapsed.num_minutes().to_string(),
            username,
        ])
        .unwrap_or_else(|err| {
            error!("Failed to write result to CSV. {}", err);
            exit(1);
        });

    let data = String::from_utf8(writer.into_inner().unwrap()).unwrap();
    let mut out_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.output_path)
        .unwrap_or_else(|err| {
            error!("Failed to open output file. {}", err);
            exit(1)
        });
    out_file.write(data.as_bytes()).unwrap();
}
