use crate::settings::get_root;
use camino::Utf8PathBuf;
use globset::{GlobBuilder, GlobSetBuilder};
use ignore::Walk;
use notify::event::{CreateKind, ModifyKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher, event, recommended_watcher};
use rayon::prelude::*;
use serde::Serialize;
use shlex::Shlex;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::{
    collections::HashSet,
    sync::{Mutex, RwLock},
};
use tauri::{Emitter, Window};
use tokio::task::spawn_blocking;

const SEARCH_RESULT_LIMIT: usize = 1000;

#[derive(Default)]
pub struct SearchState {
    root: Mutex<Option<Utf8PathBuf>>,
    full_list: Arc<RwLock<HashSet<String>>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
    pub last_search_result: Arc<RwLock<HashSet<String>>>,
    pub last_search: RwLock<String>,
    window: Mutex<Option<Window>>,
}

#[derive(Default)]
pub struct SelectionState {
    pub selection: RwLock<HashSet<String>>,
}

impl SearchState {
    pub fn search_files_by_name_interactive(
        &self,
        globs: &str,
        window: Window,
    ) -> Result<Vec<String>, crate::Error> {
        *self.window.lock().unwrap() = Some(window);
        let root = get_root()?;
        if Some(&root) != (self.root.lock().unwrap()).as_ref() {
            let files: HashSet<_> = Walk::new(&root)
                .flatten()
                .flat_map(|e| {
                    if let Ok(meta) = e.metadata()
                        && meta.is_file()
                    {
                        e.path()
                            .strip_prefix(&root)
                            .map(|r| r.to_string_lossy().to_string())
                            .ok()
                    } else {
                        None
                    }
                })
                .collect();
            *self.full_list.write().unwrap() = files;
            *self.root.lock().unwrap() = Some(root.clone());

            let mut watcher = recommended_watcher({
                let root = root.clone();
                let full_list = Arc::clone(&self.full_list);
                let last_search_result = Arc::clone(&self.last_search_result);
                let opt_win = self.window.lock().unwrap().clone();
                let patt = self.last_search.read().unwrap().clone();
                move |res: Result<event::Event, notify::Error>| match res {
                    Ok(event) => match event.kind {
                        EventKind::Create(CreateKind::File) => {
                            println!("create {:?}", event.paths);
                            for path in event.paths {
                                if let Ok(path) = path.strip_prefix(&root) {
                                    add_paths(
                                        &mut full_list.write().unwrap(),
                                        &mut last_search_result.write().unwrap(),
                                        opt_win.as_ref(),
                                        &patt,
                                        [path.to_string_lossy().to_string()],
                                    );
                                }
                            }
                        }
                        EventKind::Remove(_) => {
                            println!("remove {:?}", event.paths);
                            for path in event.paths {
                                if let Ok(path) = path.strip_prefix(&root) {
                                    remove_paths(
                                        &mut full_list.write().unwrap(),
                                        &mut last_search_result.write().unwrap(),
                                        opt_win.as_ref(),
                                        [path.to_string_lossy().to_string()],
                                    );
                                }
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::To))
                            if event.paths[0].is_file() =>
                        {
                            println!("rename to {:?}", event.paths);
                            if let Ok(path) = event.paths[0].strip_prefix(&root) {
                                add_paths(
                                    &mut full_list.write().unwrap(),
                                    &mut last_search_result.write().unwrap(),
                                    opt_win.as_ref(),
                                    &patt,
                                    [path.to_string_lossy().to_string()],
                                );
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                            println!("rename from {:?}", event.paths);
                            if let Ok(path) = event.paths[0].strip_prefix(&root) {
                                remove_paths(
                                    &mut full_list.write().unwrap(),
                                    &mut last_search_result.write().unwrap(),
                                    opt_win.as_ref(),
                                    [path.to_string_lossy().to_string()],
                                );
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                            if event.paths[1].is_file() =>
                        {
                            println!("rename both {:?}", event.paths);
                            if let Ok(path) = event.paths[0].strip_prefix(&root) {
                                remove_paths(
                                    &mut full_list.write().unwrap(),
                                    &mut last_search_result.write().unwrap(),
                                    opt_win.as_ref(),
                                    [path.to_string_lossy().to_string()],
                                );
                            }
                            if let Ok(path) = event.paths[1].strip_prefix(&root) {
                                add_paths(
                                    &mut full_list.write().unwrap(),
                                    &mut last_search_result.write().unwrap(),
                                    opt_win.as_ref(),
                                    &patt,
                                    [path.to_string_lossy().to_string()],
                                );
                            }
                        }
                        _ => {}
                    },
                    Err(e) => {
                        println!("Error {e:?}")
                    }
                }
            })
            .unwrap();

            watcher
                .watch(Path::new(&root), RecursiveMode::Recursive)
                .unwrap();

            *self.watcher.lock().unwrap() = Some(watcher);
        }

        let globs = globs.to_string();
        let files = self.full_list.read().unwrap();
        let mut res = find_by_globs(&files, &globs);
        if let Ok(ref mut v) = res {
            *self.last_search.write().unwrap() = globs.to_string();
            *self.last_search_result.write().unwrap() = v.iter().cloned().collect();
            if v.len() > SEARCH_RESULT_LIMIT {
                return Err(crate::Error::Limit {
                    item: "files".to_string(),
                    requested: v.len(),
                    limit: SEARCH_RESULT_LIMIT,
                });
            }
        }
        res
    }

    pub fn search_files_by_name(&self, globs: &str) -> Result<Vec<String>, crate::Error> {
        let root = get_root()?;
        if Some(&root) != (self.root.lock().unwrap()).as_ref() {
            let files: HashSet<_> = Walk::new(&root)
                .flatten()
                .flat_map(|e| {
                    if let Ok(meta) = e.metadata()
                        && meta.is_file()
                    {
                        e.path()
                            .strip_prefix(&root)
                            .map(|r| r.to_string_lossy().to_string())
                            .ok()
                    } else {
                        None
                    }
                })
                .collect();
            *self.full_list.write().unwrap() = files;
            *self.root.lock().unwrap() = Some(root.clone());
        }

        let globs = globs.to_string();
        let files = self.full_list.read().unwrap();
        find_by_globs(&files, &globs)
    }
}

fn dual_globsets(globs: &str) -> Result<(globset::GlobSet, globset::GlobSet), crate::error::Error> {
    let lex = Shlex::new(globs);
    let mut no_set = GlobSetBuilder::new();
    let mut yes_set = GlobSetBuilder::new();
    for pat in lex {
        if let Some(pat) = pat.strip_prefix('!') {
            let glob = GlobBuilder::new(&format!("*{pat}*"))
                .case_insensitive(true)
                .backslash_escape(false)
                .literal_separator(false)
                .build()?;
            no_set.add(glob);
        } else {
            let glob = GlobBuilder::new(&format!("*{pat}*"))
                .case_insensitive(true)
                .backslash_escape(false)
                .literal_separator(false)
                .build()?;
            yes_set.add(glob);
        }
    }
    let yes_set = yes_set.build()?;
    let no_set = no_set.build()?;
    Ok((yes_set, no_set))
}

#[derive(Serialize, Clone)]
enum SearchResultUpdate {
    Add(String),
    Remove(String),
}

pub fn remove_paths<I, S>(
    coll1: &mut HashSet<String>,
    coll2: &mut HashSet<String>,
    opt_win: Option<&Window>,
    files: I,
) where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for file in files {
        coll1.remove(file.as_ref());
        coll2.remove(file.as_ref());
        if let Some(w) = opt_win {
            let _ = w.emit(
                "search_result_update",
                SearchResultUpdate::Remove(file.as_ref().to_string()),
            );
        }
    }
}
pub fn add_paths<I>(
    coll1: &mut HashSet<String>,
    coll2: &mut HashSet<String>,
    opt_win: Option<&Window>,
    patt: &str,
    files: I,
) where
    I: IntoIterator<Item = String>,
{
    let files: Vec<_> = files.into_iter().collect();
    if let Ok((yes_set, no_set)) = dual_globsets(patt) {
        files
            .iter()
            .flat_map(|path| {
                if yes_set.matches(path).len() == yes_set.len() && no_set.matches(path).is_empty() {
                    Some(path)
                } else {
                    None
                }
            })
            .for_each(|p| {
                coll2.insert(p.clone());
                if let Some(w) = opt_win {
                    let _ = w.emit("search_result_update", SearchResultUpdate::Add(p.clone()));
                }
            });
    }
    for file in files {
        coll1.insert(file);
    }
}

fn find_by_globs(paths: &HashSet<String>, globs: &str) -> Result<Vec<String>, crate::Error> {
    let (yes_set, no_set) = dual_globsets(globs)?;
    let out: Vec<_> = paths
        .par_iter()
        .flat_map(|path| {
            if yes_set.matches(path).len() == yes_set.len() && no_set.matches(path).is_empty() {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect();
    Ok(out)
}

pub static SEARCH_STATE: LazyLock<SearchState> = LazyLock::new(Default::default);
pub static SELECTION_STATE: LazyLock<SelectionState> = LazyLock::new(Default::default);

// #[tauri::command]
// pub fn search_files_by_name_sync(globs: &str) -> Result<Vec<String>, crate::Error> {
//     SEARCH_STATE.search_files_by_name(globs)
// }
pub async fn search_files_by_name(globs: &str) -> Result<Vec<String>, crate::Error> {
    spawn_blocking({
        let globs = globs.to_string();
        move || SEARCH_STATE.search_files_by_name(&globs)
    })
    .await?
}

#[tauri::command]
pub async fn search_files_by_name_interactive(
    globs: &str,
    window: Window,
) -> Result<Vec<String>, crate::Error> {
    spawn_blocking({
        let globs = globs.to_string();
        move || SEARCH_STATE.search_files_by_name_interactive(&globs, window)
    })
    .await?
}

#[tauri::command]
pub fn selection_add(sel: String) -> bool {
    SELECTION_STATE.selection.write().unwrap().insert(sel)
}
#[tauri::command]
pub fn selection_remove(sel: &str) -> bool {
    SELECTION_STATE.selection.write().unwrap().remove(sel)
}
#[tauri::command]
pub fn selection_clear() {
    SELECTION_STATE.selection.write().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::find_by_globs;
    use shlex::Shlex;

    fn shlex_split(input: &str) -> Vec<String> {
        let shlex = Shlex::new(input);
        shlex.collect()
    }

    #[test]
    fn shlex_base() {
        assert_eq!(shlex_split("Hello world"), &["Hello", "world"]);
        assert_eq!(shlex_split(r#"Hello world"#), &["Hello", "world"]);
        assert_eq!(shlex_split(r#"Hello "world""#), &["Hello", "world"]);
        assert_eq!(shlex_split(r#"hi \"world\""#), &["hi", "\"world\""]);
        assert_eq!(shlex_split(r#"hi !world"#), &["hi", "!world"]);
        assert_eq!(shlex_split(r#"hi !"world""#), &["hi", "!world"]);
    }

    #[test]
    fn find_by_globs_test() {
        let paths: HashSet<String> =
            HashSet::from([r#"local work\savv2\something.xlsx"#.to_string()]);

        assert_eq!(find_by_globs(&paths, "savv something").unwrap().len(), 1);
        assert_eq!(find_by_globs(&paths, "something savv").unwrap().len(), 1);
        assert_eq!(find_by_globs(&paths, "something").unwrap().len(), 1);
        assert_eq!(find_by_globs(&paths, "something not").unwrap().len(), 0);
        assert_eq!(find_by_globs(&paths, "something !savv").unwrap().len(), 0);
        assert_eq!(find_by_globs(&paths, "").unwrap().len(), 1);
        assert_eq!(find_by_globs(&paths, "work").unwrap().len(), 1);
        assert_eq!(find_by_globs(&paths, r"work\\savv").unwrap().len(), 1);
    }
}
