use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use toml::{Table, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LoaderConfig {
    pub loader: LoaderSection,
    pub mods: ModsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoaderSection {
    pub enabled: bool,
    pub console: bool,
    pub unity_log: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModsSection {
    pub disabled: Vec<String>,
}

impl Default for LoaderSection {
    fn default() -> Self {
        Self {
            enabled: true,
            console: true,
            unity_log: true,
        }
    }
}

impl LoaderConfig {
    pub fn load(path: &Path) -> (Self, Option<String>) {
        if let Ok(text) = std::fs::read_to_string(path) {
            return match toml::from_str(&text) {
                Ok(cfg) => (cfg, None),
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            };
        }
        let cfg = Self::default();
        if let Ok(text) = toml::to_string_pretty(&cfg) {
            let _ = std::fs::write(path, text);
        }
        (cfg, None)
    }

    pub fn option(&self, key: &str) -> Option<String> {
        match key {
            "enabled" => Some(self.loader.enabled.to_string()),
            "console" => Some(self.loader.console.to_string()),
            "unity_log" => Some(self.loader.unity_log.to_string()),
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
}

impl Kind {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Float,
            _ => Self::String,
        }
    }

    fn parse(self, raw: &str) -> Value {
        match self {
            Self::Bool => raw
                .parse()
                .map_or_else(|_| Value::String(raw.to_owned()), Value::Boolean),
            Self::Int => raw
                .parse()
                .map_or_else(|_| Value::String(raw.to_owned()), Value::Integer),
            Self::Float => raw
                .parse()
                .map_or_else(|_| Value::String(raw.to_owned()), Value::Float),
            Self::String => Value::String(raw.to_owned()),
        }
    }
}

#[derive(Debug, Default)]
pub struct ModConfigs {
    dir: PathBuf,
    cache: HashMap<String, Table>,
}

impl ModConfigs {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            cache: HashMap::new(),
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

    fn table(&mut self, id: &str) -> &mut Table {
        let path = self.path(id);
        self.cache.entry(id.to_owned()).or_insert_with(|| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|t| t.parse().ok())
                .unwrap_or_default()
        })
    }

    pub fn get(&mut self, id: &str, key: &str) -> Option<String> {
        self.table(id).get(key).map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
    }

    pub fn set(&mut self, id: &str, key: &str, raw: &str, kind: Kind) -> std::io::Result<()> {
        let value = kind.parse(raw);
        let table = self.table(id);
        if table.get(key) == Some(&value) {
            return Ok(());
        }
        table.insert(key.to_owned(), value);
        let text = toml::to_string_pretty(table).unwrap_or_default();
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path(id);
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)
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
