mod language_servers;

use std::fs;
use std::path::PathBuf;
use zed::CodeLabel;
use zed_extension_api::{self as zed, serde_json, LanguageServerId, Result};

use crate::language_servers::{Intelephense, Phpactor};

struct PhpExtension {
    intelephense: Option<Intelephense>,
    phpactor: Option<Phpactor>,
}

impl zed::Extension for PhpExtension {
    fn new() -> Self {
        Self {
            intelephense: None,
            phpactor: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            Intelephense::LANGUAGE_SERVER_ID => {
                let intelephense = self.intelephense.get_or_insert_with(Intelephense::new);
                intelephense.language_server_command(language_server_id, worktree)
            }
            Phpactor::LANGUAGE_SERVER_ID => {
                let phpactor = self.phpactor.get_or_insert_with(Phpactor::new);

                let (platform, _) = zed::current_platform();

                let phpactor_path =
                    phpactor.language_server_binary_path(language_server_id, worktree)?;

                if platform == zed::Os::Windows {
                    // fix：.phar files are not executable https://github.com/zed-extensions/php/issues/23
                    let php_path = worktree
                        .which("php")
                        .ok_or("Could not find PHP in path! PHP needs to be installed for running phpactor on Windows")?;
                    // get abs_phparctor_path
                    let abs_phparctor_path = fs::canonicalize(&phpactor_path).or_else(|_| {
                        let path = PathBuf::from(worktree.root_path()).join(&phpactor_path);
                        if fs::exists(&path).is_ok_and(|exists| exists) {
                            Ok(path)
                        } else {
                            Err("Could not resolve phpactor path!")
                        }
                    })?;

                    Ok(zed::Command {
                        command: php_path,
                        args: vec![
                            zed_ext::sanitize_windows_path(abs_phparctor_path)
                                .to_string_lossy()
                                .to_string(),
                            "language-server".into(),
                        ],
                        env: Default::default(),
                    })
                } else {
                    Ok(zed::Command {
                        command: phpactor_path,
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
