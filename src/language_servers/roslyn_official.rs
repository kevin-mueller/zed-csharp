use std::sync::OnceLock;
use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct RoslynOfficial {}

static DOTNET_ROOT_CACHE: OnceLock<String> = OnceLock::new();
type EnvVars = Vec<(String, String)>;

fn with_optional_env(
    command: zed_extension_api::process::Command,
    dotnet_env: Option<&EnvVars>,
) -> zed_extension_api::process::Command {
    match dotnet_env {
        Some(env_vars) => command.envs(env_vars.clone()),
        None => command,
    }
}

impl RoslynOfficial {
    pub const LANGUAGE_SERVER_ID: &'static str = "roslyn-official";

    pub fn new() -> Self {
        RoslynOfficial {}
    }

    pub fn language_server_cmd(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let Some(lsp_settings) = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree).ok()
        else {
            return Err(format!("Unable to load settings"));
        };
        let use_worktree_dotnet_env = Self::use_worktree_dotnet_env(&lsp_settings);
        let dotnet_env = if use_worktree_dotnet_env {
            Some(worktree.shell_env())
        } else {
            None
        };

        // Build arguments list
        let base_args = vec!["--stdio".to_string(), "--autoLoadProjects".to_string()];

        let razor_args =
            Self::install_or_update_razor(lsp_settings, language_server_id, dotnet_env.as_ref())?;

        let final_args = match razor_args {
            Some(x) => base_args.into_iter().chain(x).collect(),
            None => base_args,
        };

        // Resolve DOTNET_ROOT from the SDK path so the LSP process finds
        // the correct .NET runtime, even if the worktree pins an older SDK.
        let env = if let Some(root) = Self::resolve_dotnet_root(dotnet_env.as_ref()) {
            vec![("DOTNET_ROOT".to_string(), root)]
        } else {
            vec![]
        };

        // Try to find roslyn-language-server in PATH
        if let Some(path) = worktree.which("roslyn-language-server") {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );

