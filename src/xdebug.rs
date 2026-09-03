use std::{env, path::Path, str::FromStr, sync::OnceLock};

use zed_extension_api::{
    DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DownloadedFileType,
    GithubReleaseAsset, GithubReleaseOptions, StartDebuggingRequestArguments,
    StartDebuggingRequestArgumentsRequest, TaskTemplate, TcpArguments, TcpArgumentsTemplate,
    download_file, latest_github_release, node_binary_path, resolve_tcp_template,
    serde_json::{self, Value, json},
};

pub(super) struct XDebug {
    current_version: OnceLock<String>,
}

/// `tasks.json` quotes arguments for a shell; the adapter spawns argv directly.
fn strip_shell_quotes(arg: &str) -> String {
    arg.replace('"', "")
}

/// `php`, `php8.3`, `C:\php\php.exe`; not `phpunit` or `php-fpm`.
fn is_php_binary(command: &str) -> bool {
    let name = command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .to_ascii_lowercase();
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    stem.strip_prefix("php")
        .is_some_and(|version| version.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// A script the PHP interpreter can run directly: a Composer binstub, a `.php` file or a `.phar`.
fn is_php_entrypoint(command: &str) -> bool {
    let path = command.replace('\\', "/").to_ascii_lowercase();
    path.contains("vendor/bin/") || path.ends_with(".php") || path.ends_with(".phar")
}

/// `PHP_BINARY` lets a project point at a real `php.exe` when `php` on the PATH is a `.bat` shim.
fn resolve_php_runtime(worktree: &zed_extension_api::Worktree) -> Option<String> {
    worktree
        .shell_env()
        .into_iter()
        .find(|(key, _)| key == "PHP_BINARY")
        .map(|(_, value)| value)
        .filter(|value| !value.is_empty())
        .or_else(|| worktree.which("php"))
}

/// Fills in what a locator-built scenario lacks before the adapter spawns PHP.
/// A config without `program` listens for incoming connections instead and is
/// left alone: handing it a PHP binary would make it spawn `php` with no script.
fn complete_launch_config(
    obj: &mut serde_json::Map<String, Value>,
    worktree_root: String,
    resolve_php: impl FnOnce() -> Option<String>,
) -> Result<(), String> {
    // Locator scenarios carry `"cwd": null`, which `entry().or_insert` would keep.
    if obj.get("cwd").is_none_or(Value::is_null) {
        obj.insert("cwd".to_string(), worktree_root.into());
    }
    let launches_program = obj
        .get("program")
        .and_then(Value::as_str)
        .is_some_and(|program| !program.is_empty());
    if !launches_program {
        return Ok(());
    }
    // On Windows `spawn("php")` doesn't find `php.exe` on the PATH.
    if !obj.contains_key("runtimeExecutable")
        && let Some(php) = resolve_php()
    {
        obj.insert("runtimeExecutable".to_string(), php.into());
    }
    // The adapter doesn't enable Xdebug itself; it substitutes `${port}`
    // with the DBGp port it listens on.
    if !obj.contains_key("runtimeArgs") {
        obj.insert(
            "runtimeArgs".to_string(),
            json!([
                "-dxdebug.mode=debug",
                "-dxdebug.start_with_request=yes",
                "-dxdebug.client_port=${port}"
            ]),
        );
    }
    // Node refuses to spawn `.bat`/`.cmd` (CVE-2024-27980) with a bare `spawn EINVAL`.
    if let Some(runtime) = obj.get("runtimeExecutable").and_then(Value::as_str) {
        let ext = runtime.to_ascii_lowercase();
        if ext.ends_with(".bat") || ext.ends_with(".cmd") {
            return Err(format!(
                "Cannot debug through the shell shim `{runtime}`: the debug adapter \
                 spawns PHP directly and Windows batch files can't be launched that way. \
                 Point `PHP_BINARY` (or a `runtimeExecutable` in your debug config) at a real `php.exe`."
            ));
        }
    }
    Ok(())
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
                        .then_some(StartDebuggingRequestArgumentsRequest::Launch)
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
    /// Offers a "Debug" counterpart to a runnable task that launches a PHP entrypoint.
    pub(crate) fn dap_locator_create_scenario(
        &self,
        build_task: TaskTemplate,
        resolved_label: String,
        adapter: String,
    ) -> Option<DebugScenario> {
        if adapter != Self::NAME {
            return None;
        }

        // `php -r <code>` has nothing to break in; `composer test` would run as
        // `php composer test`.
        let (program, args) = if is_php_binary(&build_task.command) {
            let program = build_task.args.first()?;
            if program.starts_with('-') {
                return None;
            }
            (program.clone(), build_task.args[1..].to_vec())
        } else if is_php_entrypoint(&build_task.command) {
            (build_task.command.clone(), build_task.args.clone())
        } else {
            return None;
        };
        let program = strip_shell_quotes(&program);
        let args: Vec<String> = args.iter().map(|a| strip_shell_quotes(a)).collect();

        let env = Value::Object(
            build_task
                .env
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        );

        let config = json!({
            "request": "launch",
            "program": program,
            "args": args,
            "cwd": build_task.cwd,
            "env": env,
            "stopOnEntry": false,
        });

        Some(DebugScenario {
            adapter,
            label: resolved_label,
            build: None,
            config: config.to_string(),
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
                .ok_or_else(|| "no installed version of Xdebug found".to_string())?;
            env::current_dir()
                .unwrap()
                .join(Self::NAME)
                .join(format!("{}_{version}", Self::NAME))
                .to_string_lossy()
                .into_owned()
        };

        let tcp_connection = task_definition
            .tcp_connection
            .unwrap_or(TcpArgumentsTemplate {
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
            complete_launch_config(obj, worktree.root_path(), || resolve_php_runtime(worktree))?;
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
                request: self.dap_request_kind(&configuration)?,
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
                    .flatten()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn task(command: &str, args: &[&str]) -> TaskTemplate {
        TaskTemplate {
            label: "run".into(),
            command: command.into(),
            args: args.iter().map(|a| (*a).into()).collect(),
            env: vec![],
            cwd: None,
        }
    }

    fn scenario_config(build_task: TaskTemplate) -> Value {
        let scenario = XDebug::new()
            .dap_locator_create_scenario(build_task, "label".into(), XDebug::NAME.into())
            .expect("locator claims the task");
        assert_eq!(scenario.adapter, XDebug::NAME);
        Value::from_str(&scenario.config).expect("scenario config is JSON")
    }

    #[test]
    fn testo_command_launches_the_script_after_php() {
        let config = scenario_config(task(
            "php",
            &["vendor/bin/testo", "--path=tests/Foo.php", "--filter=bar"],
        ));
        assert_eq!(config["request"], "launch");
        assert_eq!(config["program"], "vendor/bin/testo");
        assert_eq!(
            config["args"],
            json!(["--path=tests/Foo.php", "--filter=bar"])
        );
    }

    #[test]
    fn phpunit_binary_is_itself_the_program() {
        let config = scenario_config(task(
            "./vendor/bin/phpunit",
            &["--filter", "bar", "tests/Foo.php"],
        ));
        assert_eq!(config["program"], "./vendor/bin/phpunit");
        assert_eq!(config["args"], json!(["--filter", "bar", "tests/Foo.php"]));
    }

    #[test]
    fn shell_quotes_are_stripped_from_argv() {
        let config = scenario_config(task(
            "php",
            &[
                "vendor/bin/testo",
                "--path=\"tests/Foo.php\"",
                "--filter=\"bar\"",
            ],
        ));
        assert_eq!(
            config["args"],
            json!(["--path=tests/Foo.php", "--filter=bar"])
        );
    }

    #[test]
    fn versioned_and_windows_php_binaries_are_recognized() {
        for php in ["php8.3", "/usr/bin/php", "C:\\php\\php.exe", "PHP.EXE"] {
            let config = scenario_config(task(php, &["vendor/bin/testo", "--filter=bar"]));
            assert_eq!(config["program"], "vendor/bin/testo", "{php}");
            assert_eq!(config["args"], json!(["--filter=bar"]), "{php}");
        }
    }

    #[test]
    fn phar_is_an_entrypoint() {
        let config = scenario_config(task("tools/phpunit.phar", &["tests"]));
        assert_eq!(config["program"], "tools/phpunit.phar");
    }

    #[test]
    fn non_php_commands_are_declined() {
        for command in ["composer", "make", "php-fpm", "phpunit", "npm"] {
            assert!(
                XDebug::new()
                    .dap_locator_create_scenario(
                        task(command, &["test"]),
                        "label".into(),
                        XDebug::NAME.into(),
                    )
                    .is_none(),
                "{command}"
            );
        }
    }

    #[test]
    fn inline_php_code_is_not_debuggable() {
        assert!(
            XDebug::new()
                .dap_locator_create_scenario(
                    task("php", &["-r", "echo 1;"]),
                    "label".into(),
                    XDebug::NAME.into(),
                )
                .is_none()
        );
    }

    #[test]
    fn other_adapters_are_declined() {
        assert!(
            XDebug::new()
                .dap_locator_create_scenario(
                    task("php", &["vendor/bin/testo"]),
                    "label".into(),
                    "SomethingElse".into(),
                )
                .is_none()
        );
    }

    fn completed(mut config: Value, php: Option<&str>) -> Result<Value, String> {
        let obj = config.as_object_mut().expect("config is an object");
        complete_launch_config(obj, "/root".into(), || php.map(str::to_string))?;
        Ok(config)
    }

    #[test]
    fn launch_config_gets_cwd_php_and_xdebug_args() {
        let config = completed(
            json!({"program": "vendor/bin/testo", "cwd": null}),
            Some("/usr/bin/php"),
        )
        .unwrap();
        assert_eq!(config["cwd"], "/root");
        assert_eq!(config["runtimeExecutable"], "/usr/bin/php");
        assert_eq!(
            config["runtimeArgs"],
            json!([
                "-dxdebug.mode=debug",
                "-dxdebug.start_with_request=yes",
                "-dxdebug.client_port=${port}"
            ])
        );
    }

    #[test]
    fn explicit_launch_settings_are_kept() {
        let explicit = json!({
            "program": "app.php",
            "cwd": "/app",
            "runtimeExecutable": "/opt/php/bin/php",
            "runtimeArgs": ["-dxdebug.mode=debug,coverage"],
        });
        let config = completed(explicit.clone(), Some("/usr/bin/php")).unwrap();
        assert_eq!(config, explicit);
    }

    #[test]
    fn listener_config_only_gets_a_cwd() {
        let config = completed(json!({"request": "launch"}), Some("/usr/bin/php")).unwrap();
        assert_eq!(config, json!({"request": "launch", "cwd": "/root"}));
    }

    #[test]
    fn batch_shim_is_rejected() {
        let error =
            completed(json!({"program": "app.php"}), Some("C:\\tools\\php.BAT")).unwrap_err();
        assert!(error.contains("php.BAT"), "{error}");
    }
}
