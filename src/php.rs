mod language_servers;
mod xdebug;

use std::env;
use std::fs;
use zed::CodeLabel;
use zed_extension_api::{
    self as zed, serde_json, DebugConfig, DebugScenario, LanguageServerId, Result,
    StartDebuggingRequestArgumentsRequest,
};

use crate::{
    language_servers::{PhpTools, Intelephense, Phpactor},
    xdebug::XDebug,
};

struct PhpExtension {
    phptools: Option<PhpTools>,
    intelephense: Option<Intelephense>,
    phpactor: Option<Phpactor>,
    xdebug: XDebug,
}

impl zed::Extension for PhpExtension {
    fn new() -> Self {
        Self {
            phptools: None,
            intelephense: None,
            phpactor: None,
            xdebug: XDebug::new(),
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            PhpTools::LANGUAGE_SERVER_ID => {
                let phptools = self.phptools.get_or_insert_with(PhpTools::new);
                phptools.language_server_command(language_server_id, worktree)
            }
            Intelephense::LANGUAGE_SERVER_ID => {
                let intelephense = self.intelephense.get_or_insert_with(Intelephense::new);
                intelephense.language_server_command(language_server_id, worktree)
            }
            Phpactor::LANGUAGE_SERVER_ID => {
                let phpactor = self.phpactor.get_or_insert_with(Phpactor::new);

                let (platform, _) = zed::current_platform();

                let phparctor_path =
                    phpactor.language_server_binary_path(language_server_id, worktree)?;

                if platform == zed::Os::Windows {
                    // fix：.phar files are not executable https://github.com/zed-extensions/php/issues/23
                    if let Some(path) = worktree.which("php") {
                        // get abs_phparctor_path
                        let abs_phparctor_path = match fs::canonicalize(&phparctor_path) {
                            Ok(path) => path,
                            Err(_) => {
                                // canonicalize 失败，fallback
                                env::current_dir()
                                    .map_err(|e| format!("failed to get current directory: {e}"))?
                                    .join(&phparctor_path)
                            }
                        };

                        return Ok(zed::Command {
                            command: path,
                            args: vec![
                                zed_ext::sanitize_windows_path(abs_phparctor_path)
                                    .to_string_lossy()
                                    .to_string(),
                                "language-server".into(),
                            ],
                            env: Default::default(),
                        });
                    } else {
                        return Err("php not found".into());
                    }
                } else {
                    Ok(zed::Command {
                        command: phparctor_path,
                        args: vec!["language-server".into()],
                        env: Default::default(),
                    })
                }
            }
            language_server_id => Err(format!("unknown language server: {language_server_id}")),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        if language_server_id.as_ref() == PhpTools::LANGUAGE_SERVER_ID {
            if let Some(phptools) = self.phptools.as_mut() {
                return phptools.language_server_workspace_configuration(worktree);
            }
        }
        if language_server_id.as_ref() == Intelephense::LANGUAGE_SERVER_ID {
            if let Some(intelephense) = self.intelephense.as_mut() {
                return intelephense.language_server_workspace_configuration(worktree);
            }
        }

        Ok(None)
    }

    fn label_for_completion(
        &self,
        language_server_id: &zed::LanguageServerId,
        completion: zed::lsp::Completion,
    ) -> Option<CodeLabel> {
        match language_server_id.as_ref() {
            Intelephense::LANGUAGE_SERVER_ID => {
                self.intelephense.as_ref()?.label_for_completion(completion)
            }
            _ => None,
        }
    }
    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        if adapter_name != XDebug::NAME {
            return Err(format!(
                "PHP extension does not support unknown adapter in `dap_request_kind`: {adapter_name} (supported: [{}])",
                XDebug::NAME
            ));
        }
        self.xdebug.dap_request_kind(&config)
    }
    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario, String> {
        if config.adapter != XDebug::NAME {
            return Err(format!(
                "PHP extension does not support unknown adapter in `dap_config_to_scenario`: {} (supported: [{}])",
                config.adapter, XDebug::NAME
            ));
        }
        self.xdebug.dap_config_to_scenario(config)
    }
    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: zed_extension_api::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<zed_extension_api::DebugAdapterBinary, String> {
        if config.adapter != XDebug::NAME {
            return Err(format!(
                "PHP extension does not support unknown adapter in `get_dap_binary`: {} (supported: [{}])",
                adapter_name, XDebug::NAME
            ));
        }
        self.xdebug
            .get_binary(config, user_provided_debug_adapter_path, worktree)
    }
}

zed::register_extension!(PhpExtension);

/// Extensions to the Zed extension API that have not yet stabilized.
mod zed_ext {
    /// Sanitizes the given path to remove the leading `/` on Windows.
    ///
    /// On macOS and Linux this is a no-op.
    ///
    /// This is a workaround for https://github.com/bytecodealliance/wasmtime/issues/10415.
    pub fn sanitize_windows_path(path: std::path::PathBuf) -> std::path::PathBuf {
        use zed_extension_api::{current_platform, Os};

        let (os, _arch) = current_platform();
        match os {
            Os::Mac | Os::Linux => path,
            Os::Windows => path
                .to_string_lossy()
                .to_string()
                .trim_start_matches('/')
                .into(),
        }
    }
}
