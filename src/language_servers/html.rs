use std::{env, fs};

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

pub struct HtmlLanguageServer {
    cached_binary_path: Option<String>,
}

const BINARY_NAME: &str = "vscode-html-language-server";
const SERVER_PATH: &str =
    "node_modules/@zed-industries/vscode-langservers-extracted/bin/vscode-html-language-server";
const PACKAGE_NAME: &str = "@zed-industries/vscode-langservers-extracted";

impl HtmlLanguageServer {
    pub fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn server_exists(&self) -> bool {
        fs::metadata(SERVER_PATH).is_ok_and(|stat| stat.is_file())
    }

    fn server_script_path(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        let server_exists = self.server_exists();
        if self.cached_binary_path.is_some() && server_exists {
            return Ok(SERVER_PATH.to_string());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let version = zed::npm_package_latest_version(PACKAGE_NAME)?;

        if !server_exists
            || zed::npm_package_installed_version(PACKAGE_NAME)?.as_ref() != Some(&version)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let result = zed::npm_install_package(PACKAGE_NAME, &version);
            match result {
                Ok(()) => {
                    if !self.server_exists() {
                        Err(format!(
                                "installed package '{PACKAGE_NAME}' did not contain expected path '{SERVER_PATH}'",
                            ))?;
                    }
                }
                Err(error) => {
                    if !self.server_exists() {
                        Err(error)?;
                    }
                }
            }
        }
        Ok(SERVER_PATH.to_string())
    }

    pub const LANGUAGE_SERVER_ID: &'static str = "html-in-razor";

    pub fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let server_path = if let Some(path) = worktree.which(BINARY_NAME) {
            return Ok(zed::Command {
                command: path,
                args: vec!["--stdio".to_string()],
                env: Default::default(),
            });
        } else {
            let server_path = self.server_script_path(language_server_id)?;
            env::current_dir()
                .unwrap()
                .join(&server_path)
                .to_string_lossy()
                .to_string()
        };
        self.cached_binary_path = Some(server_path.clone());

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec![server_path, "--stdio".to_string()],
            env: Default::default(),
        })
    }

    pub fn initialization_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        // Without provideFormatter the server does not advertise document formatting,
        // so Zed would never send format requests to it.
        let initialization_options = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|s| s.initialization_options)
            .unwrap_or_else(|| zed::serde_json::json!({ "provideFormatter": true }));

        Ok(Some(initialization_options))
    }

    pub fn configuration_options(
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(Self::LANGUAGE_SERVER_ID, worktree)
            .ok()
            .and_then(|s| s.settings)
            .unwrap_or_default();

        Ok(Some(settings))
    }
}
