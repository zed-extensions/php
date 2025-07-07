use std::{env, path::Path, str::FromStr, sync::OnceLock};

use zed_extension_api::{
    download_file, latest_github_release, node_binary_path, resolve_tcp_template,
    serde_json::{self, json, Value},
    DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DownloadedFileType,
    GithubReleaseAsset, GithubReleaseOptions, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TcpArguments, TcpArgumentsTemplate,
};

pub(super) struct XDebug {
    current_version: OnceLock<String>,
}

impl XDebug {
    pub(super) const NAME: &'static str = "Xdebug";
    const ADAPTER_PATH: &'static str = "extension/out/phpDebug.js";
    pub(super) fn new() -> Self {
        Self {
            current_version: Default::default(),
        }
    }
    pub(super) fn dap_request_kind(
        &self,
        config: &serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        config
            .get("request")
            .and_then(|v| {
                v.as_str().and_then(|s| {
                    s.eq("launch")
                        .then(|| StartDebuggingRequestArgumentsRequest::Launch)
                })
            })
            .ok_or_else(|| "Invalid config".into())
    }

    pub(crate) fn dap_config_to_scenario(
        &self,
        config: DebugConfig,
    ) -> Result<DebugScenario, String> {
        let obj = match &config.request {
            DebugRequest::Attach(_) => {
                return Err("Php adapter doesn't support attaching".into());
            }
            DebugRequest::Launch(launch_config) => json!({
                "program": launch_config.program,
                "cwd": launch_config.cwd,
                "args": launch_config.args,
                "env": serde_json::Value::Object(
                    launch_config.envs
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_owned().into()))
                        .collect::<serde_json::Map<String, serde_json::Value>>(),
                ),
                "stopOnEntry": config.stop_on_entry.unwrap_or_default(),
            }),
        };

        Ok(DebugScenario {
            adapter: config.adapter,
            label: config.label,
            build: None,
            config: obj.to_string(),
            tcp_connection: None,
        })
    }
    fn fetch_latest_adapter_version() -> Result<(GithubReleaseAsset, String), String> {
        let release = latest_github_release(
            "xdebug/vscode-php-debug",
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = format!("php-debug-{}.vsix", release.version.trim_start_matches("v"));

        let asset = release
            .assets
            .into_iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

        Ok((asset, release.version))
    }

    fn get_installed_binary(
        &mut self,
        task_definition: zed_extension_api::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<zed_extension_api::DebugAdapterBinary, String> {
        let adapter_path = if let Some(user_installed_path) = user_provided_debug_adapter_path {
            user_installed_path
        } else {
            let version = self
                .current_version
                .get()
                .cloned()
                .ok_or_else(|| format!("no installed version of Xdebug found"))?;
            env::current_dir()
                .unwrap()
                .join(Self::NAME)
                .join(format!("{}_{version}", Self::NAME))
                .to_string_lossy()
                .into_owned()
        };

        let tcp_connection =
            task_definition
                .tcp_connection
                .clone()
                .unwrap_or_else(|| TcpArgumentsTemplate {
                    host: None,
                    port: None,
                    timeout: None,
                });
        let TcpArguments {
            host,
            port,
            timeout,
        } = resolve_tcp_template(tcp_connection)?;

        let mut configuration = Value::from_str(&task_definition.config)
            .map_err(|e| format!("Invalid JSON configuration: {e}"))?;
        if let Some(obj) = configuration.as_object_mut() {
            obj.entry("cwd")
                .or_insert_with(|| worktree.root_path().into());
        }

        Ok(DebugAdapterBinary {
            command: Some(node_binary_path()?),
            arguments: vec![
                Path::new(&adapter_path)
                    .join(Self::ADAPTER_PATH)
                    .to_string_lossy()
                    .into_owned(),
                format!("--server={}", port),
            ],
            connection: Some(TcpArguments {
                port,
                host,
                timeout,
            }),
            cwd: Some(worktree.root_path()),
            envs: vec![],
            request_args: StartDebuggingRequestArguments {
                request: self.dap_request_kind(&configuration)?.into(),
                configuration: configuration.to_string(),
            },
        })
    }
    pub(crate) fn get_binary(
        &mut self,
        config: zed_extension_api::DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<zed_extension_api::DebugAdapterBinary, String> {
        if self.current_version.get_mut().is_none() {
            if let Ok((asset, version)) = Self::fetch_latest_adapter_version() {
                let output_path = format!("{0}/{0}_{1}", Self::NAME, version);
                if !Path::new(&output_path).exists() {
                    std::fs::remove_dir_all(Self::NAME).ok();
                    std::fs::create_dir_all(Self::NAME)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                    download_file(&asset.download_url, &output_path, DownloadedFileType::Zip)?;
                }
                self.current_version.set(version).ok();
            } else {
                // Just find the highest version we currently have.
                let prefix = format!("{}_", Self::NAME);
                let mut version = std::fs::read_dir(Self::NAME)
                    .ok()
                    .into_iter()
                    .flat_map(|e| e)
                    .filter_map(|e| {
                        e.ok().and_then(|entry| {
                            entry
                                .file_name()
                                .to_string_lossy()
                                .strip_prefix(&prefix)
                                .map(ToOwned::to_owned)
                        })
                    })
                    .max();

                if let Some(version) = version.take() {
                    self.current_version.set(version).ok();
                }
            }
        }
        self.get_installed_binary(config, user_provided_debug_adapter_path, worktree)
    }
}
