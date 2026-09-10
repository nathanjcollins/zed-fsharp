use std::path::PathBuf;
use zed_extension_api::{
    self as zed,
    serde_json::{self, Map, Value},
    settings::LspSettings,
    DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DebugTaskDefinition,
    LanguageServerInstallationStatus, StartDebuggingRequestArgumentsRequest, TaskTemplate,
    Worktree,
};

mod dotnet_locator;
mod fsac;
mod netcoredbg;
use fsac::{acquire_fsac, FsacAcquisition};

struct FsharpExtension {
    cached_netcoredbg_path: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct FsAutocompleteInitOptions {
    automatic_workspace_init: bool,
    tooltip_show_documentation_link: bool,
    unused_opens_analyzer: bool,
    unused_declarations_analyzer: bool,
    add_private_access_modifier: bool,
    external_autocomplete: bool,
    interface_stub_generation: bool,
    abstract_class_stub_generation: bool,
    union_case_stub_generation: bool,
    record_stub_generation: bool,
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
    for arg in custom_args
        .iter()
        .cloned()
        .chain(std::iter::once("--adaptive-lsp-server-enabled".to_string()))
    {
        if !final_args.contains(&arg) {
            final_args.push(arg);
        }
    }
    final_args
}

impl zed::Extension for FsharpExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            cached_netcoredbg_path: None,
        }
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
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<Option<zed::serde_json::Value>> {
        let initialization_options = FsAutocompleteInitOptions {
            automatic_workspace_init: true,
            // Zed does not support info panel so documentation links are not shown
            tooltip_show_documentation_link: false,
            unused_opens_analyzer: true,
            unused_declarations_analyzer: true,
            add_private_access_modifier: true,
            external_autocomplete: false,
            interface_stub_generation: true,
            abstract_class_stub_generation: true,
            union_case_stub_generation: true,
            record_stub_generation: true,
        };

        // Defaults deep-merged with the user's lsp.fsautocomplete.initialization_options —
        // user keys win per key, so setting one option doesn't wipe the defaults.
        let mut options = serde_json::json!(initialization_options);
        if let Some(user_options) = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.initialization_options)
        {
            merge(&mut options, user_options);
        }

        Ok(Some(options))
    }

    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &Worktree,
    ) -> zed::Result<DebugAdapterBinary> {
        netcoredbg::get_dap_binary(
            &mut self.cached_netcoredbg_path,
            &adapter_name,
            config,
            user_provided_debug_adapter_path,
            worktree,
        )
    }

    fn dap_request_kind(
        &mut self,
        adapter_name: String,
        config: Value,
    ) -> zed::Result<StartDebuggingRequestArgumentsRequest> {
        netcoredbg::request_kind(&adapter_name, &config)
    }

    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> zed::Result<DebugScenario> {
        netcoredbg::config_to_scenario(config)
    }

    fn dap_locator_create_scenario(
        &mut self,
        locator_name: String,
        build_task: TaskTemplate,
        resolved_label: String,
        debug_adapter_name: String,
    ) -> Option<DebugScenario> {
        if locator_name != dotnet_locator::LOCATOR_NAME {
            return None;
        }
        dotnet_locator::create_scenario(build_task, &resolved_label, &debug_adapter_name)
    }

    fn run_dap_locator(
        &mut self,
        locator_name: String,
        build_task: TaskTemplate,
    ) -> zed::Result<DebugRequest> {
        if locator_name != dotnet_locator::LOCATOR_NAME {
            return Err(format!("Unknown debug locator: {locator_name}"));
        }
        dotnet_locator::run_locator(build_task)
    }
}

/// Recursively overlay `overlay` onto `base`; objects merge per key,
/// everything else is replaced by the overlay value.
fn merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                merge(base_map.entry(key).or_insert(Value::Null), value);
            }
        }
        (base_slot, overlay) => *base_slot = overlay,
    }
}

zed::register_extension!(FsharpExtension);
