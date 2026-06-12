use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct CsharpLs;

impl CsharpLs {
    pub const LANGUAGE_SERVER_ID: &'static str = "csharp-ls";

    pub fn new() -> Self {
        CsharpLs
    }

    pub fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if let Some(path) = worktree.which("csharp-ls") {
            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let _ = update_csharp_ls();

            zed_extension_api::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::None,
            );

            return Ok(zed::Command {
                command: path,
                args: vec![],
                env: Default::default(),
            });
        }

        zed_extension_api::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        zed_extension_api::process::Command::new("dotnet")
            .arg("tool")
            .arg("install")
            .arg("--global")
            .arg("csharp-ls")
            .output()?;

        zed_extension_api::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        if let Some(path) = worktree.which("csharp-ls") {
            return Ok(zed::Command {
                command: path,
                args: vec![],
                env: Default::default(),
            });
        }

        Err(
            "csharp-ls not found. Install manually: dotnet tool install --global csharp-ls"
                .to_string(),
        )
    }

    pub fn configuration_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let user_settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|s| s.settings)
            .unwrap_or_default();

        let mut config = zed::serde_json::json!({
            "csharp": {
                "analyzersEnabled": true,
                "useMetadataUris": true,
            }
        });

        if let (Some(config_map), zed::serde_json::Value::Object(user_map)) =
            (config["csharp"].as_object_mut(), user_settings)
        {
            for (key, value) in user_map {
                config_map.insert(key, value);
            }
        }

        Ok(Some(config))
    }
}

fn update_csharp_ls() -> Result<(), String> {
    zed_extension_api::process::Command::new("dotnet")
        .arg("tool")
        .arg("update")
        .arg("--global")
        .arg("csharp-ls")
        .output()?;
    Ok(())
}
