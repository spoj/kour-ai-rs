use std::sync::{Mutex, RwLock};

use camino::Utf8PathBuf;
use globset::{GlobBuilder, GlobSetBuilder};
use ignore::Walk;
use rayon::prelude::*;
use shlex::Shlex;
use std::sync::LazyLock;

use crate::settings::get_root;

#[derive(Default)]
pub struct SearchState {
    root: Mutex<Option<Utf8PathBuf>>,
    full_list: RwLock<Vec<String>>,
    pub last_search_result: RwLock<Vec<String>>,
    pub last_search: RwLock<String>,
}

pub static SEARCH_STATE: LazyLock<SearchState> = LazyLock::new(SearchState::default);

#[tauri::command]
pub fn search_files_by_name(globs: &str) -> Result<Vec<String>, crate::Error> {
    let root = get_root()?;
    if Some(&root) != (*SEARCH_STATE.root.lock().unwrap()).as_ref() {
        let files: Vec<_> = Walk::new(&root)
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
        *SEARCH_STATE.full_list.write().unwrap() = files;
        *SEARCH_STATE.root.lock().unwrap() = Some(root);
    }

    let globs = globs.to_string();
    let files = SEARCH_STATE.full_list.read().unwrap();
    let mut res = search_files_by_name_internal(&files, &globs);
    if let Ok(ref mut v) = res {
        *SEARCH_STATE.last_search.write().unwrap() = globs.to_string();
        *SEARCH_STATE.last_search_result.write().unwrap() = v.clone();
        v.truncate(500);
    }
    res
}

fn search_files_by_name_internal(
    paths: &[String],
    globs: &str,
) -> Result<Vec<String>, crate::Error> {
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
