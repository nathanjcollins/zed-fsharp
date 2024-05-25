use zed_extension_api as zed;
use zed_extension_api::serde_json;

struct FsharpExtension {}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct FsAutocompleteInitOptions {
    automatic_workspace_init: bool,
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
        let path = worktree
            .which("fsautocomplete")
            .expect("fsautocomplete not found");

        Ok(zed::Command {
            command: path,
            args: vec!["--adaptive-lsp-server-enabled".to_string()],
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
