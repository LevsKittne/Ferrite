use crate::log::{error, info, warning};
use crate::mono::{Class, Mono};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub dependencies: Vec<Dependency>,
    pub class: Class,
    pub file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub id: String,
    pub optional: bool,
}

impl Dependency {
    fn parse(raw: &str) -> Self {
        match raw.strip_suffix('?') {
            Some(id) => Self {
                id: id.trim().to_owned(),
                optional: true,
            },
            None => Self {
                id: raw.trim().to_owned(),
                optional: false,
            },
        }
    }
}

fn assemblies(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            assemblies(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
        {
            out.push(path);
        }
    }
}

pub fn discover(
    mono: &Mono,
    mods_dir: &Path,
    mod_class: Class,
    attr_class: Class,
    disabled: &[String],
) -> Vec<ModEntry> {
    let disabled: HashSet<String> = disabled.iter().map(|s| s.to_lowercase()).collect();
    let mut files = Vec::new();
    assemblies(mods_dir, &mut files);
    let mut found: BTreeMap<String, ModEntry> = BTreeMap::new();
    for file in files {
        let Some(image) = mono.open_image(&file.to_string_lossy()) else {
            warning!("skipping {}: not a .NET assembly", file.display());
            continue;
        };
        for class in mono
            .classes(image)
            .filter(|&c| mono.is_concrete_subclass(c, mod_class))
        {
            let type_name = mono.full_name(class);
            let Some(attr) = mono.attribute(class, attr_class) else {
                warning!(
                    "{type_name} in {} has no [Mod] attribute, skipped",
                    file.display()
                );
                continue;
            };
            let id = mono
                .string_field(attr, attr_class, "Id")
                .unwrap_or_default();
            if id.is_empty() {
                error!("{type_name} has an empty mod id, skipped");
                continue;
            }
            if disabled.contains(&id.to_lowercase()) {
                info!("skipping disabled mod {id}");
                continue;
            }
            if let Some(previous) = found.get(&id.to_lowercase()) {
                error!(
                    "duplicate mod id {id} in {} (already in {}), skipped",
                    file.display(),
                    previous.file.display()
                );
                continue;
            }
            let entry = ModEntry {
                name: mono
                    .string_field(attr, attr_class, "Name")
                    .unwrap_or_else(|| id.clone()),
                version: mono
                    .string_field(attr, attr_class, "Version")
                    .unwrap_or_default(),
                dependencies: mono
                    .string_array_field(attr, attr_class, "Dependencies")
                    .iter()
                    .map(|d| Dependency::parse(d))
                    .collect(),
                id: id.clone(),
                class,
                file: file.clone(),
            };
            found.insert(id.to_lowercase(), entry);
        }
    }
    order(&found)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Visiting,
    Done,
    Failed,
}

fn order(found: &BTreeMap<String, ModEntry>) -> Vec<ModEntry> {
    fn visit(
        key: &str,
        found: &BTreeMap<String, ModEntry>,
        marks: &mut HashMap<String, Mark>,
        out: &mut Vec<ModEntry>,
    ) -> bool {
        match marks.get(key) {
            Some(Mark::Done) => return true,
            Some(Mark::Failed) => return false,
            Some(Mark::Visiting) => {
                error!("dependency cycle at {key}");
                return false;
            }
            None => {}
        }
        let Some(entry) = found.get(key) else {
            return false;
        };
        marks.insert(key.to_owned(), Mark::Visiting);
        for dep in &entry.dependencies {
            let dep_key = dep.id.to_lowercase();
            let ok = found.contains_key(&dep_key) && visit(&dep_key, found, marks, out);
            if !ok && !dep.optional {
                error!(
                    "{} skipped: dependency {} is missing or failed",
                    entry.id, dep.id
                );
                marks.insert(key.to_owned(), Mark::Failed);
                return false;
            }
        }
        marks.insert(key.to_owned(), Mark::Done);
        out.push(entry.clone());
        true
    }

    let mut marks = HashMap::new();
    let mut out = Vec::with_capacity(found.len());
    for key in found.keys() {
        visit(key, found, &mut marks, &mut out);
    }
    out
}
