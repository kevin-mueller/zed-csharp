//! Building a project for debugging and finding its output, by asking MSBuild instead of
//! guessing paths. Runs `dotnet` as a process, as std::fs doesn't work in the debug locator.

use zed_extension_api::{self as zed, serde_json};

use crate::dotnet_run::DotnetInvocation;

pub struct BuildOutput {
    /// The built assembly.
    pub target_path: String,
    /// MSBuild's `RunWorkingDirectory`, if the project sets one.
    pub run_working_directory: Option<String>,
}

/// Builds the project like the task's `dotnet run` / `dotnet test` would (configuration,
/// framework, MSBuild properties, `--no-build`) and returns where the assembly ended up.
///
/// The path is queried from MSBuild (`TargetPath`), so custom `AssemblyName`s, `OutputPath`s,
/// artifacts output and properties from `Directory.Build.props` are all respected.
/// Multi-targeting projects use their first target framework unless the task sets one.
pub fn build_project(csproj: &str, invocation: &DotnetInvocation) -> Result<BuildOutput, String> {
    let configuration = invocation.configuration.as_deref().unwrap_or("Debug");
    let framework = match &invocation.framework {
        Some(framework) => Some(framework.clone()),
        None => default_target_framework(csproj)?,
    };

    let mut args = if invocation.no_build {
        // Only evaluate the project to find the existing build output.
        let mut args = vec![
            "msbuild".to_string(),
            csproj.to_string(),
            format!("-p:Configuration={configuration}"),
        ];
        args.extend(framework.map(|framework| format!("-p:TargetFramework={framework}")));
        args
    } else {
        let mut args = vec![
            "build".to_string(),
            csproj.to_string(),
            "--configuration".to_string(),
            configuration.to_string(),
        ];
        if let Some(framework) = framework {
            args.extend(["--framework".to_string(), framework]);
        }
        // `-getProperty` alone only evaluates the project, `-t:Build` makes it actually build.
        args.push("-t:Build".to_string());
        args
    };
    args.extend(
        invocation
            .properties
            .iter()
            .map(|property| format!("-p:{property}")),
    );

    let properties = query_properties(
        csproj,
        args,
        &["TargetPath", "RunWorkingDirectory"],
        "Build failed",
    )?;
    let target_path = properties
        .get("TargetPath")
        .ok_or_else(|| format!("MSBuild did not report an output path for {csproj}"))?;
    if invocation.no_build && !file_exists(&target_path) {
        return Err(format!(
            "{target_path} doesn't exist. Build the project first or remove --no-build from the task"
        ));
    }

    Ok(BuildOutput {
        target_path,
        run_working_directory: properties.get("RunWorkingDirectory"),
    })
}

/// Returns the first entry of `TargetFrameworks` for multi-targeting projects, `None` otherwise.
fn default_target_framework(csproj: &str) -> Result<Option<String>, String> {
    let properties = query_properties(
        csproj,
        vec!["msbuild".to_string(), csproj.to_string()],
        &["TargetFramework", "TargetFrameworks"],
        "Failed to evaluate",
    )?;
    if properties.get("TargetFramework").is_some() {
        return Ok(None);
    }
    Ok(properties.get("TargetFrameworks").and_then(|frameworks| {
        frameworks
            .split(';')
            .map(str::trim)
            .find(|framework| !framework.is_empty())
            .map(str::to_string)
    }))
}

/// MSBuild properties as printed for multiple `-getProperty` arguments.
struct Properties(serde_json::Value);

impl Properties {
    /// The property's value, or `None` if it is empty.
    fn get(&self, name: &str) -> Option<String> {
        self.0["Properties"][name]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }
}

/// Runs `dotnet <args> -getProperty:<name>...` and returns the reported properties.
/// `failure` prefixes the error message if the command fails.
fn query_properties(
    csproj: &str,
    mut args: Vec<String>,
    names: &[&str],
    failure: &str,
) -> Result<Properties, String> {
    // With a single -getProperty MSBuild prints the plain value instead of JSON.
    debug_assert!(names.len() > 1);
    args.extend(names.iter().map(|name| format!("-getProperty:{name}")));

    let output = zed::Command::new("dotnet")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run dotnet: {e}"))?;
    if output.status != Some(0) {
        return Err(format!(
            "{failure} for {csproj}:\n{}",
            build_errors(&output)
        ));
    }

    serde_json::from_slice(&output.stdout)
        .map(Properties)
        .map_err(|e| format!("Failed to parse MSBuild output: {e}"))
}

/// The error lines of a failed build, or its whole output if there are none.
fn build_errors(output: &zed::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut errors: Vec<&str> = Vec::new();
    for line in stderr.lines().chain(stdout.lines()) {
        // MSBuild appends the project path to every line: "... error CS1002: ; expected [/x/y.csproj]"
        let line = match line.trim().rsplit_once(" [") {
            Some((message, project)) if project.ends_with("proj]") => message,
            _ => line.trim(),
        };
        if line.contains(": error ") && !errors.contains(&line) {
            errors.push(line);
        }
    }

    if errors.is_empty() {
        [stderr.trim(), stdout.trim()]
            .into_iter()
            .filter(|output| !output.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        errors.join("\n")
    }
}

fn file_exists(path: &str) -> bool {
    zed::Command::new("test")
        .arg("-f")
        .arg(path)
        .output()
        .is_ok_and(|output| output.status == Some(0))
}
