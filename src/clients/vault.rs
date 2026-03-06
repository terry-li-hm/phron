use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct VaultClient {
    base_path: PathBuf,
}

impl VaultClient {
    pub fn new(vault_path: &str) -> Result<Self> {
        // Handle ~/ expansion
        let expanded = if let Some(stripped) = vault_path.strip_prefix("~/") {
            let home = dirs::home_dir().context("Could not find home dir")?;
            home.join(stripped)
        } else {
            PathBuf::from(vault_path)
        };

        Ok(Self {
            base_path: expanded,
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
