use crate::log::Level;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};
use toml_edit::{DocumentMut, Item, Table, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LoaderConfig {
    pub loader: LoaderSection,
    pub log: LogSection,
    pub mods: ModsSection,
    pub patchers: PatchersSection,
    pub debug: DebugSection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Entrypoint {
    Early,
    Scene,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct LoaderSection {
    pub enabled: bool,
    pub console: bool,
    pub unity_log: bool,
    pub entrypoint: Entrypoint,
    pub menu: bool,
    pub menu_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogSection {
    pub console_level: Level,
    pub file_level: Level,
    pub keep_previous: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModsSection {
    pub disabled: Vec<String>,
    pub hot_reload: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PatchersSection {
    pub dump_assemblies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugSection {
    pub enabled: bool,
    pub address: String,
    pub suspend: bool,
}

impl Default for LoaderSection {
    fn default() -> Self {
        Self {
            enabled: true,
            console: true,
            unity_log: true,
            entrypoint: Entrypoint::Early,
            menu: true,
            menu_key: "F1".to_owned(),
        }
    }
}

impl Default for LogSection {
    fn default() -> Self {
        Self {
            console_level: Level::Info,
            file_level: Level::Debug,
            keep_previous: true,
        }
    }
}

impl Default for DebugSection {
    fn default() -> Self {
        Self {
            enabled: false,
            address: "127.0.0.1:55555".to_owned(),
            suspend: false,
        }
    }
}

impl LoaderConfig {
    pub fn load(path: &Path) -> (Self, Option<String>) {
        if let Ok(text) = std::fs::read_to_string(path) {
            return match toml::from_str::<Self>(&text) {
                Ok(cfg) => {
                    cfg.add_missing_keys(path, &text);
                    (cfg, None)
                }
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            };
        }
        let cfg = Self::default();
        if let Ok(text) = toml::to_string_pretty(&cfg) {
            let _ = std::fs::write(path, text);
        }
        (cfg, None)
    }

    fn add_missing_keys(&self, path: &Path, text: &str) {
        let (Ok(mut doc), Ok(full)) = (
            text.parse::<DocumentMut>(),
            toml::to_string_pretty(self).map(|t| t.parse::<DocumentMut>()),
        ) else {
            return;
        };
        let Ok(full) = full else { return };
        let mut changed = false;
        for (section, item) in full.iter() {
            let Some(defaults) = item.as_table() else {
                continue;
            };
            let target = doc
                .entry(section)
                .or_insert_with(|| Item::Table(Table::new()));
            let Some(target) = target.as_table_mut() else {
                continue;
            };
            for (key, value) in defaults {
                if !target.contains_key(key) {
                    target.insert(key, value.clone());
                    changed = true;
                }
            }
        }
        if changed {
            let _ = std::fs::write(path, doc.to_string());
        }
    }

    pub fn option(&self, key: &str) -> Option<String> {
        match key {
            "enabled" => Some(self.loader.enabled.to_string()),
            "console" => Some(self.loader.console.to_string()),
            "unity_log" => Some(self.loader.unity_log.to_string()),
            "entrypoint" => Some(
                match self.loader.entrypoint {
                    Entrypoint::Early => "early",
                    Entrypoint::Scene => "scene",
                }
                .to_owned(),
            ),
            "dump_assemblies" => Some(self.patchers.dump_assemblies.to_string()),
            "hot_reload" => Some(self.mods.hot_reload.to_string()),
            "menu_key" => Some(self.loader.menu_key.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    String,
    Bool,
    Int,
    Float,
    Infer,
}

impl Kind {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::String,
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Float,
            _ => Self::Infer,
        }
    }

    fn parse(self, raw: &str, existing: Option<&Value>) -> Value {
        let kind = match (self, existing) {
            (Self::Infer, Some(Value::Boolean(_))) => Self::Bool,
            (Self::Infer, Some(Value::Integer(_))) => Self::Int,
            (Self::Infer, Some(Value::Float(_))) => Self::Float,
            (Self::Infer, Some(_)) => Self::String,
            (Self::Infer, None) => {
                return raw
                    .parse::<bool>()
                    .map(Value::from)
                    .or_else(|_| raw.parse::<i64>().map(Value::from))
                    .or_else(|_| raw.parse::<f64>().map(Value::from))
                    .unwrap_or_else(|_| Value::from(raw));
            }
            (kind, _) => kind,
        };
        match kind {
            Self::Bool => raw.parse::<bool>().map_or_else(|_| raw.into(), Value::from),
            Self::Int => raw.parse::<i64>().map_or_else(|_| raw.into(), Value::from),
            Self::Float => raw.parse::<f64>().map_or_else(|_| raw.into(), Value::from),
            Self::String | Self::Infer => Value::from(raw),
        }
    }
}

fn render(value: &Value) -> String {
    match value {
        Value::String(s) => s.value().clone(),
        Value::Boolean(b) => b.value().to_string(),
        Value::Integer(i) => i.value().to_string(),
        Value::Float(f) => format!("{:?}", f.value()),
        other => other.clone().decorated("", "").to_string(),
    }
}

#[derive(Debug)]
struct File {
    doc: DocumentMut,
    stamp: Option<SystemTime>,
}

#[derive(Debug, Default)]
pub struct ModConfigs {
    dir: PathBuf,
    cache: HashMap<String, File>,
    last_poll: Option<Instant>,
    pending: Vec<String>,
}

fn stamp(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn split(key: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = key.split('.').map(str::trim).collect();
    let leaf = parts.pop().unwrap_or_default();
    (parts, leaf)
}

fn section<'a>(table: &'a Table, path: &[&str]) -> Option<&'a Table> {
    path.iter()
        .try_fold(table, |t, seg| t.get(seg).and_then(Item::as_table))
}

fn section_mut<'a>(table: &'a mut Table, path: &[&str]) -> Option<&'a mut Table> {
    path.iter().try_fold(table, |t, seg| {
        t.entry(seg)
            .or_insert_with(|| Item::Table(Table::new()))
            .as_table_mut()
    })
}

fn flatten(table: &Table, prefix: &str, out: &mut Vec<(String, String)>) {
    for (key, item) in table {
        let full = if prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{prefix}.{key}")
        };
        match item {
            Item::Table(t) => flatten(t, &full, out),
            Item::Value(v) => out.push((full, render(v))),
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub description: String,
}

fn comment(table: &Table, key: &str) -> String {
    table
        .key(key)
        .and_then(|k| k.leaf_decor().prefix())
        .and_then(|p| p.as_str())
        .map(|p| {
            p.lines()
                .filter_map(|l| l.trim().strip_prefix('#'))
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn settings(table: &Table, prefix: &str, out: &mut Vec<Setting>) {
    for (key, item) in table {
        let full = if prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{prefix}.{key}")
        };
        match item {
            Item::Table(t) => settings(t, &full, out),
            Item::Value(v) => out.push(Setting {
                description: comment(table, key),
                key: full,
                value: render(v),
            }),
            _ => {}
        }
    }
}

pub fn edit_loader(path: &Path, edit: impl FnOnce(&mut DocumentMut)) -> std::io::Result<()> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut doc: DocumentMut = text.parse().map_err(std::io::Error::other)?;
    edit(&mut doc);
    std::fs::write(path, doc.to_string())
}

pub fn read_loader(path: &Path) -> LoaderConfig {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

impl ModConfigs {
    pub fn settings(&mut self, id: &str) -> Vec<Setting> {
        let mut out = Vec::new();
        settings(self.file(id).doc.as_table(), "", &mut out);
        out
    }

    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            ..Self::default()
        }
    }

    fn path(&self, id: &str) -> PathBuf {
        let safe: String = id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || "-_.".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.toml"))
    }

    fn read(path: &Path) -> File {
        File {
            doc: std::fs::read_to_string(path)
                .ok()
                .and_then(|t| t.parse().ok())
                .unwrap_or_default(),
            stamp: stamp(path),
        }
    }

    fn file(&mut self, id: &str) -> &mut File {
        let path = self.path(id);
        self.cache
            .entry(id.to_owned())
            .or_insert_with(|| Self::read(&path))
    }

    pub fn get(&mut self, id: &str, key: &str) -> Option<String> {
        let (path, leaf) = split(key);
        let file = self.file(id);
        section(file.doc.as_table(), &path)?
            .get(leaf)?
            .as_value()
            .map(render)
    }

    pub fn list(&mut self, id: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        flatten(self.file(id).doc.as_table(), "", &mut out);
        out
    }

    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect();
        for id in self.cache.keys() {
            if !ids.iter().any(|i| i.eq_ignore_ascii_case(id)) {
                ids.push(id.clone());
            }
        }
        ids.sort();
        ids
    }

    pub fn set(&mut self, id: &str, key: &str, raw: &str, kind: Kind) -> std::io::Result<()> {
        let (path, leaf) = split(key);
        let file = self.file(id);
        let Some(table) = section_mut(file.doc.as_table_mut(), &path) else {
            return Err(std::io::Error::other(format!("{key} is not a section")));
        };
        let existing = table.get(leaf).and_then(Item::as_value);
        let mut value = kind.parse(raw, existing);
        if let Some(old) = existing {
            if render(old) == render(&value) && old.type_name() == value.type_name() {
                return Ok(());
            }
            *value.decor_mut() = old.decor().clone();
        }
        match table.get_mut(leaf) {
            Some(item) => *item = Item::Value(value),
            None => {
                table.insert(leaf, Item::Value(value));
            }
        }
        self.save(id)
    }

    pub fn describe(&mut self, id: &str, key: &str, description: &str) -> std::io::Result<()> {
        let (path, leaf) = split(key);
        let file = self.file(id);
        let Some(table) = section_mut(file.doc.as_table_mut(), &path) else {
            return Ok(());
        };
        let Some(mut key) = table.key_mut(leaf) else {
            return Ok(());
        };
        let has_comment = key
            .leaf_decor()
            .prefix()
            .and_then(|p| p.as_str())
            .is_some_and(|p| p.contains('#'));
        if has_comment || description.trim().is_empty() {
            return Ok(());
        }
        let comment = description.lines().fold(String::new(), |mut acc, l| {
            acc.push_str("# ");
            acc.push_str(l.trim_end());
            acc.push('\n');
            acc
        });
        key.leaf_decor_mut().set_prefix(comment);
        self.save(id)
    }

    fn save(&mut self, id: &str) -> std::io::Result<()> {
        let path = self.path(id);
        let Some(file) = self.cache.get_mut(id) else {
            return Ok(());
        };
        std::fs::create_dir_all(&self.dir)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, file.doc.to_string())?;
        std::fs::rename(tmp, &path)?;
        file.stamp = stamp(&path);
        Ok(())
    }

    pub fn notify(&mut self, id: &str) {
        if !self.pending.iter().any(|p| p == id) {
            self.pending.push(id.to_owned());
        }
    }

    pub fn poll(&mut self) -> Vec<String> {
        let mut changed = std::mem::take(&mut self.pending);
        let now = Instant::now();
        if self
            .last_poll
            .is_some_and(|t| now.duration_since(t) < Duration::from_secs(1))
        {
            return changed;
        }
        self.last_poll = Some(now);
        let ids: Vec<String> = self.cache.keys().cloned().collect();
        for id in ids {
            let path = self.path(&id);
            let current = stamp(&path);
            if self.cache.get(&id).is_some_and(|f| f.stamp != current) {
                self.cache.insert(id.clone(), Self::read(&path));
                changed.push(id);
            }
        }
        changed
    }
}

pub static MOD_CONFIGS: Mutex<Option<ModConfigs>> = Mutex::new(None);

pub fn with_mod_configs<R>(f: impl FnOnce(&mut ModConfigs) -> R) -> Option<R> {
    MOD_CONFIGS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
        .map(f)
}

#[cfg(test)]
mod tests {
    use super::{Kind, ModConfigs};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ferrite-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn sections_descriptions_and_comments_survive() {
        let dir = temp_dir("cfg");
        let mut c = ModConfigs::new(dir.clone());
        c.set("m", "speed", "1.5", Kind::Float).unwrap();
        c.describe("m", "speed", "Walk speed").unwrap();
        c.set("m", "ui.scale", "2", Kind::Int).unwrap();
        c.set("m", "speed", "3.0", Kind::Infer).unwrap();
        let text = std::fs::read_to_string(dir.join("m.toml")).unwrap();
        assert!(text.contains("# Walk speed\nspeed = 3.0"), "{text}");
        assert!(text.contains("[ui]"), "{text}");
        assert_eq!(c.get("m", "ui.scale").as_deref(), Some("2"));
        assert_eq!(
            c.list("m"),
            [
                ("speed".to_owned(), "3.0".to_owned()),
                ("ui.scale".to_owned(), "2".to_owned())
            ]
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reloads_edited_file() {
        let dir = temp_dir("reload");
        let mut c = ModConfigs::new(dir.clone());
        c.set("m", "on", "true", Kind::Bool).unwrap();
        assert_eq!(c.poll(), Vec::<String>::new());
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(dir.join("m.toml"), "on = false\n").unwrap();
        assert_eq!(c.poll(), ["m"]);
        assert_eq!(c.get("m", "on").as_deref(), Some("false"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
