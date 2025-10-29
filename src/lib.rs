use std::path::PathBuf;
use zed_extension_api::{
    self as zed,
    serde_json::{self, Map, Value},
    settings::LspSettings,
    LanguageServerInstallationStatus,
};

mod fsac;
use fsac::acquire_fsac;

struct FsharpExtension {}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct FsAutocompleteInitOptions {
    automatic_workspace_init: bool,
}

fn get_fsac_path(
    settings_object: Option<&Map<String, Value>>,
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
) -> zed::Result<PathBuf> {
    if let Some(custom_path) = settings_object
        .and_then(|s| s.get("fsac_custom_path"))
        .and_then(|v| v.as_str())
    {
        Ok(PathBuf::from(custom_path))
    } else {
        match acquire_fsac(language_server_id, worktree) {
            Ok(fsac_path) => Ok(fsac_path),
            Err(e) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &LanguageServerInstallationStatus::Failed(e.clone()),
                );
                Err(e)
            }
        }
    }
}

fn get_final_args(fsac_path: PathBuf, settings_object: Option<&Map<String, Value>>) -> Vec<String> {
    let mut custom_args = if let Some(args) = settings_object
        .and_then(|s| s.get("fsac_custom_args"))
        .and_then(|v| v.as_array())
    {
        args.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    } else {
        Vec::new()
    };

    let mut final_args = vec![fsac_path.to_string_lossy().to_string()];
    final_args.append(&mut custom_args);
    final_args.push("--adaptive-lsp-server-enabled".to_string());
    final_args
}

impl zed::Extension for FsharpExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let dotnet_path = match worktree.which("dotnet") {
            Some(p) => p,
            None => {
                let error_msg = "dotnet executable not found in PATH".to_string();
                zed::set_language_server_installation_status(
                    language_server_id,
                    &LanguageServerInstallationStatus::Failed(error_msg.clone()),
                );
                return Err(error_msg);
            }
        };

        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?.settings;
        let settings_object = settings.as_ref().and_then(|v| v.as_object());

        let fsac_path = get_fsac_path(settings_object, worktree, language_server_id)?;
        let final_args = get_final_args(fsac_path, settings_object);

        Ok(zed::Command {
            command: dotnet_path,
            args: final_args,
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<Option<zed::serde_json::Value>> {
        let initialization_options = FsAutocompleteInitOptions {
            automatic_workspace_init: true,
        };

        Ok(Some(serde_json::json!(initialization_options)))
    }
}

zed::register_extension!(FsharpExtension);
