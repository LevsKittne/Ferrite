use crate::deps::{self, Catalog, Dependency, Node};
use crate::log::{error, warning};
use crate::mono::{Class, Mono};
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

impl Node for ModEntry {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    fn origin(&self) -> &Path {
        &self.file
    }
}

pub fn discover(
    mono: &Mono,
    mods_dir: &Path,
    mod_class: Class,
    attr_class: Class,
    disabled: &[String],
) -> Vec<ModEntry> {
    let mut catalog = Catalog::new(disabled, &crate::game_name());
    for file in deps::dlls(mods_dir) {
        let Some(image) = std::fs::read(&file)
            .ok()
            .and_then(|bytes| mono.load_from_memory(&file.to_string_lossy(), &bytes).ok())
            .map(|(_, image)| image)
        else {
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
            catalog.add(ModEntry {
                name: mono
                    .string_field(attr, attr_class, "Name")
                    .unwrap_or_else(|| id.clone()),
                version: mono
                    .string_field(attr, attr_class, "Version")
                    .unwrap_or_default(),
                dependencies: [
                    ("Dependencies", ""),
                    ("Incompatibilities", "!"),
                    ("Processes", "@"),
                ]
                .iter()
                .flat_map(|(field, prefix)| {
                    mono.string_array_field(attr, attr_class, field)
                        .into_iter()
                        .map(move |d| Dependency::parse(&format!("{prefix}{d}")))
                })
                .collect(),
                id,
                class,
                file: file.clone(),
            });
        }
    }
    catalog.ordered()
}
