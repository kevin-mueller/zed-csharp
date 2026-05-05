# Zed C# & Razor

This repository is a fork of the [official C# extension](https://github.com/zed-extensions/csharp) for Zed. It's pretty experimental, and I'm fully aware that some things are implemented a bit... let's say *pragmatically*. But I thought sharing it wouldn't hurt. :)

## Features

This extension provides a couple of additional features to the existing one:

### Debugger Support

Integrates the great work from [zed-netcoredbg](https://github.com/qwadrox/zed-netcoredbg) and some parts of [@Tiggilyboo](https://github.com/Tiggilyboo)'s PR. Big thanks for both of your work!

This even includes debugging support for tests. And the tests can even be started via a little ">" icon next to the test name! Crazy, right?

For normal build tasks, the extension tries it's best to automatically turn them into debug tasks. If something is missing, you can always add it manually. See [configuration](https://github.com/qwadrox/zed-netcoredbg?tab=readme-ov-file#configuration). Or just run the project manually, and use zed's attach interface.

### Razor Support

Adds syntax highlighting and a LSP for the razor language. Built on a custom fork of [tree-sitter-razor](https://github.com/kevin-mueller/tree-sitter-razor) and the official roslyn lsp. Razor support is further enhanced by mixing in the html lsp for .razor files.

### Official Roslyn LSP & Lightweight csharp-ls

Uses the latest version of the [official roslyn language server](https://github.com/dotnet/roslyn/tree/main/src/LanguageServer/Microsoft.CodeAnalysis.LanguageServer). This enables support for razor and improves the overall epxerience quite a bit.
Optionally, [csharp-ls](https://github.com/razzmatazz/csharp-language-server) can also be used, which is more lightweight, but not as fully featured.

### Built-In Language Tasks

The extension provides some pre-defined tasks to make working with .NET projects easier.
These include:
- Build Project
- Run Project
- Test Project
- Build Solution
- Test Solution

(the appropriate csproj file is resolved automatically)

### Snippets!

There are some very basic snippets that make the creation of new classes or interfaces easier. More coming soon™. Contributions welcome!

### Offline Support

The original extension did not work when not connected to the internet. This has been fixed by failing gracefully for network related operations.


## Setup

First install https://rustup.rs. Then clone this repositry and install it as a "Dev Extension" in Zed.
```bash
git clone https://github.com/kevin-mueller/zed-csharp
```

I'm using the preview version of Zed, but you don't have to.

**Required** changes to settings.json:
```jsonc
{
  "lsp": {
    "roslyn-official": {
      "settings": {
        // this is required for the razor lsp extensions. If you omit it, there will be no razor lsp support!
        // it will be downloaded automatically. You just have to provide a path
        "razor_source_repository_root": "/some/place/nice/razor" 
      }
    },
  },
  // the roslyn lsp crashes for textDocument/documentColor requests
  // didn't find a way to disable it only for this extension
  "lsp_document_colors": "none",
  "languages": {
    "CSharp": {
      "language_servers": [
        "roslyn-official" // or "csharp-ls" for a more lightweight alternative
      ]
    },
  }
}
```

**Recommended** changes to settings.json:
```jsonc
{
  // improves the c# syntax highliting further
  "semantic_tokens": "combined",
  // to enable inline reference hints
  "code_lens": "on"
}
```

## What's missing?

This extension is far from perfect. Here are a couple caveats:
1. Windows support! I never tested this on Windows, only Linux and MacOS. I think the biggest problem are the custom bash commands this extension relies on. Adding Windows support should be doable though.
2. The razor syntax highlighting is still a bit wonky. Especially the html syntax injection. My custom [tree-sitter fork](https://github.com/kevin-mueller/tree-sitter-razor) does solve some problems from the original one, but it should be mentioned that I have zero knowledge about tree sitter syntax. Improvements here would be very welcome!
3. The official Roslyn LSP is very new. Occasionally, the server crashes and has to be restarted. That said, it works pretty well for large code bases (~100 Projects) and much more reliably than the other LSPs I've tried.
4. LaunchProfiles are not yet supported. I'm not sure if it's possible with the current state of the zed extension framework. For now you'll have to define project tasks manually. For how to do that via debugging, check the original [zed-netcoredbg readme](https://github.com/qwadrox/zed-netcoredbg?tab=readme-ov-file#configuration).
5. LSP's are communicating with Zed via stdio, instead of named pipes. The roslyn language server would actually support named pipes, but I think Zed doesn't yet. Something to keep an eye on.

---

## Technical Challanges
*aka: I don't know what I'm doing*

I'm keeping this section both as a little warning and as a kind request for contributions. :)

**Binary Management**

The extension requires multiple binaries to work properly. Managing them is a bit of a pain.

1. The roslyn-lanuage-server dotnet tool. This is actually pretty ok to manage, because the extension can just install & update it via the dotnet command. Still, it has the problem of installing / updating a gloabl tool, outside of the scope of the extension.
2. The `Microsoft.VisualStudioCode.RazorExtension.dll`. This is required to provide razor LSP functionality. It is currently fetched by cloning the [source repo](https://github.com/dotnet/razor) and compiling the `Microsoft.VisualStudioCode.RazorExtension.csproj`. If someone knows of a binary distribution of this, please let me know.
3. The `Microsoft.CodeAnalysis.Razor.Compiler.dll` and `Microsoft.NET.Sdk.Razor.DesignTime.targets`. These files are also required to provide razor LSP support. They are shipped via the dotnet sdk and are currently read from there. The latest sdk version is detemined by running `dotnet --list-sdks` and parsing the output. *yikes*.
4. The `vscode-html-language-server` executable. It is used by the html-in-razor, which is required to get the html lsp working inside of .razor files. It is currently provided the exact same way the official, bundled html extension does it. Which is via npm.
5. The "Open Project" command is currently hard-coded to use the `zed-preview` executable. If you don't use the preview, you could add an alias, or simply change it in the tasks.json file in the csharp language folder.

**Build Context Resolution**

There is no way of knowing what csproj should be targeted for any given build / run / debug task. Zed does provide some [placeholder variables](https://zed.dev/docs/tasks#variables) for tasks though.

However, Zed's system for automatically turning such language tasks into debuggable tasks doesn't resolve these variables soon enough. Meaning, that when the debugger scenario locator (`dap_locator_create_scenario`) receives a request to debug "My Cool Task", we have absolutely no way of knowing what projects to compile.

Only one step later (`run_dap_locator`), where the actual debugger process should be launched (and the binaries should already be built), do we have access to those variables.

The extension works around this by building the required binaries via a custom command directly in `run_dap_locator`. This works, but has the drawback of no visual feedback being provided to the user that the project is being compiled.

Another way would be to simply build the entire solution during `dap_locator_create_scenario`, because we do know the root folder. But I think that would be annoying if you, for example, want to debug a single test in a project that is not related to another project, which might still contain build errors.

**File System Interactions**

For some reason, I didn't manage to make `std::fs` commands work in the two debugger methods. My gut feeling tells me that this is related to the sandboxing of extensions, but I'm not sure.
The workaround for this is to just use basic shell commands like `cat`, `find`, or `test`. Suboptimal, because they only work on unix based systems.

**Razor HTML Injection**

Razor is baiscally a mix of C# + HTML. To get both the HTML and Roslyn LSP working in a .razor file, the extension has to provide a custom HTML LSP. We sadly cannot reference another extension for this.

The `html-in-razor` LSP, which this extension provides, is a 1-1 copy of the offical one, bundled with Zed.

This is suboptimal, because the two will drift appart sooner or later.



## Development / Contribution

Please, feel free to contribute to this extension. My knowledge of rust and the zed internals is very limited, so some help would be much appreciated.

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.
