use std::path::PathBuf;

use semver::Version;
use zed_extension_api::http_client::{HttpMethod, HttpRequest};
use zed_extension_api::{self as zed, EnvVars, Os, Worktree, LanguageServerInstallationStatus,serde_json, DownloadedFileType};

#[derive(serde::Deserialize)]
struct PackageVersionList {
    versions: Vec<String>,
}

fn fetch_fsautocomplete_versions() -> zed::Result<PackageVersionList> {
    println!("Fetching fsautocomplete versions...");
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

    println!(
        "Fetched {} versions of fsautocomplete.",
        package_versions.versions.len()
    );
    Ok(package_versions)
}

fn download_fsautocomplete_version(version: &String) -> zed::Result<(), String> {
    println!("Downloading fsautocomplete version {}...", version);
    let download_url = format!(
        "https://api.nuget.org/v3-flatcontainer/fsautocomplete/{}/fsautocomplete.{}.nupkg",
        &version, &version
    );
    let file_type = DownloadedFileType::Zip;
    let file_path = format!("fsautocomplete_{}", &version);

    zed::download_file(&download_url, &file_path, file_type)
}

fn get_extension_home(os: &Os, env_vars: &EnvVars) -> Option<PathBuf> {
    match os {
        Os::Windows => env_vars
            .iter()
            .find(|(key, _)| key == "LOCALAPPDATA")
            .map(|(_, value)| PathBuf::from(format!("{}/Zed/extensions/work/fsharp", value))),
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

fn get_current_dotnet_version() -> zed::Result<Version> {
    let output = zed::Command {
        command: "dotnet".to_string(),
        args: vec!["--version".to_string()],
        env: Default::default(),
    }
    .output()
    .map_err(|e| format!("Failed to execute dotnet command: {}", e))?;

    match output.status {
        Some(0) => {
            let version = String::from_utf8_lossy(&output.stdout);
            match Version::parse(version.trim()) {
                Ok(ver) => {
                    println!("Detected dotnet version: {}", ver);
                    Ok(ver)
                }
                Err(e) => zed::Result::Err(format!("Failed to parse dotnet version: {}", e)),
            }
        }
        _ => {
            let error_message = String::from_utf8_lossy(&output.stderr);
            zed::Result::Err(format!("Failed to get dotnet version: {}", error_message))
        }
    }
}

fn get_fsac_tmfs_path(version: &String) -> zed::Result<Vec<Version>> {
    // The extension is not allowed to read from arbitrary paths.
    // apparently, relative paths are from the extension home, not the worktree.
    // so this is equivalent to reading
    // <local_app_data>/Zed/extensions/work/fsharp/fsautocomplete_{version}/tools
    let fsac_tools = PathBuf::from(format!("fsautocomplete_{}/tools", version));
    let mut versions = Vec::new();
    let read_dir = std::fs::read_dir(fsac_tools)
        .map_err(|e| format!("Failed to read fsautocomplete tools directory: {}", e))?;
    for entry_res in read_dir {
        let entry = entry_res
            .map_err(|e| format!("Failed to read fsautocomplete tools directory: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|os| os.to_str()) {
                if dir_name.starts_with("net") && dir_name.len() > 3 {
                    let rest = format!("{}.0", &dir_name[3..]);
                    let parsed_version =
                        Version::parse(&rest).map_err(|e| format!("{}", e.to_string()))?;
                    versions.push(parsed_version);
                }
            }
        }
    }

    Ok(versions)
}

fn select_compatible_tmfs(
    dotnet_version: &Version,
    available_versions: &Vec<Version>,
) -> zed::Result<String> {
    let mut compatible_versions: Vec<&Version> = available_versions
        .iter()
        .filter(|v| v.major <= dotnet_version.major)
        .collect();

    compatible_versions.sort();

    if let Some(selected_version) = compatible_versions.last() {
        println!(
            "Selected Target Framework Moniker: net{}.0",
            selected_version.major
        );
        Ok(format!("net{}.0", selected_version.major))
    } else {
        zed::Result::Err(format!(
            "No compatible Target Framework Monikers found for dotnet version {}",
            dotnet_version
        ))
    }
}

pub fn acquire_fsac(language_server_id: &zed::LanguageServerId, worktree: &Worktree) -> zed::Result<PathBuf> {
    let (os, _) = zed::current_platform();

    zed::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::CheckingForUpdate,
    );
    let last_version = fetch_fsautocomplete_versions()?
        .versions
        .last()
        .cloned()
        .ok_or("No Versions Found")?;
    if !PathBuf::from(format!("fsautocomplete_{}", &last_version)).exists() {
        println!("fsautocomplete version {} not found locally. Downloading...", &last_version);
        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::Downloading,
        );
        download_fsautocomplete_version(&last_version)
            .map_err(|e| format!("Failed to download fsautocomplete: {}", e))?;
    } else {
        println!(
            "fsautocomplete version {} found locally. Skipping download.",
            &last_version
        );
    }
    zed::set_language_server_installation_status(language_server_id, &LanguageServerInstallationStatus::None);

    let extension_home = get_extension_home(&os, &worktree.shell_env())
        .ok_or("Failed to determine extension home")?;

    let dotnet_version = get_current_dotnet_version()?;

    let available_tmfs = get_fsac_tmfs_path(&last_version)?;

    let selected_tmf = select_compatible_tmfs(&dotnet_version, &available_tmfs)?;

    let fsautocomplete_path = PathBuf::from(format!(
        // The dotnet command has the permissions to read the full path
        "{}/fsautocomplete_{}/tools/{}/any/fsautocomplete.dll",
        &extension_home.to_string_lossy(),
        &last_version,
        &selected_tmf,
    ));

    Ok(fsautocomplete_path)
}
