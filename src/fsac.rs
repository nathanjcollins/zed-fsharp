use std::path::PathBuf;

use semver::Version;
use zed_extension_api::http_client::{HttpMethod, HttpRequest};
use zed_extension_api::{
    self as zed, serde_json, DownloadedFileType, EnvVars, LanguageServerInstallationStatus, Os,
    Worktree,
};

pub struct FsacAcquisition {
    pub fsac_path: PathBuf,
    pub env: EnvVars,
}

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

pub fn acquire_fsac(
    language_server_id: &zed::LanguageServerId,
    worktree: &Worktree,
    custom_args: &Vec<String>,
) -> zed::Result<FsacAcquisition> {
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
        println!(
            "fsautocomplete version {} not found locally. Downloading...",
            &last_version
        );
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
    zed::set_language_server_installation_status(
        language_server_id,
        &LanguageServerInstallationStatus::None,
    );

    let extension_home =
        std::env::current_dir().map_err(|err| format!("could not get current dir: {err}"))?;

    let dotnet_version = get_current_dotnet_version()?;

    let available_tmfs = get_fsac_tmfs_path(&last_version)?;

    let selected_tmf = select_compatible_tmfs(&dotnet_version, &available_tmfs)?;

    let sdk_tfm = format!("net{}.0", &dotnet_version.major);

    let has_user_roll_forward = custom_args.iter().any(|a| a == "--roll-forward");
    let has_user_fx_version = custom_args.iter().any(|a| a == "--fx-version");

    let should_apply_implicit_roll_forward =
        !has_user_fx_version && !has_user_roll_forward && sdk_tfm != selected_tmf;

    println!("Determined SDK TFM: {}", &sdk_tfm);
    println!("Selected FSAC TFM: {}", &selected_tmf);
    if should_apply_implicit_roll_forward {
        println!("SDK version is higher than FSAC version, and no roll-forward policy is set. Applying implicit roll-forward.");
    }

    let mut env_variables = worktree.shell_env();
    if should_apply_implicit_roll_forward {
        env_variables.push(("DOTNET_ROLL_FORWARD".to_string(), "LatestMajor".to_string()));
    }

    if !dotnet_version.pre.is_empty() {
        env_variables.push((
            "DOTNET_ROLL_FORWARD_TO_PRERELEASE".to_string(),
            "1".to_string(),
        ));
        println!("SDK is a pre-release, setting DOTNET_ROLL_FORWARD_TO_PRERELEASE=1");
    }

    let fsautocomplete_path = PathBuf::from(format!(
        // The dotnet command has the permissions to read the full path
        "{}/fsautocomplete_{}/tools/{}/any/fsautocomplete.dll",
        &extension_home.to_string_lossy(),
        &last_version,
        &selected_tmf,
    ));

    Ok(FsacAcquisition {
        fsac_path: fsautocomplete_path,
        env: env_variables,
    })
}
