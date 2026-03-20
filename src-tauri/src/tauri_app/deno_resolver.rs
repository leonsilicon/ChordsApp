use std::fs;
use std::sync::Arc;
use deno_core::futures::FutureExt;
use deno_core::{resolve_import, ModuleLoader};
use deno_ast::{EmitOptions, MediaType, TranspileModuleOptions, TranspileOptions};
use deno_ast::ParseParams;
use deno_core::ModuleSource;
use deno_core::ModuleType;
use anyhow::bail;
use deno_runtime::deno_core;
use deno_runtime::deno_core::error::{JsError, ModuleLoaderError};
use deno_runtime::deno_core::{FastString, ModuleLoadOptions, ModuleLoadResponse, ModuleResolutionError, ModuleSourceCode};

pub struct TypescriptModuleLoader;

impl Default for TypescriptModuleLoader {
    fn default() -> Self {
        Self {}
    }
}

impl TypescriptModuleLoader {
    pub fn new() -> Self {
        Self { }
    }
}

impl ModuleLoader for TypescriptModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<deno_core::ModuleSpecifier, ModuleLoaderError> {
        Ok(resolve_import(specifier, referrer).map_err(|e| ModuleLoaderError::from_err(e))?)
    }

    fn load(
        &self,
        module_specifier: &deno_core::ModuleSpecifier,
        _maybe_referrer: Option<&deno_core::ModuleLoadReferrer>,
        _options: ModuleLoadOptions
    ) -> ModuleLoadResponse {
        let module_specifier = module_specifier.clone();
        ModuleLoadResponse::Async(
            async move {
                let result: anyhow::Result<_> = async {
                    let (code, module_type, media_type, should_transpile) =
                        match module_specifier.to_file_path() {
                            Ok(path) => {
                                let media_type = MediaType::from_path(&path);
                                let (module_type, should_transpile) = match media_type {
                                    MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                                        (ModuleType::JavaScript, false)
                                    }
                                    MediaType::Jsx => (ModuleType::JavaScript, true),
                                    MediaType::TypeScript
                                    | MediaType::Mts
                                    | MediaType::Cts
                                    | MediaType::Dts
                                    | MediaType::Dmts
                                    | MediaType::Dcts
                                    | MediaType::Tsx => (ModuleType::JavaScript, true),
                                    MediaType::Json => (ModuleType::Json, false),
                                    _ => bail!("Unknown extension {:?}", path.extension()),
                                };

                                (
                                    fs::read_to_string(&path)?,
                                    module_type,
                                    media_type,
                                    should_transpile,
                                )
                            }
                            Err(_) => {
                                bail!("Unsupported module specifier: {}", module_specifier);
                            }
                        };

                    let code = if should_transpile {
                        let parsed = deno_ast::parse_module(ParseParams {
                            specifier: module_specifier.clone(),
                            text: Arc::from(code.as_str()),
                            media_type,
                            capture_tokens: false,
                            scope_analysis: false,
                            maybe_syntax: None,
                        })?;

                        parsed
                            .transpile(
                                &TranspileOptions::default(),
                                &TranspileModuleOptions::default(),
                                &EmitOptions::default(),
                            )?
                            .into_source()
                            .text
                    } else {
                        code
                    };

                    let module = ModuleSource::new(
                        module_type,
                        ModuleSourceCode::String(FastString::from(code)),
                        &module_specifier,
                        None,
                    );

                    Ok(module)
                }
                    .await;

                result.map_err(|err| ModuleLoaderError::generic(err.to_string()))
            }
                .boxed_local(),
        )
    }
}
