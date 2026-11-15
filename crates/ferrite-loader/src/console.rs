use crate::config::{self, Kind};
use crate::log::{self, Level};
use std::sync::{Mutex, PoisonError};

type Handler = fn(&[&str]);

static EXTRA: Mutex<Vec<(&'static str, &'static str, Handler)>> = Mutex::new(Vec::new());

pub fn print(text: &str) {
    log::write(Level::Info, "Console", text);
}

pub fn register(name: &'static str, usage: &'static str, handler: Handler) {
    EXTRA
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((name, usage, handler));
}

pub fn is_reserved(name: &str) -> bool {
    matches!(name, "config" | "loglevel" | "menu")
        || EXTRA
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .any(|(n, _, _)| *n == name)
}

pub fn help() -> Vec<String> {
    let mut lines = vec![
        "config [mod] [key] [value] - Lists, shows or changes mod settings.".to_owned(),
        "loglevel <debug|info|warning|error> - Sets the console log level.".to_owned(),
        "menu - Toggles the in-game Ferrite menu.".to_owned(),
    ];
    lines.extend(
        EXTRA
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(name, usage, _)| format!("{name} {usage}")),
    );
    lines
}

pub fn builtin(line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let Some((&name, args)) = parts.split_first() else {
        return false;
    };
    match name.to_lowercase().as_str() {
        "config" => config_command(args),
        "loglevel" => loglevel(args),
        "menu" => crate::overlay::toggle(),
        other => {
            let handler = EXTRA
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .find(|(n, _, _)| *n == other)
                .map(|(_, _, h)| *h);
            match handler {
                Some(h) => h(args),
                None => return false,
            }
        }
    }
    true
}

fn loglevel(args: &[&str]) {
    let level = match args.first().map(|a| a.to_lowercase()).as_deref() {
        Some("debug") => Level::Debug,
        Some("info") => Level::Info,
        Some("warning" | "warn") => Level::Warning,
        Some("error") => Level::Error,
        _ => {
            print("usage: loglevel <debug|info|warning|error>");
            return;
        }
    };
    log::set_console_level(level);
    print(&format!("console log level is {level}"));
}

fn config_command(args: &[&str]) {
    let output = config::with_mod_configs(|c| match args {
        [] => {
            let ids = c.ids();
            if ids.is_empty() {
                vec!["no mod configs yet".to_owned()]
            } else {
                ids
            }
        }
        [id] => {
            let entries = c.list(id);
            if entries.is_empty() {
                vec![format!("{id} has no settings")]
            } else {
                entries
                    .into_iter()
                    .map(|(k, v)| format!("  {k} = {v}"))
                    .collect()
            }
        }
        [id, key] => vec![match c.get(id, key) {
            Some(v) => format!("{id}.{key} = {v}"),
            None => format!("{id} has no setting {key}"),
        }],
        [id, key, value @ ..] => {
            let value = value.join(" ");
            match c.set(id, key, &value, Kind::Infer) {
                Ok(()) => {
                    c.notify(id);
                    vec![format!(
                        "{id}.{key} = {}",
                        c.get(id, key).unwrap_or_default()
                    )]
                }
                Err(e) => vec![format!("cannot save {id}: {e}")],
            }
        }
    })
    .unwrap_or_default();
    for line in output {
        print(&line);
    }
}
