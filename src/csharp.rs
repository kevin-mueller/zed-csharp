mod dotnet_run;
mod language_servers;
mod msbuild;
mod netcoredbg_binary_manager;
mod simple_temp_dir;

use std::collections::HashMap;
use std::path::Path;

use dotnet_run::{launch_profile, resolve_path, DotnetInvocation, Subcommand};
use msbuild::build_project;

use language_servers::{CsharpLs, HtmlLanguageServer, RoslynOfficial};
use netcoredbg_binary_manager::NetCoreDbgBinaryManager;
use serde::{Deserialize, Serialize};
use zed_extension_api::{
    self as zed,
    serde_json::{self},
    DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DebugTaskDefinition, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

struct CsharpExtension {
    roslyn_official: Option<RoslynOfficial>,
    csharp_ls: Option<CsharpLs>,
    html: Option<HtmlLanguageServer>,
    binary_manager: NetCoreDbgBinaryManager,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NetCoreDbgDebugConfig {
    pub request: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_at_entry: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_id: Option<ProcessId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub just_my_code: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_step_filtering: Option<bool>,
}

/// Represents a process id that can be either an integer or a string (containing a number)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum ProcessId {
    Int(i32),
    String(String),
}

/// Env vars that carry a task's original command line from `dap_locator_create_scenario` to
/// `run_dap_locator`, one variable per argument so Zed resolves the $ZED variables in each.
const DEBUG_COMMAND_ENV: &str = "CSHARP_DEBUG_COMMAND";
const DEBUG_ARG_ENV_PREFIX: &str = "CSHARP_DEBUG_ARG_";

impl CsharpExtension {
    const ADAPTER_NAME: &str = "netcoredbg";
    const PROJECT_RUNNER_LOCATOR_NAME: &str = "csharp-project-runner";
}

impl zed::Extension for CsharpExtension {
    fn new() -> Self {
        Self {
            roslyn_official: None,
            csharp_ls: None,
            html: None,
            binary_manager: NetCoreDbgBinaryManager::new(),
        }
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        if adapter_name != Self::ADAPTER_NAME {
            return Err(format!("Cannot create binary for adapter: {adapter_name}"));
        }

        let configuration = config.config.to_string();
        let parsed_config: NetCoreDbgDebugConfig =
                    serde_json::from_str(&configuration).map_err(|e| {
                        format!("Failed to parse debug configuration: {}. Expected NetCoreDbg configuration format.", e)
                    })?;

        let request = match parsed_config.request.as_str() {
            "launch" => StartDebuggingRequestArgumentsRequest::Launch,
            "attach" => StartDebuggingRequestArgumentsRequest::Attach,
            other => {
                return Err(format!(
                    "Invalid 'request' value: '{}'. Expected 'launch' or 'attach'",
                    other
                ))
            }
        };

        let binary_path = self
            .binary_manager
            .get_binary_path(user_provided_debug_adapter_path)?;

        Ok(DebugAdapterBinary {
            command: Some(binary_path),
            arguments: vec!["--interpreter=vscode".to_string()],
            envs: parsed_config.env.into_iter().collect(),
            cwd: Some(parsed_config.cwd.unwrap_or_else(|| worktree.root_path())),
            connection: None,
            request_args: StartDebuggingRequestArguments {
                configuration,
                request,
            },
        })
    }

    fn dap_request_kind(
        &mut self,
        _adapter_name: String,
        _config: zed::serde_json::Value,
    ) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        if _adapter_name != Self::ADAPTER_NAME {
            return Err(format!("Unknown adapter: {}", _adapter_name));
        }

        match _config.get("request").and_then(|v| v.as_str()) {
            Some("launch") => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            Some("attach") => Ok(StartDebuggingRequestArgumentsRequest::Attach),
            Some(other) => Err(format!(
                "Invalid 'request' value: '{}'. Expected 'launch' or 'attach'",
                other
            )),
            None => Err(
                "Debug configuration missing required 'request' field. Must be 'launch' or 'attach'"
                    .to_string(),
            ),
        }
    }

    fn dap_config_to_scenario(
        &mut self,
        _adapter_name: DebugConfig,
    ) -> Result<DebugScenario, String> {
        match _adapter_name.request {
            DebugRequest::Launch(launch) => {
                let adapter_config = NetCoreDbgDebugConfig {
                    request: "launch".to_string(),
                    program: Some(launch.program),
                    args: if launch.args.is_empty() {
                        None
                    } else {
                        Some(launch.args)
                    },
                    cwd: launch.cwd,
                    env: launch.envs.into_iter().collect(),
                    stop_at_entry: _adapter_name.stop_on_entry,
                    process_id: None,
                    just_my_code: None,
                    enable_step_filtering: None,
                };

                let config_json = zed::serde_json::to_string(&adapter_config)
                    .map_err(|e| format!("Failed to serialize launch config: {}", e))?;

                Ok(DebugScenario {
                    label: _adapter_name.label,
                    adapter: _adapter_name.adapter,
                    build: None,
                    config: config_json,
                    tcp_connection: None,
                })
            }
            DebugRequest::Attach(attach) => {
                let process_id = attach.process_id.ok_or_else(|| {
                    "Attach mode requires a process ID. Please select a process from the attach modal.".to_string()
                })?;

                let adapter_config = NetCoreDbgDebugConfig {
                    request: "attach".to_string(),
                    program: None,
                    args: None,
                    cwd: None,
                    env: HashMap::new(),
                    stop_at_entry: _adapter_name.stop_on_entry,
                    process_id: Some(ProcessId::Int(process_id.try_into().map_err(|_| {
                        format!("Process ID {} is too large to fit in i32", process_id)
                    })?)),
                    just_my_code: None,
                    enable_step_filtering: None,
                };

                let config_json = zed::serde_json::to_string(&adapter_config)
                    .map_err(|e| format!("Failed to serialize attach config: {}", e))?;

                Ok(DebugScenario {
                    label: _adapter_name.label,
                    adapter: _adapter_name.adapter,
                    build: None,
                    config: config_json,
                    tcp_connection: None,
                })
            }
        }
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: zed::TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<DebugScenario> {
        if debug_adapter_name != Self::ADAPTER_NAME {
            return None;
        }

        // Handle project runner locator
        if locator_name == Self::PROJECT_RUNNER_LOCATOR_NAME {
            // The extension's own task scripts (see languages/csharp/scripts) get their
            // project from the DOTNET_FILE env var, so they are treated like a bare
            // `dotnet run` / `dotnet test`. Any other task must be a `dotnet run` / `dotnet test`.
            let script_command = (build_task.command == "sh")
                .then(|| {
                    build_task.args.iter().find_map(|arg| match arg.as_str() {
                        "run-project.sh" => Some("dotnet run"),
                        "test-project.sh" | "run-tests.sh" => Some("dotnet test"),
                        _ => None,
                    })
                })
                .flatten();
            let (command, args) = match script_command {
                Some(command) => (command.to_string(), Vec::new()),
                None => {
                    DotnetInvocation::parse(&build_task.command, &build_task.args)?;
                    (build_task.command.clone(), build_task.args.clone())
                }
            };

            // The $ZED variables are not resolved yet at this point, so we can't tell which
            // project to build. Instead, the command is stashed in the env of a placeholder
            // build step. Zed resolves the variables in it, and run_dap_locator builds the project.
            let mut env = build_task.env;
            env.push((DEBUG_COMMAND_ENV.to_string(), command));
            env.extend(
                args.into_iter()
                    .enumerate()
                    .map(|(index, arg)| (format!("{DEBUG_ARG_ENV_PREFIX}{index}"), arg)),
            );

            let template = zed::BuildTaskTemplate {
                label: "prepare debug session".to_string(),
                command: "echo".to_string(),
                cwd: build_task.cwd,
                args: vec!["'Building the project for debugging...'".to_string()],
                env,
            };

            let build_template =
                zed::BuildTaskDefinition::Template(zed::BuildTaskDefinitionTemplatePayload {
                    locator_name: Some(locator_name.clone()),
                    template,
                });

            return Some(DebugScenario {
                adapter: debug_adapter_name,
                label: resolved_label,
                build: Some(build_template),
                config: "{}".into(), // no config, so we run_dap_locator
                tcp_connection: None,
            });
        }

        None
    }

    fn run_dap_locator(
        &mut self,
        locator_name: String,
        build_task: zed::TaskTemplate,
    ) -> Result<DebugRequest, String> {
        if locator_name == Self::PROJECT_RUNNER_LOCATOR_NAME {
            // Here the $ZED variables in the env are resolved, so we can find the project.
            // Later entries win, like task env over the project env.
            let env: HashMap<String, String> = build_task.env.into_iter().collect();

            let command = env
                .get(DEBUG_COMMAND_ENV)
                .ok_or("The debug session was not prepared by the C# extension")?;
            let args: Vec<String> = (0..)
                .map_while(|index| env.get(&format!("{DEBUG_ARG_ENV_PREFIX}{index}")).cloned())
                .collect();
            let invocation = DotnetInvocation::parse(command, &args)
                .ok_or_else(|| format!("Not a `dotnet run` or `dotnet test` command: {command}"))?;

            // Where `dotnet run` would run: the task's cwd (if Zed resolved it), or else the
            // worktree root.
            let task_dir = build_task
                .cwd
                .filter(|cwd| !cwd.contains('$'))
                .or_else(|| env.get("ZED_WORKTREE_ROOT").cloned())
                .ok_or("Unable to determine the task's working directory")?;

            // `--project` (or `dotnet test`'s project argument) wins over the extension's
            // DOTNET_FILE, then the project is searched upwards from the task's directory.
            let project_path = match (&invocation.project, env.get("DOTNET_FILE")) {
                (Some(project), _) => resolve_path(&task_dir, project),
                (None, Some(file)) => file.clone(),
                (None, None) => task_dir.clone(),
            };
            if project_path.ends_with(".sln") || project_path.ends_with(".slnx") {
                return Err(format!(
                    "Can't debug a whole solution ({project_path}), point the task at a project instead"
                ));
            }
            let csproj = find_csproj(&project_path)
                .ok_or_else(|| format!("Failed to locate a csproj file for {project_path}"))?;
            let project_dir = Path::new(&csproj)
                .parent()
                .ok_or_else(|| format!("Failed to get the directory of {csproj}"))?
                .to_string_lossy()
                .to_string();

            let build = build_project(&csproj, &invocation)?;

            // The program inherits the task's env, without our internal variables.
            let mut launch_env: HashMap<String, String> = env
                .iter()
                .filter(|(key, _)| {
                    *key != DEBUG_COMMAND_ENV && !key.starts_with(DEBUG_ARG_ENV_PREFIX)
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let mut program_args = invocation.program_args.clone();

            let cwd = match invocation.subcommand {
                Subcommand::Run => {
                    let profile = if invocation.no_launch_profile {
                        None
                    } else {
                        let launch_settings =
                            read_file(&format!("{project_dir}/Properties/launchSettings.json"));
                        launch_profile(
                            launch_settings.as_deref(),
                            invocation.launch_profile.as_deref(),
                            &env,
                        )?
                    };

                    let mut profile_dir = None;
                    if let Some(profile) = profile {
                        launch_env.extend(profile.env);
                        // Like `dotnet run`, arguments after `--` replace the profile's.
                        if program_args.is_empty() {
                            program_args = profile.args;
                        }
                        profile_dir = profile.working_directory;
                    }

                    // `dotnet run` ignores the profile's workingDirectory, but IDEs respect it.
                    profile_dir
                        .or(build.run_working_directory)
                        .map(|dir| resolve_path(&project_dir, &dir))
                        .unwrap_or(task_dir)
                }
                Subcommand::Test => {
                    if let Some(class) = env.get("CSHARP_TEST_CLASS") {
                        program_args.extend(["-class".to_string(), format!("*.{class}")]);
                    }
                    if let Some(method) = env.get("CSHARP_TEST_METHOD") {
                        program_args.extend(["-method".to_string(), format!("*.{method}")]);
                    }
                    Path::new(&build.target_path)
                        .parent()
                        .ok_or_else(|| {
                            format!("Failed to get the directory of {}", build.target_path)
                        })?
                        .to_string_lossy()
                        .to_string()
                }
            };
            launch_env.extend(invocation.environment.iter().cloned());

            return Ok(DebugRequest::Launch(zed::LaunchRequest {
                program: build.target_path,
                args: program_args,
                cwd: Some(cwd),
                envs: launch_env.into_iter().collect(),
            }));
        }

        return Err(format!("Unknown locator: {locator_name}"));
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        match language_server_id.as_ref() {
            RoslynOfficial::LANGUAGE_SERVER_ID => {
                let roslyn_official = self.roslyn_official.get_or_insert_with(RoslynOfficial::new);
                roslyn_official.language_server_cmd(language_server_id, worktree)
            }
            CsharpLs::LANGUAGE_SERVER_ID => {
                let csharp_ls = self.csharp_ls.get_or_insert_with(CsharpLs::new);
                csharp_ls.language_server_command(language_server_id, worktree)
            }
            HtmlLanguageServer::LANGUAGE_SERVER_ID => {
                let html = self.html.get_or_insert_with(HtmlLanguageServer::new);
                html.language_server_command(language_server_id, worktree)
            }
            language_server_id => Err(format!("unknown language server: {language_server_id}")),
        }
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        match language_server_id.as_ref() {
            RoslynOfficial::LANGUAGE_SERVER_ID => RoslynOfficial::configuration_options(worktree),
            CsharpLs::LANGUAGE_SERVER_ID => CsharpLs::configuration_options(worktree),
            HtmlLanguageServer::LANGUAGE_SERVER_ID => {
                HtmlLanguageServer::configuration_options(worktree)
            }
            _ => Ok(None),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        match language_server_id.as_ref() {
            HtmlLanguageServer::LANGUAGE_SERVER_ID => {
                HtmlLanguageServer::initialization_options(worktree)
            }
            _ => Ok(None),
        }
    }
}

zed::register_extension!(CsharpExtension);

fn find_csproj(start_path: &str) -> Option<String> {
    let mut current_path = start_path.to_string();

    // Walk up the directory tree
    loop {
        // Try to find .csproj in current directory
        let result = zed_extension_api::process::Command::new("find")
            .arg(&current_path)
            .arg("-maxdepth")
            .arg("1")
            .arg("-name")
            .arg("*.csproj")
            .arg("-print")
            .arg("-quit")
            .output()
            .ok()?;

        let output = String::from_utf8_lossy(&result.stdout).trim().to_string();

        if !output.is_empty() {
            // Found it!
            return Some(output);
        }

        // Strip one directory level
        if let Some(last_slash) = current_path.rfind('/') {
            if last_slash == 0 {
                break; // Reached root
            }
            current_path = current_path[..last_slash].to_string();
        } else {
            break; // No more parent directories
        }
    }

    None // Not found
}

/// The contents of a file, or `None` if it can't be read.
/// Uses `cat`, as std::fs doesn't work in the debug locator.
fn read_file(path: &str) -> Option<String> {
    let output = zed::Command::new("cat").arg(path).output().ok()?;
    (output.status == Some(0)).then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
