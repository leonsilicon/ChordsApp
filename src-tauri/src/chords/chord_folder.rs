use crate::chords::AppChordsFile;
use anyhow::Result;
use fast_radix_trie::StringRadixMap;
use include_dir::{include_dir, Dir};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ChordFolder {
    pub root_dir: Option<PathBuf>,

    // Map from file path to chord
    pub chords_files: StringRadixMap<AppChordsFile>,
    pub js_files: StringRadixMap<String>,
}

static BUNDLED_MACOS_CHORDS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../data/chords");

impl ChordFolder {
    pub fn load_bundled() -> Result<Self> {
        let mut chords_files = StringRadixMap::new();
        for file in BUNDLED_MACOS_CHORDS_DIR.find("**/macos.toml")? {
            let path = format!("chords/{}", file.path().to_string_lossy());
            let content = file
                .as_file()
                .and_then(|f| f.contents_utf8())
                .ok_or_else(|| anyhow::anyhow!("Could not read file as utf8: {:?}", file.path()))?;
            let app_chords_file = AppChordsFile::parse(content)?;
            chords_files.insert(path, app_chords_file);
        }

        Ok(Self {
            root_dir: None,
            chords_files,
            js_files: StringRadixMap::new(),
        })
    }

    pub fn load_from_git_repo(repo: &gix::Repository) -> Result<Self> {
        let root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        Self::load_from_local_folder(root)
    }

    pub fn load_from_local_folder(root: &Path) -> Result<Self> {
        let mut js_files = StringRadixMap::new();
        let mut chords_files = StringRadixMap::new();

        if root.exists() {
            for entry in WalkDir::new(root) {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let relative_path = path.strip_prefix(root)?.to_path_buf();

                // ------------------------
                // Handle chords/**/macos.toml
                // ------------------------
                if relative_path.starts_with("chords") {
                    if entry.file_name() == "macos.toml" {
                        let content = std::fs::read_to_string(path)?;

                        match AppChordsFile::parse(&content) {
                            Ok(parsed) => {
                                chords_files
                                    .insert(relative_path.to_string_lossy().to_string(), parsed);
                            }
                            Err(error) => {
                                log::warn!("Skipping invalid {:?}: {}", path, error);
                                continue;
                            }
                        }
                    }
                }

                // ------------------------
                // Handle *.js files
                // ------------------------
                if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    let content = std::fs::read_to_string(path)?;

                    js_files.insert(relative_path.to_string_lossy().to_string(), content);
                }
            }
        } else {
            log::debug!("Root folder does not exist: {:?}", root);
        }

        Ok(Self {
            root_dir: Some(root.to_path_buf()),
            chords_files,
            js_files,
        })
    }

    pub fn merge(&mut self, other: Self) {
        self.chords_files.extend(other.chords_files);
        self.js_files.extend(other.js_files);
    }
}
