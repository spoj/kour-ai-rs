use crate::settings::get_root;
use camino::Utf8PathBuf;
use globset::{GlobBuilder, GlobSetBuilder};
use ignore::Walk;
use notify::event::{CreateKind, ModifyKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher, event, recommended_watcher};
use rayon::prelude::*;
use shlex::Shlex;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::{
    collections::HashSet,
    sync::{Mutex, RwLock},
};
use tokio::task::spawn_blocking;

const SEARCH_RESULT_LIMIT: usize = 1000;

#[derive(Default)]
pub struct SearchState {
    root: Mutex<Option<Utf8PathBuf>>,
    full_list: Arc<RwLock<HashSet<String>>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
    pub last_search_result: RwLock<Vec<String>>,
    pub last_search: RwLock<String>,
}

#[derive(Default)]
pub struct SelectionState {
    pub selection: RwLock<HashSet<String>>,
}
impl SearchState {
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

            let mut watcher = recommended_watcher({
                let root = root.clone();
                let full_list = Arc::clone(&self.full_list);
                move |res: Result<event::Event, notify::Error>| match res {
                    Ok(event) => match event.kind {
                        EventKind::Create(CreateKind::File) => {
                            println!("create {:?}", event.paths);
                            for path in event.paths {
                                if let Ok(path) = path.strip_prefix(&root) {
                                    full_list
                                        .write()
                                        .unwrap()
                                        .insert(path.to_string_lossy().to_string());
                                }
                            }
                        }
                        EventKind::Remove(_) => {
                            println!("remove {:?}", event.paths);
                            for path in event.paths {
                                if let Ok(path) = path.strip_prefix(&root) {
                                    full_list
                                        .write()
                                        .unwrap()
                                        .remove(path.to_string_lossy().as_ref());
                                }
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::To))
                            if event.paths[0].is_file() =>
                        {
                            println!("rename to {:?}", event.paths);
                            if let Ok(path) = event.paths[0].strip_prefix(&root) {
                                full_list
                                    .write()
                                    .unwrap()
                                    .insert(path.to_string_lossy().to_string());
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                            println!("rename from {:?}", event.paths);
                            if let Ok(path) = event.paths[0].strip_prefix(&root) {
                                full_list
                                    .write()
                                    .unwrap()
                                    .remove(path.to_string_lossy().as_ref());
                            }
                        }
                        EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                            if event.paths[1].is_file() =>
                        {
                            println!("rename both {:?}", event.paths);
                            if let Ok(path) = event.paths[0].strip_prefix(&root) {
                                full_list
                                    .write()
                                    .unwrap()
                                    .remove(path.to_string_lossy().as_ref());
                            }
                            if let Ok(path) = event.paths[1].strip_prefix(&root) {
                                full_list
                                    .write()
                                    .unwrap()
                                    .insert(path.to_string_lossy().to_string());
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
        let mut res = Self::find_by_globs(&files, &globs);
        if let Ok(ref mut v) = res {
            *self.last_search.write().unwrap() = globs.to_string();
            *self.last_search_result.write().unwrap() = v.clone();
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

    fn find_by_globs(paths: &HashSet<String>, globs: &str) -> Result<Vec<String>, crate::Error> {
        let lex = Shlex::new(globs);
        let mut set = GlobSetBuilder::new();
        for mut pat in lex {
            if !pat.contains('*') {
                pat = format!("*{pat}*");
            }
            let glob = GlobBuilder::new(&pat)
                .case_insensitive(true)
                .backslash_escape(false)
                .literal_separator(false)
                .build()?;
            set.add(glob);
        }
        let set = set.build()?;

        let out: Vec<_> = paths
            .par_iter()
            .flat_map(|path| {
                if set.matches(path).len() == set.len() {
                    Some(path.to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(out)
    }
}

pub static SEARCH_STATE: LazyLock<SearchState> = LazyLock::new(Default::default);
pub static SELECTION_STATE: LazyLock<SelectionState> = LazyLock::new(Default::default);

// #[tauri::command]
// pub fn search_files_by_name_sync(globs: &str) -> Result<Vec<String>, crate::Error> {
//     SEARCH_STATE.search_files_by_name(globs)
// }
#[tauri::command]
pub async fn search_files_by_name(globs: &str) -> Result<Vec<String>, crate::Error> {
    spawn_blocking({
        let globs = globs.to_string();
        move || SEARCH_STATE.search_files_by_name(&globs)
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
