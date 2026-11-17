use crate::log::{error, info};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    Required,
    Optional,
    Incompatible,
    Process,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub id: String,
    pub rule: Rule,
    pub min_version: Option<String>,
}

impl Dependency {
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        if let Some(name) = raw.strip_prefix('!') {
            return Self::plain(name, Rule::Incompatible);
        }
        if let Some(name) = raw.strip_prefix('@') {
            return Self::plain(name, Rule::Process);
        }
        let (body, rule) = match raw.strip_suffix('?') {
            Some(body) => (body, Rule::Optional),
            None => (raw, Rule::Required),
        };
        match body.split_once(">=") {
            Some((id, version)) => Self {
                id: id.trim().to_owned(),
                rule,
                min_version: Some(version.trim().to_owned()).filter(|v| !v.is_empty()),
            },
            None => Self::plain(body, rule),
        }
    }

    fn plain(id: &str, rule: Rule) -> Self {
        Self {
            id: id.trim().to_owned(),
            rule,
            min_version: None,
        }
    }

    pub fn parse_list(raw: &str) -> Vec<Self> {
        raw.split(',')
            .filter(|s| !s.trim().is_empty())
            .map(Self::parse)
            .collect()
    }

    pub fn is_dependency(&self) -> bool {
        matches!(self.rule, Rule::Required | Rule::Optional)
    }
}

pub fn compare_versions(a: &str, b: &str) -> Ordering {
    fn parts(v: &str) -> Vec<u64> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map_while(|p| {
                let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().ok()
            })
            .collect()
    }
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let ord = a
            .get(i)
            .copied()
            .unwrap_or(0)
            .cmp(&b.get(i).copied().unwrap_or(0));
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

pub trait Node: Clone {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn dependencies(&self) -> &[Dependency];
    fn origin(&self) -> &Path;
}

#[derive(Debug)]
pub struct Catalog<T> {
    disabled: HashSet<String>,
    process: String,
    found: BTreeMap<String, T>,
}

impl<T: Node> Catalog<T> {
    pub fn new(disabled: &[String], process: &str) -> Self {
        Self {
            disabled: disabled.iter().map(|s| s.to_lowercase()).collect(),
            process: process.to_lowercase(),
            found: BTreeMap::new(),
        }
    }

    fn runs_in_process(&self, node: &T) -> bool {
        let mut filters = node
            .dependencies()
            .iter()
            .filter(|d| d.rule == Rule::Process)
            .peekable();
        if filters.peek().is_none() || self.process.is_empty() {
            return true;
        }
        filters.any(|d| {
            let name = d.id.to_lowercase();
            name.strip_suffix(".exe").unwrap_or(&name) == self.process
        })
    }

