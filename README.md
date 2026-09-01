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

Feel free to contribute.
