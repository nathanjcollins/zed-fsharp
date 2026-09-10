This is a basic extension for Zed to enable F# features.

## Installation

Inside your [plugin directory](https://zed.dev/docs/extensions/installing-extensions#installation-location), run:

```sh
git clone git@github.com:nathanjcollins/zed-fsharp.git installed/fsharp
```

You can update the plugin to a pre-release version in the Zed installed plugin tab.

## Configuration

Settings are defined in your Zed `settings.json` (or per project in `.zed/settings.json`):

Below are a list of recommended configuration settings for F# projects:

### `env` 

```jsonc
{
  "lsp": {
    "fsautocomplete": {
      "binary": {
        "env": {
          // EXPERIMENTAL: Parallel project reference resolution in FSharp.Compiler.Service
          "FCS_ParallelReferenceResolution": "true",
          // Server GC: better throughput for solution-wide analysis
          "DOTNET_GCServer": "1",
          // DATAS (.NET 8+): scales GC heaps with live data instead of one-per-core,
          // keeping server GC's memory footprint reasonable
          "DOTNET_GCDynamicAdaptationMode": "1"
        }
      }
    }
  }
}
```

### `initialization_options`

Allows you to pass additional initialization options to `fsautocomplete`.

Your options take precedence over the default initialization options.

Below is non exhaustive list of available initialization options, with their default values.

```jsonc
{
  "lsp": {
    "fsautocomplete": {
      "initialization_options": {
        "UnnecessaryParenthesesAnalyzer": true,
        "AddPrivateAccessModifier": true,
        "ExternalAutocomplete": false,
        "fsac": {
          "cachedTypeCheckCount": 200,
        },
        "UnusedOpensAnalyzer": true,
        "UnusedDeclarationsAnalyzer": true,
        "InterfaceStubGeneration": true,
        "AbstractClassStubGeneration": true,
        "UnionCaseStubGeneration": true,
        "RecordStubGeneration": true,
        "TooltipShowDocumentationLink": false,
        // etc.
      },
    }
  }
}
```

### `fsac_custom_args`

Allows you to pass additional command-line arguments to `fsautocomplete`.

```jsonc
{
  "lsp": {
    "fsautocomplete": {
      "settings": {
        "fsac_custom_args": [
          // EXPERIMENTAL: Enable MSBuild Graph workspace loading. Should be faster than the default workspace loading
          "--project-graph-enabled",
          // EXPERIMENTAL: Use Transparent Compiler in FSharp.Compiler.Service. Should have better performance characteristics
          // See https://github.com/dotnet/fsharp/pull/15179 for more details
          "--use-fcs-transparent-compiler"
        ]
      }
    }
  }
}
```

The arguments above are the recommended configuration.

Run `fsautocomplete --help` to see all available arguments.

### `fsac_custom_path`

Allows you to use your own `fsautocomplete` instead of the one automatically downloaded by the extension. The path must point to `fsautocomplete.dll`, as it is run via `dotnet`.

```jsonc
{
  "lsp": {
    "fsautocomplete": {
      "settings": {
        "fsac_custom_path": "/absolute/path/to/fsautocomplete.dll"
      }
    }
  }
}
```

## Debugging

The extension ships a debug adapter based on [netcoredbg](https://github.com/Samsung/netcoredbg), which is downloaded automatically from its GitHub releases the first time you start a debug session.

### Debugging a `dotnet run` task (recommended)

If you have a `dotnet run` task in your `.zed/tasks.json`:

```jsonc
[
  {
    "label": "run my app",
    "command": "dotnet",
    // `--project` and arguments after `--` are supported
    "args": ["run", "--project", "src/MyApp"],
  }
]
```

then the task shows up in the debugger's new session modal (`debugger: start`) and can be debugged directly: the extension turns it into a `dotnet build` invocation, asks MSBuild for the built assembly path, and launches that assembly under the debugger. No `debug.json` and no hardcoded DLL path needed.

Note that the task must use `"command": "dotnet"` with the arguments in `"args"` (as above) to be recognized, and resolving the assembly path requires .NET SDK 8 or later. If the project targets multiple frameworks, add e.g. `--framework net9.0` to the task's args.

### Debugging with an explicit configuration

Alternatively, create a `.zed/debug.json` in your project:

```jsonc
[
  {
    "label": "Debug F# app",
    "adapter": "netcoredbg",
    "request": "launch",
    // Path to the assembly produced by `dotnet build`
    "program": "$ZED_WORKTREE_ROOT/bin/Debug/net9.0/MyApp.dll",
    "cwd": "$ZED_WORKTREE_ROOT",
    "args": [],
    "env": {},
    "stopAtEntry": false
  }
]
```

Build your project first (`dotnet build`), or add a `build` step to the configuration so Zed builds before every session:

```jsonc
[
  {
    "label": "Debug F# app (with build)",
    "adapter": "netcoredbg",
    "request": "launch",
    "program": "$ZED_WORKTREE_ROOT/bin/Debug/net9.0/MyApp.dll",
    "build": {
      "command": "dotnet",
      "args": ["build"]
    }
  }
]
```

You can also attach to a running .NET process:

```jsonc
[
  {
    "label": "Attach to F# app",
    "adapter": "netcoredbg",
    "request": "attach",
    "processId": 12345
  }
]
```

Alternatively, launching via the new session modal (`debugger: start`) works without any configuration file.

To use your own netcoredbg build instead of the downloaded one (e.g. on platforms without official builds, such as macOS x64), point Zed at it in your `settings.json`:

```jsonc
{
  "dap": {
    "netcoredbg": {
      "binary": "/absolute/path/to/netcoredbg"
    }
  }
}
```

Feel free to contribute.