    pub fn add(&mut self, node: T) -> bool {
        let key = node.id().to_lowercase();
        if self.disabled.contains(&key) {
            info!("skipping disabled mod {}", node.id());
            return false;
        }
        if !self.runs_in_process(&node) {
            info!("skipping {}: not meant for this game", node.id());
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

    fn problem(&self, node: &T, dep: &Dependency) -> Option<String> {
        let found = self.found.get(&dep.id.to_lowercase());
        match dep.rule {
            Rule::Incompatible => found.map(|_| format!("incompatible with {}", dep.id)),
            Rule::Process => None,
            Rule::Required | Rule::Optional => {
                let Some(other) = found else {
                    return (dep.rule == Rule::Required)
                        .then(|| format!("dependency {} is missing", dep.id));
                };
                let min = dep.min_version.as_deref()?;
                (compare_versions(other.version(), min) == Ordering::Less)
                    .then(|| format!("needs {} {min} or newer, found {}", dep.id, other.version()))
            }
        }
        .filter(|_| node.id() != dep.id)
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
            if let Some(problem) = self.problem(node, dep) {
                error!("{} skipped: {problem}", node.id());
                marks.insert(key.to_owned(), Mark::Failed);
                return false;
            }
            if !dep.is_dependency() {
                continue;
            }
            let dep_key = dep.id.to_lowercase();
            let ok = !self.found.contains_key(&dep_key) || self.visit(&dep_key, marks, out);
            if !ok && dep.rule == Rule::Required {
                error!("{} skipped: dependency {} failed", node.id(), dep.id);
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
    files(dir, "dll")
}

pub fn files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, extension, out);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, extension, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::{Catalog, Dependency, Node, Rule, compare_versions};
    use std::cmp::Ordering;
    use std::path::{Path, PathBuf};

    #[derive(Debug, Clone)]
    struct Fake {
        id: String,
        version: String,
        deps: Vec<Dependency>,
        origin: PathBuf,
    }

    impl Node for Fake {
        fn id(&self) -> &str {
            &self.id
        }
        fn version(&self) -> &str {
            &self.version
        }
        fn dependencies(&self) -> &[Dependency] {
            &self.deps
        }
        fn origin(&self) -> &Path {
            &self.origin
        }
    }

    fn fake(id: &str, deps: &str) -> Fake {
        versioned(id, "1.0.0", deps)
    }

    fn versioned(id: &str, version: &str, deps: &str) -> Fake {
        Fake {
            id: id.to_owned(),
            version: version.to_owned(),
            deps: Dependency::parse_list(deps),
            origin: PathBuf::from(format!("{id}.dll")),
        }
    }

    fn ids(catalog: &Catalog<Fake>) -> Vec<String> {
        catalog.ordered().into_iter().map(|f| f.id).collect()
    }

    #[test]
    fn dependencies_load_first() {
        let mut c = Catalog::new(&[], "");
        c.add(fake("a", "b"));
        c.add(fake("b", "c"));
        c.add(fake("c", ""));
        assert_eq!(ids(&c), ["c", "b", "a"]);
    }

    #[test]
    fn missing_required_dependency_skips_mod() {
        let mut c = Catalog::new(&[], "");
        c.add(fake("a", "missing"));
        c.add(fake("b", ""));
        assert_eq!(ids(&c), ["b"]);
    }

    #[test]
    fn missing_optional_dependency_is_ignored() {
        let mut c = Catalog::new(&[], "");
        c.add(fake("a", "missing?"));
        assert_eq!(ids(&c), ["a"]);
    }

    #[test]
    fn cycle_skips_both() {
        let mut c = Catalog::new(&[], "");
        c.add(fake("a", "b"));
        c.add(fake("b", "a"));
        c.add(fake("c", ""));
        assert_eq!(ids(&c), ["c"]);
    }

    #[test]
    fn disabled_and_duplicates_are_rejected() {
        let mut c = Catalog::new(&["Off".to_owned()], "");
        assert!(!c.add(fake("off", "")));
        assert!(c.add(fake("x", "")));
        assert!(!c.add(fake("X", "")));
        assert_eq!(ids(&c), ["x"]);
    }

    #[test]
    fn dependents_of_failed_mods_fail() {
        let mut c = Catalog::new(&[], "");
        c.add(fake("a", "b"));
        c.add(fake("b", "missing"));
        assert_eq!(ids(&c), [] as [std::string::String; 0]);
    }

    #[test]
    fn parses_dependency_list() {
        let deps = Dependency::parse_list(" core , extra? ,");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].id, "core");
        assert_eq!(deps[0].rule, Rule::Required);
        assert_eq!(deps[1].id, "extra");
        assert_eq!(deps[1].rule, Rule::Optional);
    }

    #[test]
    fn minimum_version_is_enforced() {
        let mut c = Catalog::new(&[], "");
        c.add(versioned("lib", "1.4.2", ""));
        c.add(fake("new", "lib>=1.5"));
        c.add(fake("old", "lib>=1.4.0"));
        c.add(fake("soft", "lib>=2.0?"));
        assert_eq!(ids(&c), ["lib", "old"]);
    }

    #[test]
    fn incompatible_mods_are_skipped() {
        let mut c = Catalog::new(&[], "");
        c.add(fake("a", "!b"));
        c.add(fake("b", ""));
        c.add(fake("c", "!absent"));
        assert_eq!(ids(&c), ["b", "c"]);
    }

    #[test]
    fn process_filter() {
        let mut c = Catalog::new(&[], "Game");
        assert!(c.add(fake("here", "@game.exe")));
        assert!(!c.add(fake("there", "@Other")));
        assert!(c.add(fake("any", "")));
        assert_eq!(ids(&c), ["any", "here"]);
    }

    #[test]
    fn compares_versions() {
        assert_eq!(compare_versions("1.10", "1.9"), Ordering::Greater);
        assert_eq!(compare_versions("v2.0.0", "2"), Ordering::Equal);
        assert_eq!(compare_versions("1.2.3-beta", "1.2.4"), Ordering::Less);
    }

    #[test]
    fn parses_rules() {
        let deps = Dependency::parse_list("a>=1.2?, !b, @Game");
        assert_eq!(deps[0].rule, Rule::Optional);
        assert_eq!(deps[0].min_version.as_deref(), Some("1.2"));
        assert_eq!(deps[1].rule, Rule::Incompatible);
        assert_eq!(deps[2].rule, Rule::Process);
        assert_eq!(deps[2].id, "Game");
    }
}
