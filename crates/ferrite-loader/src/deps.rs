use crate::log::{error, info};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Dependency {
    pub id: String,
    pub optional: bool,
}

impl Dependency {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().strip_suffix('?') {
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

    pub fn parse_list(raw: &str) -> Vec<Self> {
        raw.split(',')
            .filter(|s| !s.trim().is_empty())
            .map(Self::parse)
            .collect()
    }
}

pub trait Node: Clone {
    fn id(&self) -> &str;
    fn dependencies(&self) -> &[Dependency];
    fn origin(&self) -> &Path;
}

#[derive(Debug)]
pub struct Catalog<T> {
    disabled: HashSet<String>,
    found: BTreeMap<String, T>,
}

impl<T: Node> Catalog<T> {
    pub fn new(disabled: &[String]) -> Self {
        Self {
            disabled: disabled.iter().map(|s| s.to_lowercase()).collect(),
            found: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, node: T) -> bool {
        let key = node.id().to_lowercase();
        if self.disabled.contains(&key) {
            info!("skipping disabled mod {}", node.id());
            return false;
        }
        if let Some(previous) = self.found.get(&key) {
            error!(
                "duplicate mod id {} in {} (already in {}), skipped",
                node.id(),
                node.origin().display(),
                previous.origin().display()
            );
            return false;
        }
        self.found.insert(key, node);
        true
    }

    pub fn ordered(&self) -> Vec<T> {
        let mut marks = HashMap::new();
        let mut out = Vec::with_capacity(self.found.len());
        for key in self.found.keys() {
            self.visit(key, &mut marks, &mut out);
        }
        out
    }

    fn visit(&self, key: &str, marks: &mut HashMap<String, Mark>, out: &mut Vec<T>) -> bool {
        match marks.get(key) {
            Some(Mark::Done) => return true,
            Some(Mark::Failed) => return false,
            Some(Mark::Visiting) => {
                error!("dependency cycle at {key}");
                return false;
            }
            None => {}
        }
        let Some(node) = self.found.get(key) else {
            return false;
        };
        marks.insert(key.to_owned(), Mark::Visiting);
        for dep in node.dependencies() {
            let dep_key = dep.id.to_lowercase();
            let ok = self.found.contains_key(&dep_key) && self.visit(&dep_key, marks, out);
            if !ok && !dep.optional {
                error!(
                    "{} skipped: dependency {} is missing or failed",
                    node.id(),
                    dep.id
                );
                marks.insert(key.to_owned(), Mark::Failed);
                return false;
            }
        }
        marks.insert(key.to_owned(), Mark::Done);
        out.push(node.clone());
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Visiting,
    Done,
    Failed,
}

pub fn dlls(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::{Catalog, Dependency, Node};
    use std::path::{Path, PathBuf};

    #[derive(Debug, Clone)]
    struct Fake {
        id: String,
        deps: Vec<Dependency>,
        origin: PathBuf,
    }

    impl Node for Fake {
        fn id(&self) -> &str {
            &self.id
        }
        fn dependencies(&self) -> &[Dependency] {
            &self.deps
        }
        fn origin(&self) -> &Path {
            &self.origin
        }
    }

    fn fake(id: &str, deps: &str) -> Fake {
        Fake {
            id: id.to_owned(),
            deps: Dependency::parse_list(deps),
            origin: PathBuf::from(format!("{id}.dll")),
        }
    }

    fn ids(catalog: &Catalog<Fake>) -> Vec<String> {
        catalog.ordered().into_iter().map(|f| f.id).collect()
    }

    #[test]
    fn dependencies_load_first() {
        let mut c = Catalog::new(&[]);
        c.add(fake("a", "b"));
        c.add(fake("b", "c"));
        c.add(fake("c", ""));
        assert_eq!(ids(&c), ["c", "b", "a"]);
    }

    #[test]
    fn missing_required_dependency_skips_mod() {
        let mut c = Catalog::new(&[]);
        c.add(fake("a", "missing"));
        c.add(fake("b", ""));
        assert_eq!(ids(&c), ["b"]);
    }

    #[test]
    fn missing_optional_dependency_is_ignored() {
        let mut c = Catalog::new(&[]);
        c.add(fake("a", "missing?"));
        assert_eq!(ids(&c), ["a"]);
    }

    #[test]
    fn cycle_skips_both() {
        let mut c = Catalog::new(&[]);
        c.add(fake("a", "b"));
        c.add(fake("b", "a"));
        c.add(fake("c", ""));
        assert_eq!(ids(&c), ["c"]);
    }

    #[test]
    fn disabled_and_duplicates_are_rejected() {
        let mut c = Catalog::new(&["Off".to_owned()]);
        assert!(!c.add(fake("off", "")));
        assert!(c.add(fake("x", "")));
        assert!(!c.add(fake("X", "")));
        assert_eq!(ids(&c), ["x"]);
    }

    #[test]
    fn dependents_of_failed_mods_fail() {
        let mut c = Catalog::new(&[]);
        c.add(fake("a", "b"));
        c.add(fake("b", "missing"));
        assert_eq!(ids(&c), [] as [std::string::String; 0]);
    }

    #[test]
    fn parses_dependency_list() {
        let deps = Dependency::parse_list(" core , extra? ,");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].id, "core");
        assert!(!deps[0].optional);
        assert_eq!(deps[1].id, "extra");
        assert!(deps[1].optional);
    }
}
