use crate::chords::{ChordFolder, LoadedAppChords};
use crate::feature::Chorder;
use crate::{
    feature::ChorderIndicatorPanel,
    input::KeyEventState,
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use device_query::DeviceState;
use objc2_app_kit::NSWorkspace;
use parking_lot::RwLock;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepoInfo {
    pub owner: String,
    pub name: String,
    pub slug: String,
    pub url: String,
    pub local_path: String,
}

#[derive(Debug, Clone)]
pub struct GitHubRepoRef {
    pub owner: String,
    pub name: String,
}

impl GitHubRepoRef {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Repository cannot be empty");
        }

        let slug = trimmed
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .strip_prefix("https://github.com/")
            .or_else(|| trimmed.strip_prefix("http://github.com/"))
            .or_else(|| trimmed.strip_prefix("git@github.com:"))
            .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
            .unwrap_or(trimmed)
            .trim_matches('/');

        let mut parts = slug.split('/');
        let owner = parts
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Repository must be in the form owner/name"))?;
        let name = parts
            .next()
            .filter(|segment| !segment.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Repository must be in the form owner/name"))?;

        if parts.next().is_some() {
            anyhow::bail!("Repository must be in the form owner/name");
        }

        if owner.contains(char::is_whitespace) || name.contains(char::is_whitespace) {
            anyhow::bail!("Repository owner and name cannot contain spaces");
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    pub fn url(&self) -> String {
        format!("https://github.com/{}", self.slug())
    }

    pub fn local_path(&self, repos_root: &Path) -> PathBuf {
        repos_root.join(&self.owner).join(&self.name)
    }

    pub fn into_info(self, repos_root: &Path) -> GitRepoInfo {
        let slug = self.slug();
        let url = self.url();
        let local_path = self.local_path(repos_root);
        GitRepoInfo {
            owner: self.owner,
            name: self.name,
            slug,
            url,
            local_path: local_path.display().to_string(),
        }
    }
}

pub struct AppContext {
    pub chorder: Chorder,

    pub device_state: Option<DeviceState>,
    pub loaded_app_chords: RwLock<LoadedAppChords>,
    pub frontmost_application_id: ArcSwap<Option<String>>,
    pub key_event_state: KeyEventState,

    // Not a mutex since it uses Atomics
    app_mode_state_machine: Arc<AppModeStateMachine>,
}

impl AppContext {
    pub fn new(chorder: Chorder) -> Result<Self> {
        let device_state = if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        };

        let app_mode_state_machine = Arc::new(AppModeStateMachine::new(device_state.clone()));

        Ok(Self {
            device_state,
            frontmost_application_id: ArcSwap::new(Arc::new(None)),
            key_event_state: KeyEventState::new(app_mode_state_machine.clone()),
            loaded_app_chords: RwLock::new(LoadedAppChords::from_folder(
                ChordFolder::load_bundled()?,
            )?),
            app_mode_state_machine,
            chorder,
        })
    }

    pub fn get_app_mode(&self) -> AppMode {
        self.app_mode_state_machine.get_app_mode()
    }
}

pub fn initialize_app_context(app: AppHandle) -> Result<()> {
    let chorder = {
        let window = app
            .get_webview_window(crate::constants::INDICATOR_WINDOW_LABEL)
            .ok_or(anyhow::anyhow!("chord indicator window not found"))?;
        Chorder::new(ChorderIndicatorPanel::from_window(window)?)
    };

    let context = AppContext::new(chorder)?;

    // Setting the frontmost application immediately (the frontmost crate only detects changes)
    let workspace = NSWorkspace::sharedWorkspace();
    if let Some(application) = workspace.frontmostApplication() {
        if let Some(bundle_id) = application.bundleIdentifier() {
            context
                .frontmost_application_id
                .store(Arc::new(Some(bundle_id.to_string())));
        }
    }

    app.manage(context);
    reload_loaded_app_chords(&app)?;

    Ok(())
}

pub fn github_repos_root(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().app_cache_dir()?.join("repos/github.com"))
}

