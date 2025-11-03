use std::path::PathBuf;
use zed_extension_api::{
    self as zed,
    serde_json::{self, Map, Value},
    settings::LspSettings,
    LanguageServerInstallationStatus,
};

mod fsac;
use fsac::{acquire_fsac, FsacAcquisition};

struct FsharpExtension {}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct FsAutocompleteInitOptions {
    automatic_workspace_init: bool,
}

fn get_custom_args(settings_object: Option<&Map<String, Value>>) -> Vec<String> {
    if let Some(args) = settings_object
        .and_then(|s| s.get("fsac_custom_args"))
        .and_then(|v| v.as_array())
    {
        args.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    } else {
        Vec::new()
    }
}

fn get_fsac_acquisition(
    settings_object: Option<&Map<String, Value>>,
    worktree: &zed::Worktree,
    language_server_id: &zed::LanguageServerId,
    custom_args: &Vec<String>,
) -> zed::Result<FsacAcquisition> {
    if let Some(custom_path) = settings_object
        .and_then(|s| s.get("fsac_custom_path"))
        .and_then(|v| v.as_str())
    {
        Ok(FsacAcquisition {
            fsac_path: PathBuf::from(custom_path),
            env: Default::default(),
        })
    } else {
        match acquire_fsac(language_server_id, worktree, custom_args) {
            Ok(acquisition) => Ok(acquisition),
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

fn get_final_args(fsac_path: PathBuf, custom_args: &[String]) -> Vec<String> {
    let mut final_args = vec![fsac_path.to_string_lossy().to_string()];
    final_args.extend_from_slice(custom_args);
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

        let custom_args = get_custom_args(settings_object);
        let acquisition = get_fsac_acquisition(
            settings_object,
            worktree,
            language_server_id,
            &custom_args,
        )?;

        let final_args = get_final_args(acquisition.fsac_path, &custom_args);

        Ok(zed::Command {
            command: dotnet_path,
            args: final_args,
            env: acquisition.env,
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
