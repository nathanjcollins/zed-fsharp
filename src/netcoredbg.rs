use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zed_extension_api::{
    self as zed,
    serde_json::{self, Value},
    Architecture, DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario,
    DebugTaskDefinition, DownloadedFileType, GithubReleaseOptions, Os,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, Worktree,
};

pub const ADAPTER_NAME: &str = "netcoredbg";

/// Launch/attach configuration understood by netcoredbg's VS Code interpreter.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetcoredbgConfig {
    request: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    program: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stop_at_entry: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    process_id: Option<u32>,
}

fn request_kind_of(config: &Value) -> zed::Result<StartDebuggingRequestArgumentsRequest> {
    match config.get("request").and_then(|v| v.as_str()) {
        Some("launch") => Ok(StartDebuggingRequestArgumentsRequest::Launch),
        Some("attach") => Ok(StartDebuggingRequestArgumentsRequest::Attach),
        Some(other) => Err(format!(
            "Invalid 'request' value: '{}'. Expected 'launch' or 'attach'",
            other
        )),
        None => Err("Debug configuration is missing the 'request' field ('launch' or 'attach')"
            .to_string()),
    }
}

pub fn request_kind(
    adapter_name: &str,
    config: &Value,
) -> zed::Result<StartDebuggingRequestArgumentsRequest> {
    if adapter_name != ADAPTER_NAME {
        return Err(format!("Unknown debug adapter: {adapter_name}"));
    }
    request_kind_of(config)
}

pub fn get_dap_binary(
    cached_path: &mut Option<String>,
    adapter_name: &str,
    config: DebugTaskDefinition,
    user_provided_debug_adapter_path: Option<String>,
    worktree: &Worktree,
) -> zed::Result<DebugAdapterBinary> {
    if adapter_name != ADAPTER_NAME {
        return Err(format!("Unknown debug adapter: {adapter_name}"));
    }

    // The config string is passed through to netcoredbg untouched; it is only
    // parsed here to determine the request kind and the adapter's working directory.
    let parsed_config: Value = serde_json::from_str(&config.config)
        .map_err(|e| format!("Failed to parse debug configuration: {}", e))?;
    let request = request_kind_of(&parsed_config)?;
    let cwd = parsed_config
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| worktree.root_path());

    let netcoredbg_path = match user_provided_debug_adapter_path {
        Some(user_path) => user_path,
        None => match cached_path
            .clone()
            .filter(|path| PathBuf::from(path).exists())
        {
            Some(path) => path,
            None => {
                let path = acquire_netcoredbg()?;
                *cached_path = Some(path.clone());
                path
            }
        },
    };

    Ok(DebugAdapterBinary {
        command: Some(netcoredbg_path),
        arguments: vec!["--interpreter=vscode".to_string()],
        // The debuggee's environment comes from the launch configuration; this is
        // the adapter's own environment, which needs `dotnet` on PATH.
        envs: worktree.shell_env(),
        cwd: Some(cwd),
        connection: None,
        request_args: StartDebuggingRequestArguments {
            configuration: config.config,
            request,
        },
    })
}

pub fn config_to_scenario(config: DebugConfig) -> zed::Result<DebugScenario> {
    let adapter_config = match config.request {
        DebugRequest::Launch(launch) => NetcoredbgConfig {
            request: "launch".to_string(),
            program: Some(launch.program),
            args: launch.args,
            cwd: launch.cwd,
            env: launch.envs.into_iter().collect(),
            stop_at_entry: config.stop_on_entry,
            process_id: None,
        },
        DebugRequest::Attach(attach) => NetcoredbgConfig {
            request: "attach".to_string(),
            program: None,
            args: Vec::new(),
            cwd: None,
            env: BTreeMap::new(),
            stop_at_entry: config.stop_on_entry,
            process_id: Some(attach.process_id.ok_or_else(|| {
                "Attaching requires a process id. Please select a process to attach to."
                    .to_string()
            })?),
        },
    };

    Ok(DebugScenario {
        label: config.label,
        adapter: config.adapter,
        build: None,
        config: serde_json::to_string(&adapter_config)
            .map_err(|e| format!("Failed to serialize debug configuration: {}", e))?,
        tcp_connection: None,
    })
}

fn executable_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "netcoredbg.exe",
        _ => "netcoredbg",
    }
}

/// Release asset name prefix for the current platform, e.g. "netcoredbg-linux-amd64".
/// The archive extension is intentionally not part of the prefix: upstream has shipped
/// both .tar.gz and .zip for the same platform across releases.
fn platform_asset_prefix(os: Os, arch: Architecture) -> zed::Result<&'static str> {
    match (os, arch) {
        (Os::Linux, Architecture::X8664) => Ok("netcoredbg-linux-amd64"),
        (Os::Linux, Architecture::Aarch64) => Ok("netcoredbg-linux-arm64"),
        (Os::Mac, Architecture::X8664) => Ok("netcoredbg-osx-amd64"),
        (Os::Mac, Architecture::Aarch64) => Ok("netcoredbg-osx-arm64"),
        // netcoredbg has no Windows arm64 builds; the x64 build runs under emulation
        (Os::Windows, _) => Ok("netcoredbg-win64"),
        _ => Err("netcoredbg does not provide builds for this platform".to_string()),
    }
}

