use anyhow::anyhow;

use crate::settings::get_root;

pub fn list_files() -> Result<Vec<String>, crate::Error> {
    let root = get_root()?;
    let res = glob::glob(&format!("{root}/**/*")).map_err(|_| anyhow!("glob error"))?;
    Ok(res
        .flat_map(|p| match p {
            Ok(p) => Some(p.to_string_lossy().into_owned()),
            Err(_) => None,
        })
        .collect())
}
