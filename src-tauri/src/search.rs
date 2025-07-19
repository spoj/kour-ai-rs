use std::{
    path::PathBuf,
    sync::{Mutex, RwLock},
};

use globset::{GlobBuilder, GlobSetBuilder};
use ignore::Walk;
use rayon::prelude::*;
use shlex::Shlex;
use tauri::State;

use crate::settings::get_root;

#[derive(Default)]
pub struct Searching {
    root: Mutex<Option<PathBuf>>,
    files: RwLock<Vec<String>>,
}
#[tauri::command]
pub fn search_files_by_name(
    globs: &str,
    state: State<'_, Searching>,
) -> Result<Vec<String>, crate::Error> {
    let root = get_root()?;
    if Some(&root) != (*state.root.lock().unwrap()).as_ref() {
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
        *state.files.write().unwrap() = files;
        *state.root.lock().unwrap() = Some(root);
    }

    let globs = globs.to_string();
    let files = state.files.read().unwrap();
    search_files_by_name_internal(&files, &globs)
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

    let mut out: Vec<_> = paths
        .par_iter()
        .flat_map(|path| {
            if set.matches(path).len() == set.len() {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect();
    out.truncate(500);
    Ok(out)
}
