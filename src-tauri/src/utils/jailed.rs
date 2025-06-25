use std::io;
use std::path::{Component, Path, PathBuf};

/// A trait to provide chroot-like "jailed" file path operations,
/// ensuring that all resolved paths remain within a designated directory.
pub trait Jailed {
    /// Securely joins a user-provided path to `self`, where `self` is the jail.
    ///
    /// This function implements special handling for absolute paths: it strips
    /// the root component (e.g., `/` or `C:\`) and treats the rest of the
    /// path as relative to the jail.
    ///
    /// # Arguments
    /// * `&self` - The path acting as the jail directory.
    /// * `user_path` - The path to join.
    fn jailed_join(&self, user_path: &Path) -> io::Result<PathBuf>;

    /// Checks if `self` (the jail) securely contains the `other` path.
    ///
    /// Both paths are canonicalized to prevent directory traversal attacks.
    ///
    /// # Arguments
    /// * `&self` - The path acting as the jail directory.
    /// * `other` - The path to check for containment within the jail.
    fn jailed_contains(&self, other: &Path) -> io::Result<bool>;
}

impl Jailed for Path {
    fn jailed_join(&self, user_path: &Path) -> io::Result<PathBuf> {
        let path_to_join: PathBuf = if user_path.has_root() {
            user_path
                .components()
                .skip_while(|&x| x != Component::RootDir)
                .skip(1)
                .collect()
        } else {
            user_path.to_owned()
        };

        let new_path = self.join(path_to_join);

        // We must check for traversal attacks AFTER joining.
        if self.jailed_contains(&new_path)? {
            Ok(new_path)
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Path traversal attempt detected",
            ))
        }
    }

    fn jailed_contains(&self, other: &Path) -> io::Result<bool> {
        let canonical_jail = self.canonicalize()?;
        if !other.exists() {
            let parent = other.parent().unwrap_or(other);
            return self.jailed_contains(parent);
        }

        let canonical_other = other.canonicalize()?;
        Ok(canonical_other.starts_with(canonical_jail))
    }
}
