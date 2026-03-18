use crate::chords::AppChordsFile;
use anyhow::Result;
use fast_radix_trie::StringRadixMap;
use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ChordFolder {
    pub root_dir: Option<PathBuf>,

    // Map from file path to chord
    pub chords_files: StringRadixMap<AppChordsFile>,
    // Can contain multiple when merged
    pub lua_dirs: Vec<PathBuf>,
}

static BUNDLED_MACOS_CHORDS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../data/chords/macos");

impl ChordFolder {
    pub fn load_bundled() -> Result<Self> {
        let mut chords_files = StringRadixMap::new();
        for file in BUNDLED_MACOS_CHORDS_DIR.find("**/chords.toml")? {
            let path = file.path().to_string_lossy().to_string();
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
            lua_dirs: vec![],
        })
    }

    pub fn load_from_git_repo(repo: &gix::Repository) -> Result<Self> {
        let mut chords_files = StringRadixMap::new();

        let root = repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

        // ------------------------
        // Load chords/macos
        // ------------------------
        let chords_dir = root.join("chords").join("macos");
        if chords_dir.exists() {
            for entry in WalkDir::new(&chords_dir) {
                let entry = entry?;
                if entry.file_name() == "chords.toml" {
                    let content = std::fs::read_to_string(entry.path())?;
                    match AppChordsFile::parse(&content) {
                        Ok(parsed) => {
                            let relative_path = entry
                                .path()
                                .strip_prefix(&chords_dir)?
                                .to_string_lossy()
                                .to_string();

                            chords_files.insert(relative_path, parsed);
                        }
                        Err(error) => {
                            log::warn!("Skipping invalid {:?}: {}", entry.path(), error);
                            continue;
                        }
                    };
                }
            }
        } else {
            log::debug!("No chords/macos folder found in {:?}", root);
        }

        let mut lua_dirs = Vec::new();
        let lua_dir = root.join("lua");
        if lua_dir.exists() {
            lua_dirs.push(lua_dir.clone());
        }

        let lua_src_dir = lua_dir.join("src");
        if lua_src_dir.exists() {
            lua_dirs.push(lua_src_dir);
        }

        Ok(Self {
            root_dir: Some(root.to_path_buf()),
            chords_files,
            lua_dirs,
        })
    }

    pub fn merge(&mut self, other: Self) {
        self.chords_files.extend(other.chords_files);
        self.lua_dirs.extend(other.lua_dirs);
    }
}