            if let Err(error) = update_roslyn_server(dotnet_env.as_ref()) {
                println!("Unable to update roslyn-language-server: {}", error);
            }

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            return Ok(zed::Command {
                command: path,
                args: final_args,
                env: env,
            });
        } else {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            download_roslyn_server(dotnet_env.as_ref())?;

            // check again
            if let Some(path) = worktree.which("roslyn-language-server") {
                zed_extension_api::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );

                return Ok(zed::Command {
                    command: path,
                    args: final_args,
                    env: env,
                });
            }
        }

        Err(format!(
            "roslyn-language-server not found or unable to install. Please try installing it manually using: 'dotnet tool install --global roslyn-language-server --prerelease'"
        ))
    }

    fn use_worktree_dotnet_env(lsp_settings: &LspSettings) -> bool {
        lsp_settings
            .settings
            .as_ref()
            .and_then(|settings| settings.get("use_worktree_dotnet_env"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    fn resolve_dotnet_root(dotnet_env: Option<&EnvVars>) -> Option<String> {
        if dotnet_env.is_none() {
            return Self::cached_dotnet_root();
        }
        Self::find_dotnet_root(dotnet_env)
    }

    fn cached_dotnet_root() -> Option<String> {
        if let Some(root) = DOTNET_ROOT_CACHE.get() {
            return Some(root.clone());
        }

        let root = Self::find_dotnet_root(None)?;
        let _ = DOTNET_ROOT_CACHE.set(root.clone());
        Some(root)
    }

    /// Resolve DOTNET_ROOT from the active dotnet installation.
    /// Uses `dotnet --info` and derives the root from "Base Path".
    fn find_dotnet_root(dotnet_env: Option<&EnvVars>) -> Option<String> {
        let mut command = with_optional_env(
            zed_extension_api::process::Command::new("dotnet").arg("--info"),
            dotnet_env,
        );
        let output = command.output().ok()?;

        if output.status != Some(0) {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let base_path = stdout
            .lines()
            .find_map(|line| line.trim_start().strip_prefix("Base Path:"))
            .map(|value| value.trim())?;

        let root = std::path::Path::new(base_path).parent()?.parent()?;
        Some(root.to_string_lossy().to_string())
    }

    fn find_dotnet_sdk_path(dotnet_env: Option<&EnvVars>) -> Result<(String, String), String> {
        let mut sdk_list_command = with_optional_env(
            zed_extension_api::process::Command::new("dotnet").arg("--list-sdks"),
            dotnet_env,
        );
        let sdks_output = sdk_list_command.output()?;
        if sdks_output.status != Some(0) {
            return Err(format!(
                "Unable to list installed dotnet SDKs.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&sdks_output.stdout),
                String::from_utf8_lossy(&sdks_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&sdks_output.stdout);
        let installed_sdks = stdout
            .lines()
            .filter_map(|line| {
                let version = line.split_whitespace().next()?;
                let start = line.find('[')? + 1;
                let end = line.rfind(']')?;
                Some((version.to_string(), line[start..end].to_string()))
            })
            .collect::<Vec<_>>();

        if installed_sdks.is_empty() {
            return Err(format!(
                "Unable to parse installed dotnet SDKs from output: {}",
                stdout
            ));
        }

        let (sdk_version, sdk_base_path) = installed_sdks
            .last()
            .expect("installed_sdks is non-empty after explicit guard");

        let sdk_path = std::path::Path::new(&sdk_base_path)
            .join(sdk_version)
            .to_string_lossy()
            .to_string();
        Ok((sdk_path, sdk_version.clone()))
    }

    // Find the Razor Compiler DLL in the given SDK path
    fn find_razor_compiler_dll(sdk_path: &str) -> String {
        let dll_path = format!(
            "{}/Sdks/Microsoft.NET.Sdk.Razor/source-generators/Microsoft.CodeAnalysis.Razor.Compiler.dll",
            sdk_path
        );

        return dll_path;
    }

    // Find the Razor Design Time Targets file in the given SDK path
    fn find_razor_design_time_targets(sdk_path: &str) -> String {
        return format!(
            "{}/Sdks/Microsoft.NET.Sdk.Razor/targets/Microsoft.NET.Sdk.Razor.DesignTime.targets",
            sdk_path
        );
    }

    pub fn configuration_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings);

        Ok(settings.map(Self::transform_settings_for_roslyn))
    }

    fn install_or_update_razor(
        lsp_settings: LspSettings,
        language_server_id: &LanguageServerId,
        dotnet_env: Option<&EnvVars>,
    ) -> Result<Option<Vec<String>>, String> {
        let lsp_user_settings = match lsp_settings.settings {
            Some(settings) => settings,
            None => {
                return Ok(None); // no settings is also fine => no razor support
            }
        };

        let razor_root = lsp_user_settings["razor_source_repository_root"]
            .as_str()
            .map(|s| s.to_string());

        let razor_root_unwrapped = &match razor_root {
            None => return Ok(None), // no settings is also fine => no razor support
            Some(x) => x,
        };

        let (sdk_path, sdk_version) = Self::find_dotnet_sdk_path(dotnet_env)?;

        let directory_exists = zed_extension_api::Command::new("test")
            .arg("-d")
            .arg(razor_root_unwrapped)
            .output()?;

        if directory_exists.status.unwrap() == 0 {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::CheckingForUpdate,
            );
            // in this case we can reset the git repository and pull the latest changes
            let razor_root_reset = zed_extension_api::process::Command::new("git")
                .arg("-C")
                .arg(razor_root_unwrapped)
                .arg("reset")
                .arg("--hard")
                .output()?;

            if razor_root_reset.status.is_none() || razor_root_reset.status.unwrap() != 0 {
                return Err(format!(
                    "Unable to reset razor git repository. Git installed?"
                ));
            }
            let razor_root_git_pull = zed_extension_api::process::Command::new("git")
                .arg("-C")
                .arg(razor_root_unwrapped)
                .arg("pull")
                .output()?;

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            if razor_root_git_pull.status.is_none() || razor_root_git_pull.status.unwrap() != 0 {
                println!("Unable to pull latest changes in razor repository. Offline?");
            }
        } else {
            // in this case we need to clone the repository
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let razor_root_clone = zed_extension_api::process::Command::new("git")
                .arg("clone")
                .arg("https://github.com/dotnet/razor")
                .arg(razor_root_unwrapped)
                .output()?;

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            if razor_root_clone.status.is_none() || razor_root_clone.status.unwrap() != 0 {
                return Err(format!("Unable to clone razor git repository. For this initial setup step, an internet connection is required."));
            }
        }

        let mut dotnet_build_command = with_optional_env(
            zed_extension_api::process::Command::new("dotnet")
                .arg("build")
                .arg(format!(
                    "{}/src/Razor/src/Microsoft.VisualStudioCode.RazorExtension/Microsoft.VisualStudioCode.RazorExtension.csproj",
                    razor_root_unwrapped
                ))
                .arg("--configuration")
                .arg("Release"),
            dotnet_env,
        );
        let dotnet_build = dotnet_build_command.output()?;

        if dotnet_build.status.is_none() || dotnet_build.status.unwrap() != 0 {
            return Err(format!(
                "Unable to build razor extension.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&dotnet_build.stdout),
                String::from_utf8_lossy(&dotnet_build.stderr)
            ));
        }

        // if someone knows a better way to get this dll, i'm all ears
        let sdk_version_short = format!(
            "net{}",
            sdk_version.split('.').take(2).collect::<Vec<_>>().join(".")
        );
        let razor_vscode_extension_path = format!("{}/artifacts/bin/Microsoft.VisualStudioCode.RazorExtension/Release/{}/Microsoft.VisualStudioCode.RazorExtension.dll", razor_root_unwrapped, sdk_version_short);

        let mut args = vec![];

        args.push("--extension".to_string());
        args.push(razor_vscode_extension_path);

        let razor_dll = Self::find_razor_compiler_dll(&sdk_path);
        let razor_targets = Self::find_razor_design_time_targets(&sdk_path);

        // Add Razor source generator argument
        args.push("--razorSourceGenerator".to_string());
        args.push(razor_dll);

        // Add Razor design time path argument
        args.push("--razorDesignTimePath".to_string());
        args.push(razor_targets);

        return Ok(Some(args));
    }

    fn transform_settings_for_roslyn(settings: zed::serde_json::Value) -> zed::serde_json::Value {
        let mut roslyn_config = zed::serde_json::json!({
            // Enable Razor cohosting for proper Razor/Blazor support
            "razor|language_server.cohosting_enabled": true,
            // These code lenses show up as "Unknown Command" in Zed and don't do anything when clicked. Disable them by default.
            "csharp|code_lens.dotnet_enable_references_code_lens": false,
            "csharp|code_lens.dotnet_enable_tests_code_lens": false,
            // Disable code lenses for Razor files to prevent errors
            "razor|code_lens.dotnet_enable_references_code_lens": false,
            "razor|code_lens.dotnet_enable_tests_code_lens": false,
            // Enable inlay hints in the language server by default.
            // This way, enabling inlay hints in Zed will cause inlay hints to show up in C# without extra configuration.
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_literal_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_indexer_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_object_creation_parameters": true,
            "csharp|inlay_hints.dotnet_enable_inlay_hints_for_other_parameters": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_variable_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_lambda_parameter_types": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_object_creation": true,
            "csharp|inlay_hints.csharp_enable_inlay_hints_for_collection_expressions": true,
        });

        let config_map = roslyn_config.as_object_mut().unwrap();
        if let zed::serde_json::Value::Object(settings_map) = settings {
            for (key, value) in settings_map {
                config_map.insert(key.clone(), value.clone());
            }
        }

        roslyn_config
    }
}

fn download_roslyn_server(dotnet_env: Option<&EnvVars>) -> Result<(), String> {
    let mut command = with_optional_env(
        zed_extension_api::process::Command::new("dotnet")
            .arg("tool")
            .arg("install")
            .arg("--global")
            .arg("roslyn-language-server")
            .arg("--prerelease")
            .arg("--source")
            .arg("https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json"),
        dotnet_env,
    );
    let output = command.output()?;
    if output.status != Some(0) {
        return Err(format!(
            "Unable to install roslyn-language-server.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn update_roslyn_server(dotnet_env: Option<&EnvVars>) -> Result<(), String> {
    let mut command = with_optional_env(
        zed_extension_api::process::Command::new("dotnet")
            .arg("tool")
            .arg("update")
            .arg("roslyn-language-server")
            .arg("--global")
            .arg("--prerelease")
            .arg("--source")
            .arg("https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json"),
        dotnet_env,
    );
    let output = command.output()?;
    if output.status != Some(0) {
        return Err(format!(
            "Unable to update roslyn-language-server.\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
