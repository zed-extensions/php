use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, Result};

const MINIMUM_PHP_VERSION_ID: u32 = 80200;

pub struct Phpactor {
    cached_binary_path: Option<String>,
}

impl Phpactor {
    pub const LANGUAGE_SERVER_ID: &'static str = "phpactor";

    pub fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    pub fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = worktree.which("phpactor") {
            return Ok(path);
        }

        Self::check_php_version(language_server_id)?;

        if let Some(path) = &self.cached_binary_path
            && fs::metadata(path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(path.clone());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "phpactor/phpactor",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = "phpactor.phar";
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("phpactor-{}", release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

        let binary_path = format!("{version_dir}/phpactor.phar");

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn check_php_version(language_server_id: &LanguageServerId) -> Result<()> {
        let mut command = zed::process::Command::new("php")
            .arg("-r")
            .arg("echo PHP_VERSION_ID;");

        let output = command.output().map_err(|error| {
                    Self::fail(
                        language_server_id,
                        &format!(
                            "Could not run `php` from PATH: {error}. phpactor requires PHP >= 8.2. Install PHP 8.2+ or switch to a language server that does not need PHP at runtime (intelephense, phpantom)."
                        ),
                    )
                })?;

        let version_id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .ok();

        match version_id {
            Some(version) if version >= MINIMUM_PHP_VERSION_ID => Ok(()),
            Some(version) => Err(Self::fail(
                language_server_id,
                &format!(
                    "phpactor requires PHP >= 8.2, but the `php` in PATH is PHP {}. Install PHP 8.2+ or switch to a language server that does not need PHP at runtime (intelephense, phpantom).",
                    Self::format_php_version(version)
                ),
            )),
            None => Err(Self::fail(
                language_server_id,
                "Could not determine the version of `php` in PATH. phpactor requires PHP >= 8.2. Install PHP 8.2+ or switch to a language server that does not need PHP at runtime (intelephense, phpantom).",
            )),
        }
    }

    fn format_php_version(version_id: u32) -> String {
        format!(
            "{}.{}.{}",
            version_id / 10000,
            (version_id / 100) % 100,
            version_id % 100
        )
    }

    fn fail(language_server_id: &LanguageServerId, message: &str) -> String {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Failed(message.to_string()),
        );
        message.to_string()
    }
}
