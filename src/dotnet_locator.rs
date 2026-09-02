use zed_extension_api::{
    self as zed, serde_json, BuildTaskDefinition, BuildTaskDefinitionTemplatePayload, DebugRequest,
    DebugScenario, LaunchRequest, TaskTemplate,
};

pub const LOCATOR_NAME: &str = "dotnet";

/// Arguments destined for the debuggee (everything after `--` in `dotnet run`)
/// cannot appear on the build command line, so they ride along on the build
/// task's environment and are recovered in `run_locator`.
const DEBUG_ARGS_ENV_VAR: &str = "ZED_FSHARP_DEBUG_ARGS";

/// `dotnet run` options that must not be forwarded to `dotnet build`.
/// The bool indicates whether the option consumes a value argument.
const RUN_ONLY_OPTIONS: &[(&str, bool)] = &[
    ("--no-build", false),
    ("--no-launch-profile", false),
    ("--launch-profile", true),
    ("-e", true),
    ("--environment", true),
];

/// `dotnet build` options that consume a value argument, so the value is not
/// mistaken for the project path when parsing the build task back.
const BUILD_OPTIONS_WITH_VALUE: &[&str] = &[
    "-c",
    "--configuration",
    "-f",
    "--framework",
    "-r",
    "--runtime",
    "-a",
    "--arch",
    "--os",
    "-v",
    "--verbosity",
    "-o",
    "--output",
    "--artifacts-path",
    "-p",
    "--property",
];

/// Turns a `dotnet run` task into a debug scenario that builds the project
/// with `dotnet build` and then debugs the produced assembly.
pub fn create_scenario(
    task: TaskTemplate,
    resolved_label: &str,
    debug_adapter_name: &str,
) -> Option<DebugScenario> {
    if debug_adapter_name != crate::netcoredbg::ADAPTER_NAME {
        return None;
    }
    if task.command != "dotnet" || task.args.first().map(String::as_str) != Some("run") {
        return None;
    }

    let mut build_args = vec!["build".to_string()];
    let mut app_args = Vec::new();
    let mut rest = task.args[1..].iter().cloned();
    'args: while let Some(arg) = rest.next() {
        if arg == "--" {
            app_args.extend(rest);
            break;
        }
        // `dotnet build` takes the project as a positional argument instead
        if arg == "--project" {
            build_args.extend(rest.next());
            continue;
        }
        if let Some(project) = arg.strip_prefix("--project=") {
            build_args.push(project.to_string());
            continue;
        }
        for (option, has_value) in RUN_ONLY_OPTIONS {
            if arg == *option {
                if *has_value {
                    rest.next();
                }
                continue 'args;
            }
            if *has_value && arg.starts_with(&format!("{option}=")) {
                continue 'args;
            }
        }
        build_args.push(arg);
    }

    let mut env = task.env;
    if !app_args.is_empty() {
        env.push((
            DEBUG_ARGS_ENV_VAR.to_string(),
            serde_json::to_string(&app_args).ok()?,
        ));
    }

    Some(DebugScenario {
        label: resolved_label.to_string(),
        adapter: debug_adapter_name.to_string(),
        build: Some(BuildTaskDefinition::Template(
            BuildTaskDefinitionTemplatePayload {
                locator_name: Some(LOCATOR_NAME.to_string()),
                template: TaskTemplate {
                    label: format!("{resolved_label} (build)"),
                    command: task.command,
                    args: build_args,
                    env,
                    cwd: task.cwd,
                },
            },
        )),
        config: "{}".to_string(),
        tcp_connection: None,
    })
}

fn is_absolute_path(path: &str) -> bool {
    // Also treat Windows paths (C:\..., \\server\...) as absolute; the wasm
    // sandbox is unix-flavored, so std's Path::is_absolute can't be used.
    path.starts_with('/') || path.starts_with('\\') || path.chars().nth(1) == Some(':')
}

