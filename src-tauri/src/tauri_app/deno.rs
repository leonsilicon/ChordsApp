use std::{rc::Rc, sync::Arc};

use deno_resolver::npm::{DenoInNpmPackageChecker, NpmResolver};
use deno_runtime::{
    deno_core::{ModuleSpecifier, error::AnyError},
    deno_fs::RealFs,
    deno_permissions::PermissionsContainer,
    ops::bootstrap::SnapshotOptions,
    permissions::RuntimePermissionDescriptorParser,
    worker::{MainWorker, WorkerOptions, WorkerServiceOptions},
};
use deno_runtime::deno_core::{FsModuleLoader};
use sys_traits::impls::RealSys;
use crate::tauri_app::deno_resolver::TypescriptModuleLoader;

// Extension to provide SnapshotOptions
deno_runtime::deno_core::extension!(
    snapshot_options_extension,
    options = {
        snapshot_options: SnapshotOptions,
    },
    state = |state, options| {
        state.put::<SnapshotOptions>(options.snapshot_options);
    },
);

pub async fn create_main_worker() -> Result<MainWorker, AnyError> {
    let main_module = ModuleSpecifier::parse("data:application/javascript,")?;
    let fs = Arc::new(RealFs);
    let permission_desc_parser = Arc::new(RuntimePermissionDescriptorParser::new(RealSys));
    let permissions = PermissionsContainer::allow_all(permission_desc_parser);

    let module_loader = Rc::new(TypescriptModuleLoader::new());
    // Set up worker service options with our npm-capable module loader
    let services = WorkerServiceOptions::<DenoInNpmPackageChecker, NpmResolver<RealSys>, RealSys> {
        module_loader,
        permissions,
        blob_store: Default::default(),
        broadcast_channel: Default::default(),
        feature_checker: Default::default(),
        fs: fs.clone(),
        node_services: Default::default(),
        npm_process_state_provider: Default::default(),
        root_cert_store_provider: Default::default(),
        fetch_dns_resolver: Default::default(),
        shared_array_buffer_store: Default::default(),
        compiled_wasm_module_store: Default::default(),
        v8_code_cache: Default::default(),
        deno_rt_native_addon_loader: Default::default(),
        bundle_provider: None,
    };

    // Set up worker options with our extension
    let snapshot_options = SnapshotOptions::default();
    let options = WorkerOptions {
        extensions: vec![snapshot_options_extension::init(snapshot_options)],
        ..Default::default()
    };

    let options = WorkerOptions {
        extensions: vec![],
        ..Default::default()
    };

    // Create the MainWorker
    Ok(MainWorker::bootstrap_from_options(
        &main_module,
        services,
        options,
    ))
}