pub fn discover_git_repos(app: &AppHandle) -> Result<Vec<GitRepoInfo>> {
    let repos_root = github_repos_root(app)?;
    if !repos_root.exists() {
        return Ok(Vec::new());
    }

    let mut repos = Vec::new();
    for owner_entry in fs::read_dir(&repos_root)? {
        let owner_entry = owner_entry?;
        let owner_path = owner_entry.path();
        if !owner_path.is_dir() {
            continue;
        }

        for repo_entry in fs::read_dir(&owner_path)? {
            let repo_entry = repo_entry?;
            let repo_path = repo_entry.path();
            if !repo_path.is_dir() || !repo_path.join(".git").exists() {
                continue;
            }

            let Some(owner) = owner_path.file_name().and_then(|segment| segment.to_str()) else {
                continue;
            };
            let Some(name) = repo_path.file_name().and_then(|segment| segment.to_str()) else {
                continue;
            };

            repos.push(
                GitHubRepoRef {
                    owner: owner.to_string(),
                    name: name.to_string(),
                }
                .into_info(&repos_root),
            );
        }
    }

    repos.sort_by(|left, right| left.slug.cmp(&right.slug));
    Ok(repos)
}

pub fn add_git_repo(app: &AppHandle, repo_input: &str) -> Result<GitRepoInfo> {
    let repo_ref = GitHubRepoRef::parse(repo_input)?;
    let repos_root = github_repos_root(app)?;
    let repo_path = repo_ref.local_path(&repos_root);

    if repo_path.join(".git").exists() {
        reload_loaded_app_chords(app)?;
        return Ok(repo_ref.into_info(&repos_root));
    }

    clone_repo(&repo_ref, &repo_path)?;
    reload_loaded_app_chords(app)?;

    Ok(repo_ref.into_info(&repos_root))
}

pub fn sync_git_repo(app: &AppHandle, repo_input: &str) -> Result<GitRepoInfo> {
    let repo_ref = GitHubRepoRef::parse(repo_input)?;
    let repos_root = github_repos_root(app)?;
    let repo_path = repo_ref.local_path(&repos_root);

    if !repo_path.join(".git").exists() {
        anyhow::bail!("Repository {} has not been added yet", repo_ref.slug());
    }

    refresh_repo(&repo_ref, &repo_path)?;
    reload_loaded_app_chords(app)?;

    Ok(repo_ref.into_info(&repos_root))
}

pub fn reload_loaded_app_chords(app: &AppHandle) -> Result<()> {
    let loaded_chords = load_all_app_chords(app)?;
    let context = app.state::<AppContext>();
    *context.loaded_app_chords.write() = loaded_chords;
    Ok(())
}

fn load_all_app_chords(app: &AppHandle) -> Result<LoadedAppChords> {
    let mut chord_folder = ChordFolder::load_bundled()?;
    for repo in discover_git_repos(app)? {
        match gix::open(&repo.local_path)
            .context(format!("failed to open repo {}", repo.slug))
            .and_then(|repo_handle| ChordFolder::load_from_git_repo(&repo_handle))
        {
            Ok(repo_folder) => chord_folder.merge(repo_folder),
            Err(error) => log::warn!("Skipping repo {}: {error}", repo.slug),
        }
    }

    LoadedAppChords::from_folder(chord_folder)
}

fn clone_repo(repo_ref: &GitHubRepoRef, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }

    let mut clone = gix::prepare_clone(repo_ref.url(), destination)?;
    let (mut checkout, checkout_outcome) =
        clone.fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    log::debug!(
        "Checkout outcome for {}: {:?}",
        repo_ref.slug(),
        checkout_outcome
    );
    let (_repo, worktree_outcome) =
        checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    log::debug!(
        "Worktree outcome for {}: {:?}",
        repo_ref.slug(),
        worktree_outcome
    );

    Ok(())
}

fn refresh_repo(repo_ref: &GitHubRepoRef, destination: &Path) -> Result<()> {
    let temp_destination = destination.with_extension("syncing");
    if temp_destination.exists() {
        fs::remove_dir_all(&temp_destination)?;
    }

    clone_repo(repo_ref, &temp_destination)?;
    fs::remove_dir_all(destination)?;
    fs::rename(temp_destination, destination)?;

    Ok(())
}
