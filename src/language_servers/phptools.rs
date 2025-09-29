use std::fs;
use zed::{Architecture, Os};
use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, serde_json, LanguageServerId, Result};

const PACKAGE_NAME: &str = "devsense-php-ls";

pub struct PhpTools {
    did_find_server: bool,
}

impl PhpTools {
    pub const LANGUAGE_SERVER_ID: &'static str = "phptools";

    pub fn new() -> Self {
        Self {
            did_find_server: false,
        }
    }

    pub fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if let Some(path) = worktree.which("phptools") {
            return Ok(zed::Command {
                command: path,
                args: vec!["--stdio".to_string()],
                env: Default::default(),
            });
        }

        let server_path = self.server_script_path(language_server_id)?;
        Ok(zed::Command {
            command: server_path,
            args: vec![
                "--composerNodes".into(),
                "false".into(), // disable /vendor/ caching
            ],
            env: Default::default(),
        })
    }

    fn server_file_path(&self) -> std::string::String {
        let (os, arch) = zed::current_platform();

        let os_str = match os {
            Os::Mac => "darwin",
            Os::Linux => "linux",
            Os::Windows => "win32",
        };

        let arch_str = match arch {
            Architecture::Aarch64 => "arm64",
            Architecture::X86 | Architecture::X8664 => "x64",
        };

        let ext_str = match os {
            Os::Windows => ".exe",
            _ => "",
        };
        //
        format!(
            "node_modules/devsense-php-ls-{0}-{1}/dist/devsense.php.ls{2}",
            os_str, arch_str, ext_str
        )
    }

    fn server_exists(&self) -> bool {
        fs::metadata(self.server_file_path()).map_or(false, |stat| stat.is_file())
    }

    fn server_script_path(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        let server_exists = self.server_exists();
        let server_path = self.server_file_path();
        if self.did_find_server && server_exists {
            return Ok(server_path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    if !self.server_exists() {
                        Err(format!(
                            "installed package '{PACKAGE_NAME}' did not contain expected path '{server_path}'",
                        ))?;
                    }
                }
                Err(error) => {
                    if !self.server_exists() {
                        Err(error)?;
                    }
                }
            }
        }

        self.did_find_server = true;
        Ok(server_path)
    }

    pub fn language_server_workspace_configuration(
        &mut self,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree("phptools", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();

        Ok(Some(serde_json::json!({
            "phptools": settings
        })))
    }
}
