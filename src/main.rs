use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "log", "json", "jsonl", "toml", "yaml", "yml", "rs",
    "py", "js", "ts", "tsx", "jsx", "html", "css", "xml", "csv", "sql", "sh",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    relative: String,
    bytes: u64,
    lines: usize,
}

fn is_text_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| TEXT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn visit(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut children = fs::read_dir(dir)
        .map_err(|e| format!("cannot read directory {}: {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("cannot enumerate {}: {e}", dir.display()))?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let path = child.path();
        let name = child.file_name().to_string_lossy().to_string();
        if name == ".git" || name == "target" || name == "node_modules" {
            continue;
        }
        let kind = child
            .file_type()
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        if kind.is_dir() {
            visit(root, &path, files)?;
        } else if kind.is_file() && is_text_file(&path) {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn collect_entries(root: &Path) -> Result<Vec<Entry>, String> {
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    let mut relatives = Vec::new();
    visit(root, root, &mut relatives)?;
    let mut entries = Vec::new();
    for relative in relatives {
        let path = root.join(&relative);
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        entries.push(Entry {
            relative: relative.to_string_lossy().replace('\\', "/"),
            bytes: text.len() as u64,
            lines: text.lines().count(),
        });
    }
    entries.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(entries)
}

fn manifest(entries: &[Entry]) -> String {
    let mut out = String::from("LOOM1\n");
    for entry in entries {
        out.push_str(&format!(
            "{}|{}|{}\n",
            entry.bytes, entry.lines, entry.relative
        ));
    }
    out
}

fn search(root: &Path, needle: &str, limit: usize) -> Result<Vec<String>, String> {
    let entries = collect_entries(root)?;
    let needle_lower = needle.to_ascii_lowercase();
    let mut hits = Vec::new();
    for entry in entries {
        let path = root.join(&entry.relative);
        let text = fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        for (index, line) in text.lines().enumerate() {
            if line.to_ascii_lowercase().contains(&needle_lower) {
                hits.push(format!("{}:{}:{}", entry.relative, index + 1, line.trim()));
                if hits.len() >= limit {
                    return Ok(hits);
                }
            }
        }
    }
    Ok(hits)
}

fn stats(entries: &[Entry]) -> BTreeMap<&'static str, u64> {
    let mut out = BTreeMap::new();
    out.insert("files", entries.len() as u64);
    out.insert("bytes", entries.iter().map(|entry| entry.bytes).sum());
    out.insert("lines", entries.iter().map(|entry| entry.lines as u64).sum());
    out
}

fn usage() {
    eprintln!(
        "memoryloom — local transcript indexing and literal search\n\n\
         USAGE:\n\
           memoryloom index <root> [--out memory.loom]\n\
           memoryloom search <root> <phrase...> [--limit N]\n\
           memoryloom stats <root>\n\n\
         Skips .git, target, and node_modules. Reads common text formats only."
    );
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn positional(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if arg == "--out" || arg == "--limit" {
            skip = true;
        } else if !arg.starts_with("--") {
            out.push(arg.clone());
        }
    }
    out
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let pos = positional(&args);
    match args.first().map(String::as_str) {
        Some("index") => {
            let root = PathBuf::from(pos.get(1).ok_or("index requires a root directory")?);
            let out = option_value(&args, "--out").unwrap_or_else(|| "memory.loom".to_string());
            let entries = collect_entries(&root)?;
            fs::write(&out, manifest(&entries)).map_err(|e| format!("cannot write {out}: {e}"))?;
            println!("indexed {} files into {out}", entries.len());
            Ok(())
        }
        Some("search") => {
            let root = PathBuf::from(pos.get(1).ok_or("search requires a root directory")?);
            let phrase = pos.get(2..).unwrap_or(&[]).join(" ");
            if phrase.is_empty() {
                return Err("search requires a phrase".to_string());
            }
            let limit = option_value(&args, "--limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(100);
            for hit in search(&root, &phrase, limit)? {
                println!("{hit}");
            }
            Ok(())
        }
        Some("stats") => {
            let root = PathBuf::from(pos.get(1).ok_or("stats requires a root directory")?);
            for (key, value) in stats(&collect_entries(&root)?) {
                println!("{key}: {value}");
            }
            Ok(())
        }
        Some("--help") | Some("-h") | None => {
            usage();
            Ok(())
        }
        Some(other) => Err(format!("unknown command {other}; try --help")),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("memoryloom: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("memoryloom-{name}-{}-{nonce}", process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn detects_text_files() {
        assert!(is_text_file(Path::new("notes.md")));
        assert!(!is_text_file(Path::new("cover.png")));
    }

    #[test]
    fn manifest_is_sorted_and_plain_text() {
        let entries = vec![
            Entry { relative: "b.md".into(), bytes: 2, lines: 1 },
            Entry { relative: "a.md".into(), bytes: 4, lines: 2 },
        ];
        let mut sorted = entries;
        sorted.sort_by(|a, b| a.relative.cmp(&b.relative));
        assert_eq!(manifest(&sorted), "LOOM1\n4|2|a.md\n2|1|b.md\n");
    }

    #[test]
    fn searches_text_recursively() {
        let root = temp_root("search");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested/note.md"), "hello\nopenshark handoff\n").unwrap();
        fs::write(root.join("ignored.bin"), "openshark").unwrap();
        let hits = search(&root, "OPENSHARK", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].starts_with("nested/note.md:2:"));
        fs::remove_dir_all(root).unwrap();
    }
}
