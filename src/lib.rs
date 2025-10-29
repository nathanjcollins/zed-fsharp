use std::path::PathBuf;

use zed_extension_api::http_client::{HttpMethod, HttpRequest};
use zed_extension_api::{self as zed, EnvVars, Os};
use zed_extension_api::{serde_json, DownloadedFileType};

#[derive(serde::Deserialize)]
struct PackageVersionList {
    versions: Vec<String>,
}

fn fetch_fsautocomplete_versions() -> zed::Result<PackageVersionList> {
    let url = "https://api.nuget.org/v3-flatcontainer/fsautocomplete/index.json";
    let req = HttpRequest::builder()
        .url(url)
        .method(HttpMethod::Get)
        .build()
        .map_err(|_| "Failed to build request")?;

    let response = zed::http_client::fetch(&req)?;

    let package_versions: PackageVersionList =
        serde_json::from_slice(&response.body).map_err(|e| {
            let body = &str::from_utf8(&response.body).unwrap_or("");
            format!(
                "Error: {}\nFailed to parse fsautocomplete versions JSON: {}",
                e.to_string(),
                body
            )
        })?;

    Ok(package_versions)
}

fn download_fsautocomplete_version(version: &String) -> zed::Result<(), String> {
    let download_url = format!(
        "https://api.nuget.org/v3-flatcontainer/fsautocomplete/{}/fsautocomplete.{}.nupkg",
        &version, &version
    );
    let file_type = DownloadedFileType::Zip;
    let file_path = format!("fsautocomplete_{}", &version);

    zed::download_file(&download_url, &file_path, file_type)
}



struct FsharpExtension {}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct FsAutocompleteInitOptions {
    automatic_workspace_init: bool,
}

fn get_extension_home(os: &Os, env_vars: &EnvVars) -> Option<PathBuf> {
    match os {
        Os::Windows => env_vars
            .iter()
            .find(|(key, _)| key == "LOCALAPPDATA")
            .map(|(_, value)| PathBuf::from(format!("{}\\Zed\\extensions\\work\\fsharp", value))),
        Os::Mac => env_vars
            .iter()
            .find(|(key, _)| key == "HOME")
            .map(|(_, value)| {
                PathBuf::from(format!(
                    "{}/Library/Application Support/Zed/extensions/work/fsharp",
                    value
                ))
            }),
        Os::Linux => env_vars
            .iter()
            .find(|(key, _)| key == "XDG_DATA_HOME")
            .map(|(_, value)| PathBuf::from(format!("{}/Zed/extensions/work/fsharp", value))),
    }
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
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let (os, _) = zed::current_platform();
        let extension_home = match get_extension_home(&os, &worktree.shell_env()) {
            Some(path) => Ok(path),
            None => zed::Result::Err(format!("Failed to get extension work directory")),
        }?;
        let last_version = fetch_fsautocomplete_versions()?
            .versions
            .last()
            .cloned()
            .ok_or("No Versions Found")?;

        download_fsautocomplete_version(&last_version)
            .map_err(|e| format!("Failed to download fsautocomplete: {}", e))?;

        let dotnet = worktree
            .which("dotnet")
            .expect("dotnet executable not found in PATH");

        let fsautocomplete_path = PathBuf::from(format!(
            "{}/fsautocomplete_{}/tools/net8.0/any/fsautocomplete.dll",
            extension_home.to_string_lossy(),
            &last_version
        ));
        let args = std::string::String::from(fsautocomplete_path.to_string_lossy());

        Ok(zed::Command {
            command: dotnet,
            args: vec![args, "--adaptive-lsp-server-enabled".to_string()],
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