/// The archives don't have a stable internal layout (the binary may sit behind a
/// top-level `netcoredbg/` directory), so search for it instead of assuming a path.
fn find_executable(dir: &Path, executable_name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().is_some_and(|name| name == executable_name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_executable(&path, executable_name) {
                return Some(found);
            }
        }
    }
    None
}

fn download_netcoredbg(
    release: &zed::GithubRelease,
    asset_prefix: &str,
    version_dir: &Path,
) -> zed::Result<()> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name.starts_with(asset_prefix))
        .ok_or_else(|| {
            format!(
                "No netcoredbg {} release asset for this platform (looked for '{}*'). Available assets: [{}]. \
                You can install netcoredbg yourself and point Zed at it via the `dap.netcoredbg.binary` setting.",
                release.version,
                asset_prefix,
                release
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    let file_type = if asset.name.ends_with(".tar.gz") {
        DownloadedFileType::GzipTar
    } else if asset.name.ends_with(".zip") {
        DownloadedFileType::Zip
    } else {
        return Err(format!("Unsupported archive type for asset: {}", asset.name));
    };

    println!(
        "Downloading netcoredbg {} ({})...",
        release.version, asset.name
    );
    zed::download_file(
        &asset.download_url,
        &version_dir.to_string_lossy(),
        file_type,
    )
    .map_err(|e| format!("Failed to download netcoredbg: {}", e))
}

/// Best-effort removal of older downloaded versions once a new one is verified.
/// Failures are logged and ignored: on Windows the old binary may still be
/// locked by a running debug session, and keeping it around is harmless.
fn remove_stale_versions(current_version_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(".") else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("netcoredbg_") || path.file_name() == current_version_dir.file_name()
        {
            continue;
        }
        println!("Removing stale {}...", name);
        if let Err(e) = std::fs::remove_dir_all(&path) {
            println!("Failed to remove stale {}: {}", name, e);
        }
    }
}

/// Best-effort fallback when the GitHub release check fails (e.g. offline or
/// rate-limited): use the newest previously downloaded version, if any.
fn find_newest_local_netcoredbg(executable_name: &str) -> Option<PathBuf> {
    let mut versions: Vec<(semver::Version, PathBuf)> = std::fs::read_dir(".")
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            let version = semver::Version::parse(name.strip_prefix("netcoredbg_")?).ok()?;
            Some((version, path))
        })
        .collect();
    versions.sort();
    versions.into_iter().rev().find_map(|(version, dir)| {
        let path = find_executable(&dir, executable_name)?;
        println!("Using previously downloaded netcoredbg {}", version);
        Some(path)
    })
}

/// Returns the absolute path to a netcoredbg executable, downloading the latest
/// GitHub release into the extension work directory if it isn't present yet.
/// When the release check fails, falls back to a previously downloaded version.
fn acquire_netcoredbg() -> zed::Result<String> {
    let (os, arch) = zed::current_platform();
    let asset_prefix = platform_asset_prefix(os, arch)?;
    let executable_name = executable_name(os);

    let executable_path = match zed::latest_github_release(
        "Samsung/netcoredbg",
        GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    ) {
        Ok(release) => {
            let version_dir = PathBuf::from(format!("netcoredbg_{}", release.version));

            match find_executable(&version_dir, executable_name) {
                Some(path) => {
                    println!(
                        "netcoredbg {} found locally. Skipping download.",
                        release.version
                    );
                    path
                }
                None => {
                    download_netcoredbg(&release, asset_prefix, &version_dir)?;
                    let path = find_executable(&version_dir, executable_name).ok_or_else(|| {
                        format!(
                            "netcoredbg executable not found in downloaded archive at {}",
                            version_dir.display()
                        )
                    })?;
                    // Only prune once the new version is verified, so a failed
                    // download never leaves us with no local version at all.
                    remove_stale_versions(&version_dir);
                    path
                }
            }
        }
        Err(fetch_error) => find_newest_local_netcoredbg(executable_name).ok_or_else(|| {
            format!(
                "Failed to fetch the latest netcoredbg release ({}) and no previously \
                downloaded version was found.",
                fetch_error
            )
        })?,
    };

    zed::make_file_executable(&executable_path.to_string_lossy())
        .map_err(|e| format!("Failed to make netcoredbg executable: {}", e))?;

    let extension_home =
        std::env::current_dir().map_err(|err| format!("could not get current dir: {err}"))?;

    Ok(extension_home
        .join(executable_path)
        .to_string_lossy()
        .to_string())
}
