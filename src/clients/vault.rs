use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct VaultClient {
    base_path: PathBuf,
}

fn expand_vault_path(vault_path: &str, home: Option<PathBuf>) -> Result<PathBuf> {
    // Handle ~/ expansion
    if let Some(stripped) = vault_path.strip_prefix("~/") {
        let home = home.context("Could not find home dir")?;
        Ok(home.join(stripped))
    } else {
        Ok(PathBuf::from(vault_path))
    }
}

impl VaultClient {
    pub fn new(vault_path: &str) -> Result<Self> {
        Ok(Self {
            base_path: expand_vault_path(vault_path, dirs::home_dir())?,
        })
    }

    pub fn write_overnight_digest(
        &self,
        dir_name: &str,
        date_str: &str,
        content: &str,
    ) -> Result<PathBuf> {
        let target_dir = self.base_path.join(dir_name);
        fs::create_dir_all(&target_dir)
            .with_context(|| format!("Failed to create dir: {:?}", target_dir))?;

        let file_name = format!("{}-digest.md", date_str);
        let file_path = target_dir.join(&file_name);

        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write vault file: {:?}", file_path))?;

        Ok(file_path)
    }

    pub fn read_digest(&self, dir_name: &str, date_str: &str) -> Result<Option<String>> {
        let file_path = self
            .base_path
            .join(dir_name)
            .join(format!("{}-digest.md", date_str));
        if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn expands_tilde_slash_against_injected_home() {
        let home = PathBuf::from("/tmp/fake-home");
        let path = expand_vault_path("~/notes", Some(home)).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/fake-home/notes"));
    }

    #[test]
    fn expands_nested_tilde_path() {
        let home = PathBuf::from("/Users/tester");
        let path = expand_vault_path("~/vault/sub", Some(home)).unwrap();
        assert_eq!(path, PathBuf::from("/Users/tester/vault/sub"));
    }

    #[test]
    fn tilde_without_slash_is_not_expanded() {
        // Current behavior: only the "~/" prefix expands. Bare "~" stays literal.
        let path = expand_vault_path("~", Some(PathBuf::from("/tmp/home"))).unwrap();
        assert_eq!(path, PathBuf::from("~"));
    }

    #[test]
    fn tilde_user_syntax_is_not_expanded() {
        let path = expand_vault_path("~other/notes", Some(PathBuf::from("/tmp/home"))).unwrap();
        assert_eq!(path, PathBuf::from("~other/notes"));
    }

    #[test]
    fn tilde_expansion_errors_without_home() {
        let err = expand_vault_path("~/notes", None).unwrap_err();
        assert!(err.to_string().contains("Could not find home dir"));
    }

    #[test]
    fn absolute_path_ignores_home() {
        let path = expand_vault_path("/abs/vault", Some(PathBuf::from("/tmp/home"))).unwrap();
        assert_eq!(path, PathBuf::from("/abs/vault"));
    }

    #[test]
    fn relative_path_is_unchanged() {
        let path = expand_vault_path("notes", None).unwrap();
        assert_eq!(path, PathBuf::from("notes"));
    }

    #[test]
    fn write_and_read_digest_roundtrip() {
        let dir = crate::test_support::temp_dir();
        let client = VaultClient::new(dir.to_str().unwrap()).unwrap();
        let path = client
            .write_overnight_digest("Daily Intelligence", "2026-08-21", "hello digest")
            .unwrap();
        assert_eq!(path.file_name().unwrap(), "2026-08-21-digest.md");
        let read = client
            .read_digest("Daily Intelligence", "2026-08-21")
            .unwrap();
        assert_eq!(read.as_deref(), Some("hello digest"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn read_missing_digest_returns_none() {
        let dir = crate::test_support::temp_dir();
        let client = VaultClient::new(dir.to_str().unwrap()).unwrap();
        let read = client
            .read_digest("Daily Intelligence", "1999-01-01")
            .unwrap();
        assert_eq!(read, None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn write_empty_digest_then_read() {
        let dir = crate::test_support::temp_dir();
        let client = VaultClient::new(dir.to_str().unwrap()).unwrap();
        client
            .write_overnight_digest("out", "2026-01-01", "")
            .unwrap();
        let read = client.read_digest("out", "2026-01-01").unwrap();
        assert_eq!(read.as_deref(), Some(""));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn write_overwrites_existing_digest() {
        let dir = crate::test_support::temp_dir();
        let client = VaultClient::new(dir.to_str().unwrap()).unwrap();
        client
            .write_overnight_digest("out", "2026-01-01", "v1")
            .unwrap();
        client
            .write_overnight_digest("out", "2026-01-01", "v2")
            .unwrap();
        let read = client.read_digest("out", "2026-01-01").unwrap();
        assert_eq!(read.as_deref(), Some("v2"));
        let _ = fs::remove_dir_all(dir);
    }
}
