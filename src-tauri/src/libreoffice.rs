use std::io::Write;
use std::path::Path;
use std::{fs::File, path::PathBuf};

use futures::StreamExt;
use tokio::task::spawn_blocking;

pub struct Libreoffice {
    pub local_dir: PathBuf,
    pub url: String,
}

#[cfg(not(any(target_os = "windows")))]
impl Libreoffice {
    pub async fn ensure(&self) -> Result<PathBuf, crate::Error> {
        Err(crate::Error::Io(std::io::Error::other(
            "cannot download libreoffice",
        )))
    }
}
#[cfg(target_os = "windows")]
impl Libreoffice {
    async fn download(&self) -> Result<PathBuf, crate::Error> {
        use std::fs::create_dir_all;

        let local_path = self.local_dir.join("libreoffice.msi");
        println!("start download to {local_path:?}");
        create_dir_all(&self.local_dir)?;
        let mut local_file = File::create(&local_path)?;
        println!("file created {local_path:?}");
        let resp = reqwest::get(&self.url).await?;
        println!("requesting");
        let mut stream = resp.bytes_stream();
        while let Some(b) = stream.next().await {
            local_file.write_all(&b?)?;
        }
        Ok(local_path)
    }

    async fn extract<P: AsRef<Path>>(&self, path: P) -> Result<(), crate::Error> {
        println!("start extract");
        let local_dir = self.local_dir.clone();
        let path = path.as_ref().to_owned();
        spawn_blocking(|| {
            if let Ok(mut package) = msi_extract::MsiExtractor::from_path(path)
                .map_err(|_| crate::Error::Io(std::io::Error::other("extraction error")))
            {
                package.to(local_dir);
            }
        })
        .await?;

        Ok(())
    }

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

    pub async fn ensure(&self) -> Result<PathBuf, crate::Error> {
        println!("start ensure");
        if let Some(p) = self.locate() {
            Ok(p)
        } else if let Ok(p) = self.download().await
            && let Ok(_) = self.extract(p).await
            && let Some(p) = self.locate()
        {
            Ok(p)
        } else {
            Err(crate::Error::Io(std::io::Error::other(
                "cannot download libreoffice",
            )))
        }
    }
}
