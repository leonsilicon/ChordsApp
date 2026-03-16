use crate::chords::{ChordFolder, LoadedAppChords};
use crate::feature::Chorder;
use crate::{
    feature::ChorderIndicatorPanel,
    input::KeyEventState,
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use arc_swap::ArcSwap;
use device_query::DeviceState;
use objc2_app_kit::NSWorkspace;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

pub struct AppContext {
    pub chorder: Chorder,

    pub device_state: Option<DeviceState>,
    pub loaded_app_chords: LoadedAppChords,
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
            loaded_app_chords: LoadedAppChords::from_folder(ChordFolder::load_bundled()?)?,
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

    let mut context = AppContext::new(chorder)?;

    // Setting the frontmost application immediately (the frontmost crate only detects changes)
    let workspace = NSWorkspace::sharedWorkspace();
    if let Some(application) = workspace.frontmostApplication() {
        if let Some(bundle_id) = application.bundleIdentifier() {
            context
                .frontmost_application_id
                .store(Arc::new(Some(bundle_id.to_string())));
        }
    }

    let cache_dir = app.path().app_cache_dir()?;
    let local_repos_path = cache_dir.join(Path::new("repos/github.com/leonsilicon"));
    fs::create_dir_all(&local_repos_path)?;
    let local_chords_repo_path = local_repos_path.join("chords");
    let repo = if fs::exists(&local_chords_repo_path.join(".git"))? {
        gix::open(local_chords_repo_path)?
    } else {
        let mut clone = gix::prepare_clone(
            "https://github.com/leonsilicon/chords",
            local_chords_repo_path,
        )?;
        let (mut checkout, checkout_outcome) =
            clone.fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
        log::debug!("Checkout outcome: {:?}", checkout_outcome);
        let (repo, worktree_outcome) =
            checkout.main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
        log::debug!("Worktree outcome: {:?}", worktree_outcome);
        repo
    };
    context.loaded_app_chords =
        LoadedAppChords::from_folder(ChordFolder::load_from_git_repo(&repo)?)?;

    app.manage(context);

    Ok(())
}
