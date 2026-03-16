use crate::chords::AppChordsFile;
use anyhow::Result;
use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ChordFolder {
    // Map from file path to chord
    pub files_map: HashMap<String, AppChordsFile>,
}

static BUNDLED_CHORDS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../data/chords");

impl ChordFolder {
    pub fn load_bundled() -> Result<Self> {
        let mut files_map = HashMap::new();
        for file in BUNDLED_CHORDS_DIR.find("**/chords.toml")? {
            let path = file.path().to_string_lossy().to_string();
            let content = file
                .as_file()
                .and_then(|f| f.contents_utf8())
                .ok_or_else(|| anyhow::anyhow!("Could not read file as utf8: {:?}", file.path()))?;
            let app_chords_file = AppChordsFile::parse(content)?;
            files_map.insert(path, app_chords_file);
        }

        Ok(Self { files_map })
    }

    pub fn load_from_git_repo(repo: &gix::Repository) -> Result<Self> {
        let mut files_map = HashMap::new();

        let root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        let chords_dir = root.join("chords");
        if !chords_dir.exists() {
            return Ok(Self { files_map });
        }

        for entry in WalkDir::new(&chords_dir) {
            let entry = entry?;
            if entry.file_name() == "chords.toml" {
                let content = std::fs::read_to_string(entry.path())?;
                let parsed = AppChordsFile::parse(&content)?;

                let relative_path = entry
                    .path()
                    .strip_prefix(&chords_dir)?
                    .to_string_lossy()
                    .to_string();

                files_map.insert(relative_path, parsed);
            }
        }

        Ok(Self { files_map })
    }

    pub fn merge(&mut self, other: Self) {
        self.files_map.extend(other.files_map);
    }
}
