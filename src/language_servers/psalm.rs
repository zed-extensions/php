use std::fs;
use std::path::PathBuf;

use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, serde_json, LanguageServerId, Result};

pub struct Psalm;

impl Psalm {
    pub const LANGUAGE_SERVER_ID: &'static str = "psalm";

    pub fn new() -> Self {
        Self
    }

    pub fn language_server_command(
        &self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let php_path = worktree.which("php").ok_or("PHP not found in PATH")?;

        let possible_commands = [
            "vendor/bin/psalm-language-server",
            "psalm-language-server", 
            "vendor/bin/psalm",
            "psalm"
        ];
        
        for command in &possible_commands {
            if let Some(found_path) = worktree.which(command) {
                let args = if command.contains("psalm-language-server") {
                    vec![]
                } else {
                    vec!["--language-server".to_string()]
                };
                
                return Ok(zed::Command {
                    command: php_path.clone(),
                    args: {
                        let mut cmd_args = vec![found_path];
                        cmd_args.extend(args);
                        cmd_args
                    },
                    env: Default::default(),
                });
            }
        }
        
        let vendor_bin_path = PathBuf::from(worktree.root_path()).join("vendor/bin/psalm-language-server");
        if vendor_bin_path.exists() && vendor_bin_path.is_file() {
            return Ok(zed::Command {
                command: php_path.clone(),
                args: vec![vendor_bin_path.to_string_lossy().to_string()],
                env: Default::default(),
            });
        }
        
        let psalm_vendor_path = PathBuf::from(worktree.root_path()).join("vendor/bin/psalm");
        if psalm_vendor_path.exists() && psalm_vendor_path.is_file() {
            return Ok(zed::Command {
                command: php_path.clone(),
                args: vec![psalm_vendor_path.to_string_lossy().to_string(), "--language-server".to_string()],
                env: Default::default(),
            });
        }

        if let Ok(lsp_settings) = LspSettings::for_worktree("psalm", worktree) {
            if let Some(initialization_options) = lsp_settings.initialization_options {
                if let Some(command_array) = initialization_options.get("command").and_then(|v| v.as_array()) {
                    if let Some(command) = command_array.get(0).and_then(|v| v.as_str()) {
                        let command_path = PathBuf::from(worktree.root_path()).join(command);
                        if command_path.exists() && command_path.is_file() {
                            return Ok(zed::Command {
                                command: php_path.clone(),
                                args: vec![command_path.to_string_lossy().to_string()],
                                env: Default::default(),
                            });
                        }
                    }
                }
            }
        }


        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Failed(
                "psalm-language-server not found. Install with: composer require --dev vimeo/psalm".to_string()
            ),
        );
        
        Err("psalm-language-server not found. Please install it with: composer require --dev vimeo/psalm".to_string())?
    }

    pub fn language_server_workspace_configuration(
        &self,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings = LspSettings::for_worktree("psalm", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();

        // Check if psalm.xml exists in the workspace
        let psalm_config_path = PathBuf::from(worktree.root_path()).join("psalm.xml");
        let has_config = fs::metadata(&psalm_config_path).map_or(false, |stat| stat.is_file());

        let mut config = serde_json::json!({
            "psalm": settings
        });

        // If psalm.xml exists, configure to use it
        if has_config {
            if let Some(psalm_settings) = config.get_mut("psalm").and_then(|v| v.as_object_mut()) {
                psalm_settings.insert(
                    "configPaths".to_string(),
                    serde_json::json!([psalm_config_path.to_string_lossy().to_string()])
                );
            }
        }

        Ok(Some(config))
    }
}