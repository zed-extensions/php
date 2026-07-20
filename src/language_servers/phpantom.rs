use std::fs;

use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, DownloadedFileType, LanguageServerId, Result};

const REPO: &str = "PHPantom-dev/phpantom_lsp";
const BINARY_NAME: &str = "phpantom_lsp";

pub struct Phpantom {
    cached_binary_path: Option<String>,
}

impl Phpantom {
    pub const LANGUAGE_SERVER_ID: &'static str = "phpantom";

    pub fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    pub fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Allow users to point at their own build via
        // `lsp.phpantom.binary.{path,arguments}` in the settings.
        if let Some(binary) = LspSettings::for_worktree("phpantom", worktree)
            .ok()
            .and_then(|settings| settings.binary)
        {
            if let Some(path) = binary.path {
                return Ok(zed::Command {
                    command: path,
                    args: binary.arguments.unwrap_or_default(),
                    env: Default::default(),
                });
            }
        }

        Ok(zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = worktree.which(BINARY_NAME) {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();

        let (os_str, file_type) = match platform {
            zed::Os::Mac => ("apple-darwin", DownloadedFileType::GzipTar),
            zed::Os::Linux => ("unknown-linux-gnu", DownloadedFileType::GzipTar),
            zed::Os::Windows => ("pc-windows-msvc", DownloadedFileType::Zip),
        };

        let arch_str = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            _ => return Err(format!("unsupported architecture: {arch:?}")),
        };

        let asset_name = format!(
            "{BINARY_NAME}-{arch_str}-{os_str}.{extension}",
            extension = match file_type {
                DownloadedFileType::GzipTar => "tar.gz",
                DownloadedFileType::Zip => "zip",
                _ => "",
            }
        );
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| {
                format!(
                    "no release asset found matching {asset_name:?} — you may need to build \
                     {BINARY_NAME} from source for your platform"
                )
            })?;

        let version_dir = format!("{BINARY_NAME}-{}", release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("failed to create directory: {e}"))?;

        let binary_path = match platform {
            zed::Os::Windows => format!("{version_dir}/{BINARY_NAME}.exe"),
            _ => format!("{version_dir}/{BINARY_NAME}"),
        };

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory: {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry: {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}
