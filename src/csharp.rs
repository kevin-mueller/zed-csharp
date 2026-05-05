mod language_servers;
mod netcoredbg_binary_manager;
mod simple_temp_dir;

use std::collections::HashMap;

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
        _locator_name: String,
        _build_task: zed::TaskTemplate,
        _resolved_label: String,
        _debug_adapter_name: String,
    ) -> Option<DebugScenario> {
        if _debug_adapter_name != Self::ADAPTER_NAME {
            return None;
        }

        // Handle project runner locator
        if _locator_name == Self::PROJECT_RUNNER_LOCATOR_NAME {
            // Check if command is "dotnet" or starts with "dotnet run" or is a variable
            let is_dotnet_command = _build_task.command.starts_with("dotnet")
                && (_build_task
                    .args
                    .iter()
                    .any(|arg| arg == "run" || arg == "test")
                    || _build_task.command.contains("run")
                    || _build_task.command.contains("test"));

            if !is_dotnet_command {
                return None;
            }

            // unfortunately the $ZED variables are not resolved here yet
            // so we have no way of figuring out what the build context (csproj) should be
            // for now we're deferring the build step to the run_dap_locator method (yes very ugly, I know. But it works.)
            // another alternative would be to build the entire solution here, but that can be annoying when not all projects are compiling
            //
            let mut var_to_test = env_value(&_build_task.env, "DOTNET_FILE")?;
            var_to_test.remove(0);
            let args = vec![
                "$ZED variables not resolved yet, deferring build step.".into(),
                format!(
                    "variable content: {:?}",
                    var_to_test // strip the first char so the shell doesn't resolve it
                ),
            ];

            let template = zed::BuildTaskTemplate {
                label: "echo info".to_string(),
                command: "echo".to_string(),
                cwd: _build_task.cwd,
                args,
                env: _build_task.env,
            };

            let build_template =
                zed::BuildTaskDefinition::Template(zed::BuildTaskDefinitionTemplatePayload {
                    locator_name: Some(_locator_name.clone()),
                    template,
                });

            return Some(DebugScenario {
                adapter: _debug_adapter_name,
                label: _resolved_label,
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
        _build_task: zed::TaskTemplate,
    ) -> Result<DebugRequest, String> {
        if locator_name == Self::PROJECT_RUNNER_LOCATOR_NAME {
            // here the values in .env are actually resolved!
            // we can resolve the appropriate csproj & .dll that way

            let file_path = match env_value(&_build_task.env, "DOTNET_FILE") {
                Some(fp) => fp,
                None => return Err("Failed to get file path from env var. Make sure your run task defines a env variable called DOTNET_FILE with the value '$ZED_FILE'".to_string()),
            };

            let csproj = match find_csproj(&file_path) {
                Some(x) => x,
                None => {
                    return Err(format!(
                        "Failed to locate csproj file from file path: {}",
                        &file_path
                    ))
                }
            };

            _ = zed_extension_api::process::Command::new("dotnet")
                .arg("build")
                .arg(&csproj)
                .arg("--configuration")
                .arg("Debug")
                .output()?;

            let dll_path_result = &find_dll_from_csproj_or_build_props(&csproj, None);
            let dll_path = match dll_path_result {
                Err(_) => {
                    // fallback with Directory.Build.Props
                    let worktree_root = &env_value(&_build_task.env, "ZED_WORKTREE_ROOT")
                        .ok_or("Unable to resolve worktree root from env var")?
                        .to_string();
                    let build_props_path = format!("{}/Directory.Build.Props", worktree_root);

                    find_dll_from_csproj_or_build_props(&csproj, Some(&build_props_path))?
                }
                Ok(path) => path.to_string(),
            };

            let mut args: Vec<String> = vec![];

            let test_class_name = env_value(&_build_task.env, "CSHARP_TEST_CLASS");
            let test_method_name = env_value(&_build_task.env, "CSHARP_TEST_METHOD");

            if test_class_name.is_some() {
                args.push("-class".into());
                args.push(format!("*.{}", test_class_name.unwrap()));
            }

            if test_method_name.is_some() {
                args.push("-method".into());
                args.push(format!("*.{}", test_method_name.unwrap()));
            }

            let last_slash_index = dll_path
                .rfind('/')
                .ok_or_else(|| "Failed to find last slash in dll path".to_string())?;

            let cwd = &dll_path[..last_slash_index];

            return Ok(DebugRequest::Launch(zed::LaunchRequest {
                program: dll_path.to_string(),
                args: args,
                cwd: Some(cwd.to_string()),
                envs: _build_task.env,
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

fn find_dll_from_csproj_or_build_props(
    csproj_path: &str,
    build_props_path: Option<&str>,
) -> Result<String, String> {
    let file_to_parse = build_props_path.unwrap_or(csproj_path);

    let xml_output = zed::Command::new("cat")
        .arg(file_to_parse)
        .output()
        .map_err(|e| e.to_string())?;

    let xml_content = String::from_utf8_lossy(&xml_output.stdout)
        .trim()
        .to_string();

    let target_framework = match parse_target_framework(&xml_content) {
        Some(it) => it,
        None => {
            return Err(format!(
                "Unable to parse target framework from xml content {}",
                xml_content
            ))
        }
    };

    let binary_name = csproj_path
        .split("/")
        .last()
        .ok_or("Could not extract binary name from path")?
        .strip_suffix(".csproj")
        .ok_or("Path does not end with .csproj")?;

    let dll_path = if let Some(last_slash) = csproj_path.rfind('/') {
        if last_slash == 0 {
            return Err("Unable to parse sdk output".into()); // Reached root
        }
        format!(
            "{}/bin/Debug/{}/{}.dll",
            &csproj_path[..last_slash],
            target_framework,
            binary_name
        )
    } else {
        return Err(format!(
            "Unable to find last slash in csproj path {}",
            csproj_path
        ));
    };

    // verify if file_path exists
    if file_exists(&dll_path)? {
        return Ok(dll_path);
    }

    return Err("Unable to find dll".into());
}

fn file_exists(path: &str) -> Result<bool, String> {
    let status = zed::Command::new("test").arg(path).output()?.status;

    return match status {
        Some(s) => Ok(s == 0),
        None => Err(format!("File not found at {}", path)),
    };
}

fn parse_target_framework(csproj_content: &str) -> Option<String> {
    // Look for <TargetFramework>net6.0</TargetFramework>
    let start = csproj_content.find("<TargetFramework>")?;
    let start = start + "<TargetFramework>".len();
    let end = csproj_content[start..].find("</TargetFramework>")?;
    Some(csproj_content[start..start + end].to_string())
}

fn env_value(env: &Vec<(String, String)>, key: &str) -> Option<String> {
    env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}
