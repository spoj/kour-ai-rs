use std::sync::Mutex;

use globset::{GlobBuilder, GlobSetBuilder};
use ignore::WalkBuilder;
use shlex::Shlex;
use tauri::State;
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;

use crate::settings::get_root;

#[tauri::command]
pub fn list_files() -> Result<Vec<String>, crate::Error> {
    let root = get_root()?;
    Ok(WalkBuilder::new(root)
        .build()
        .flatten()
        .map(|p| p.path().to_string_lossy().to_string())
        .collect())
}

#[derive(Default)]
pub struct Searching {
    cancel: Mutex<Option<CancellationToken>>,
}
#[tauri::command]
pub async fn search_files_by_name(
    globs: &str,
    state: State<'_, Searching>,
) -> Result<Vec<String>, crate::Error> {
    let token = CancellationToken::new();
    if let Some(old) = state.cancel.lock().unwrap().replace(token.clone()) {
        old.cancel();
    }
    token
        .run_until_cancelled(search_files_by_name_internal(globs))
        .await
        .ok_or(crate::error::Error::Other)?
}

async fn search_files_by_name_internal(globs: &str) -> Result<Vec<String>, crate::Error> {
    let root = get_root()?;
    let walker = WalkBuilder::new(root.clone()).build();
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
    let mut out = vec![];

    for entry in walker.flatten() {
        let path = entry
            .path()
            .strip_prefix(&root)
            .map_err(|_| crate::error::Error::Other)?;
        if set.matches(path).len() == set.len() {
            out.push(path.to_string_lossy().to_string());
        }
        yield_now().await;
    }
    Ok(out)
}