struct ParsedBuildTask {
    /// Absolute path to the project file, or a directory containing one.
    project: String,
    /// `-p:Name=Value` properties for msbuild evaluation.
    properties: Vec<String>,
}

fn parse_build_task(build_task: &TaskTemplate) -> zed::Result<ParsedBuildTask> {
    if build_task.command != "dotnet"
        || build_task.args.first().map(String::as_str) != Some("build")
    {
        return Err(format!(
            "The dotnet locator cannot resolve the build task `{} {}`",
            build_task.command,
            build_task.args.join(" ")
        ));
    }

    let mut project = None;
    let mut properties = Vec::new();
    let mut args = build_task.args[1..].iter();
    while let Some(arg) = args.next() {
        let (option, value) = match arg.split_once('=') {
            Some((option, value)) => (option, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        // MSBuild evaluates TargetPath differently depending on these options,
        // so they must be forwarded as properties.
        let property = match option {
            "-c" | "--configuration" => Some("Configuration"),
            "-f" | "--framework" => Some("TargetFramework"),
            "-r" | "--runtime" => Some("RuntimeIdentifier"),
            _ => None,
        };
        let takes_value = BUILD_OPTIONS_WITH_VALUE.contains(&option);
        let value = match (value, takes_value) {
            (Some(value), _) => Some(value),
            (None, true) => args.next().cloned(),
            (None, false) => None,
        };
        if let Some(property) = property {
            if let Some(value) = value {
                properties.push(format!("-p:{property}={value}"));
            }
        } else if matches!(option, "-p" | "--property") {
            if let Some(value) = value {
                properties.push(format!("-p:{value}"));
            }
        } else if arg.starts_with("-p:") || arg.starts_with("/p:") {
            properties.push(arg.clone());
        } else if !arg.starts_with('-') && !takes_value && project.is_none() {
            project = Some(arg.clone());
        }
    }

    let project = match project {
        Some(project) if is_absolute_path(&project) => project,
        Some(project) => {
            let cwd = build_task.cwd.as_ref().ok_or(
                "The task has no working directory to resolve the project path against",
            )?;
            format!("{cwd}/{project}")
        }
        None => build_task
            .cwd
            .clone()
            .ok_or("The task has no working directory and no explicit project path")?,
    };

    Ok(ParsedBuildTask {
        project,
        properties,
    })
}

/// Runs after the build task succeeded: asks MSBuild where the built assembly
/// ended up and turns that into a launch request for the debugger.
pub fn run_locator(build_task: TaskTemplate) -> zed::Result<DebugRequest> {
    let parsed = parse_build_task(&build_task)?;

    let mut msbuild_args = vec!["msbuild".to_string(), parsed.project.clone()];
    msbuild_args.extend(parsed.properties);
    msbuild_args.push("-getProperty:TargetPath".to_string());

    let output = zed::Command {
        command: "dotnet".to_string(),
        args: msbuild_args,
        env: build_task.env.clone(),
    }
    .output()
    .map_err(|e| format!("Failed to run dotnet msbuild: {}", e))?;

    if output.status != Some(0) {
        return Err(format!(
            "Failed to determine the built assembly path for {}: {}",
            parsed.project,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let target_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if target_path.is_empty() {
        return Err(format!(
            "MSBuild reported no TargetPath for {}. If the project targets multiple frameworks, \
            add e.g. `--framework net9.0` to the task.",
            parsed.project
        ));
    }

    let args = build_task
        .env
        .iter()
        .find(|(key, _)| key == DEBUG_ARGS_ENV_VAR)
        .map(|(_, value)| serde_json::from_str(value))
        .transpose()
        .map_err(|e| format!("Failed to parse saved program arguments: {}", e))?
        .unwrap_or_default();

    Ok(DebugRequest::Launch(LaunchRequest {
        program: target_path,
        cwd: build_task.cwd,
        args,
        envs: build_task
            .env
            .into_iter()
            .filter(|(key, _)| key != DEBUG_ARGS_ENV_VAR)
            .collect(),
    }))
}
