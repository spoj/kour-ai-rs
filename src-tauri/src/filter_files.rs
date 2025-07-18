use globset::{GlobBuilder, GlobSetBuilder};
use ignore::WalkBuilder;
use shlex::Shlex;

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

#[tauri::command]
pub fn search_files_by_name(globs: &str) -> Result<Vec<String>, crate::Error> {
    let root = get_root()?;
    let walker = WalkBuilder::new(root.clone()).build();
    let lex = Shlex::new(globs);
    let mut set = GlobSetBuilder::new();
    for mut pat in lex {
        if !pat.contains('*') {
            pat = format!("*{pat}*");
        }
        println!("pat: {pat}");

        let glob = GlobBuilder::new(&pat)
            .case_insensitive(true)
            .backslash_escape(false)
            .literal_separator(false)
            .build()?;
        set.add(glob);
    }
    println!();
    let set = set.build()?;
    Ok(walker
        .flatten()
        .filter(|p| {
            let rel = p.path().strip_prefix(&root);
            match rel {
                Ok(p) => set.matches(p).len() == set.len(),
                Err(_) => false,
            }
        })
        .map(|p| p.path().to_string_lossy().to_string())
        .collect())
}
