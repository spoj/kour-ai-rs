use futures::StreamExt;
use serde::Serialize;
use std::io::Write;
use std::path::Path;
use std::{fs::File, path::PathBuf};
use tauri::{Emitter, Window};
use tokio::task::spawn_blocking;

#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum LibreOfficePayload {
    Downloading,
    Installing,
    Success(String),
    Error(String),
}

pub struct Libreoffice {
    pub local_dir: PathBuf,
}

impl Libreoffice {
    fn locate(&self) -> Option<PathBuf> {
        println!("start locate");
        match glob::glob(
            self.local_dir
                .join("**/soffice.com")
                .to_string_lossy()
                .as_ref(),
        )
        .ok()?
        .next()
        {
            Some(Ok(p)) => {
                println!("found {p:?}");
                Some(p)
            }
            _ => None,
        }
    }

    pub async fn ensure(&self, window: &Window) -> Result<(), crate::Error> {
        println!("start ensure");
        if let Some(p) = self.locate() {
            window.emit(
                "libreoffice_update",
                LibreOfficePayload::Success(p.to_string_lossy().to_string()),
            )?;
            Ok(())
        } else if let Ok(p) = self.download(window).await
            && let Ok(_) = self.extract(p, window).await
            && let Some(p) = self.locate()
        {
            window.emit(
                "libreoffice_update",
                LibreOfficePayload::Success(p.to_string_lossy().to_string()),
            )?;
            Ok(())
        } else {
            let err_msg = "cannot download libreoffice".to_string();
            window.emit("libreoffice_update", LibreOfficePayload::Error(err_msg))?;
            Err(crate::Error::Io(std::io::Error::other(
                "cannot download libreoffice",
            )))
        }
    }
}

#[cfg(not(any(target_os = "windows")))]
impl Libreoffice {
    async fn download(&self, _window: &Window) -> Result<PathBuf, crate::Error> {
        use anyhow::anyhow;
        Err(crate::Error::Anyhow(anyhow!("platform not supported")))
    }
    async fn extract<P: AsRef<Path>>(&self, path: P, _window: &Window) -> Result<(), crate::Error> {
        use anyhow::anyhow;
        Err(crate::Error::Anyhow(anyhow!("platform not supported")))
    }
}

#[cfg(target_os = "windows")]
impl Libreoffice {
    async fn download(&self, window: &Window) -> Result<PathBuf, crate::Error> {
        let url = "https://mirror-hk.koddos.net/tdf/libreoffice/stable/25.2.4/win/x86_64/LibreOffice_25.2.4_Win_x86-64.msi";
        window.emit("libreoffice_update", LibreOfficePayload::Downloading)?;
        use std::fs::create_dir_all;
        let local_path = self.local_dir.join("libreoffice.msi");
        println!("start download to {local_path:?}");
        create_dir_all(&self.local_dir)?;
        let mut local_file = File::create(&local_path)?;
        println!("file created {local_path:?}");
        let resp = reqwest::get(url).await?;
        println!("requesting");
        let mut stream = resp.bytes_stream();
        while let Some(b) = stream.next().await {
            local_file.write_all(&b?)?;
        }
        Ok(local_path)
    }

    async fn extract<P: AsRef<Path>>(&self, path: P, window: &Window) -> Result<(), crate::Error> {
        window.emit("libreoffice_update", LibreOfficePayload::Installing)?;
        println!("start extract");
        let local_dir = self.local_dir.clone();
        let path = path.as_ref().to_owned();
        spawn_blocking(move || {
            if let Ok(mut package) = msi_extract::MsiExtractor::from_path(path)
                .map_err(|_| crate::Error::Io(std::io::Error::other("extraction error")))
            {
                package.to(local_dir);
            }
        })
        .await?;

        Ok(())
    }
}
