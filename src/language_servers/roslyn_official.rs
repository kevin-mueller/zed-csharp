use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct RoslynOfficial {}

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
        // Razor support is bundled with the roslyn-language-server package and loaded automatically
        let args = vec![
            "--stdio".to_string(),
            "--autoLoadProjects".to_string(),
            "--telemetryLevel".to_string(),
            "off".to_string(),
        ];

        // Try to find roslyn-language-server in PATH
        if let Some(path) = worktree.which("roslyn-language-server") {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let _ = update_roslyn_server();

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            return Ok(zed::Command {
                command: path,
                args,
                env: Default::default(),
            });
        } else {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            download_roslyn_server()?;

            // check again
            if let Some(path) = worktree.which("roslyn-language-server") {
                return Ok(zed::Command {
                    command: path,
                    args,
                    env: Default::default(),
                });
            }
        }

        Err(format!(
            "roslyn-language-server not found or unable to install. Please try installing it manually using: 'dotnet tool install --global roslyn-language-server --prerelease'"
        ))
    }

    pub fn configuration_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings);

        // Always send the defaults, even when the user has no settings configured.
        Ok(Some(Self::transform_settings_for_roslyn(
            settings.unwrap_or_default(),
        )))
    }

    fn transform_settings_for_roslyn(settings: zed::serde_json::Value) -> zed::serde_json::Value {
        let mut roslyn_config = zed::serde_json::json!({
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
            // The test code lenses use the client-side "dotnet.test.run" command, which only exists in VS Code.
            "csharp|code_lens.dotnet_enable_tests_code_lens": false,
            // Generate auto properties instead of throwing ones (e.g. for "Implement interface").
            "csharp|type_members.dotnet_property_generation_behavior": "preferAutoProperties",
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

fn download_roslyn_server() -> Result<(), String> {
    zed_extension_api::process::Command::new("dotnet")
        .arg("tool")
        .arg("install")
        .arg("--global")
        .arg("roslyn-language-server")
        .arg("--prerelease")
        .arg("--source")
        .arg("https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json")
        .output()?;
    Ok(())
}

fn update_roslyn_server() -> Result<(), String> {
    zed_extension_api::process::Command::new("dotnet")
        .arg("tool")
        .arg("update")
        .arg("roslyn-language-server")
        .arg("--global")
        .arg("--prerelease")
        .arg("--source")
        .arg("https://pkgs.dev.azure.com/azure-public/vside/_packaging/vs-impl/nuget/v3/index.json")
        .output()?;
    Ok(())
}
