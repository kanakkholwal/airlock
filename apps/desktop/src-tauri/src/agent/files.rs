//! Finding files in the folders the person shared, for `@` mentions in chat.

use super::config;
use serde::Serialize;
use specta::Type;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::AppHandle;

const MAX_DEPTH: usize = 4;
const MAX_VISITED: usize = 5_000;
const SKIPPED: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".svn",
    "__pycache__",
    "dist",
    "build",
];

#[derive(Debug, Clone, Serialize, Type)]
pub struct FileHit {
    pub path: String,
    pub name: String,
    /// The shared folder it sits in.
    pub folder: String,
}

fn modified(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Files whose name contains `query` (any file when it is empty), newest first. Walks a few
/// levels down and skips build and hidden folders, so a huge folder never freezes the list.
pub fn find_files(roots: &[PathBuf], query: &str, limit: usize) -> Vec<FileHit> {
    let needle = query.trim().to_lowercase();
    let mut hits: Vec<(SystemTime, FileHit)> = Vec::new();
    let mut visited = 0;
    for root in roots {
        let folder = root.file_name().map_or_else(
            || root.to_string_lossy().into_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
        let mut queue = vec![(root.clone(), 0usize)];
        while let Some((dir, depth)) = queue.pop() {
            let Ok(read) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in read.flatten() {
                visited += 1;
                if visited > MAX_VISITED {
                    break;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
                    continue;
                }
                let path = entry.path();
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    if depth + 1 < MAX_DEPTH {
                        queue.push((path, depth + 1));
                    }
                } else if kind.is_file()
                    && (needle.is_empty() || name.to_lowercase().contains(&needle))
                {
                    hits.push((
                        modified(&path),
                        FileHit {
                            path: path.to_string_lossy().into_owned(),
                            name,
                            folder: folder.clone(),
                        },
                    ));
                }
            }
        }
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0));
    hits.into_iter().take(limit).map(|(_, h)| h).collect()
}

#[tauri::command]
#[specta::specta]
pub fn agent_search_files(app: AppHandle, query: String) -> Vec<FileHit> {
    let cfg = config::load(&app);
    let roots: Vec<PathBuf> = cfg.folders.iter().map(PathBuf::from).collect();
    find_files(&roots, &query, 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_files_by_name_in_nested_folders_and_skips_build_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("notes/deep")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("plan.md"), "x").unwrap();
        std::fs::write(root.join("notes/deep/Plan-two.txt"), "x").unwrap();
        std::fs::write(root.join("node_modules/plan-junk.js"), "x").unwrap();
        std::fs::write(root.join(".secret-plan"), "x").unwrap();

        let hits = find_files(&[root.clone()], "plan", 10);
        let names: Vec<_> = hits.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(hits.len(), 2, "{names:?}");
        assert!(names.contains(&"plan.md") && names.contains(&"Plan-two.txt"));
    }

    #[test]
    fn an_empty_query_lists_files_and_the_limit_applies() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std::fs::write(tmp.path().join(format!("f{i}.txt")), "x").unwrap();
        }
        assert_eq!(find_files(&[tmp.path().to_path_buf()], "", 3).len(), 3);
    }

    #[test]
    fn no_shared_folders_means_no_files() {
        assert!(find_files(&[], "anything", 5).is_empty());
    }
}
