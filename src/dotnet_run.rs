//! Turns a `dotnet run` / `dotnet test` task into what the debugger needs: which project to
//! build with which options, and the arguments, environment and working directory to launch
//! the program with. Launch profiles are applied the same way `dotnet run` applies them.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// The parts of a `dotnet run` / `dotnet test` command line that matter for debugging.
#[derive(Debug, Default, PartialEq)]
pub struct DotnetInvocation {
    pub subcommand: Subcommand,
    /// `--project` for `dotnet run`, the positional project argument for `dotnet test`.
    pub project: Option<String>,
    pub configuration: Option<String>,
    pub framework: Option<String>,
    pub launch_profile: Option<String>,
    pub no_launch_profile: bool,
    pub no_build: bool,
    /// MSBuild properties (`-p:Name=Value`), passed on to the build.
    pub properties: Vec<String>,
    /// Environment variables from `-e NAME=VALUE`, passed to the program.
    pub environment: Vec<(String, String)>,
    /// Arguments after `--`, passed to the program.
    pub program_args: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Subcommand {
    #[default]
    Run,
    Test,
}

impl DotnetInvocation {
    /// Parses a task's command and args, e.g. `"dotnet run"` + `["--project", "x.csproj"]`.
    /// Returns `None` if it is not a `dotnet run` or `dotnet test` command.
    pub fn parse(command: &str, args: &[String]) -> Option<Self> {
        let mut tokens = command
            .split_whitespace()
            .map(str::to_string)
            .chain(args.iter().cloned())
            .map(|token| unquote(&token).to_string());

        let program = tokens.next()?;
        if program != "dotnet" && !program.ends_with("/dotnet") {
            return None;
        }

        let mut invocation = DotnetInvocation {
            subcommand: match tokens.next()?.as_str() {
                "run" => Subcommand::Run,
                "test" => Subcommand::Test,
                _ => return None,
            },
            ..Default::default()
        };

        while let Some(token) = tokens.next() {
            if token == "--" {
                invocation.program_args.extend(tokens.by_ref());
                break;
            }

            // Options can be written as `--name value` or `--name=value` / `--name:value`.
            let (name, inline_value) = match token.find(['=', ':']) {
                Some(index) if token.starts_with('-') => {
                    (&token[..index], Some(token[index + 1..].to_string()))
                }
                _ => (token.as_str(), None),
            };
            let mut value = || inline_value.clone().or_else(|| tokens.next());

            match name {
                "--project" => invocation.project = value(),
                // `-p` is the deprecated short form of `--project` for `dotnet run`, unless it
                // is written like an MSBuild property (`-p:Name=Value`).
                "-p" if inline_value.is_none() && invocation.subcommand == Subcommand::Run => {
                    invocation.project = value()
                }
                "-p" | "--property" => {
                    if let Some(property) = value() {
                        invocation.properties.push(property);
                    }
                }
                "-c" | "--configuration" => invocation.configuration = value(),
                "-f" | "--framework" => invocation.framework = value(),
                "-lp" | "--launch-profile" => invocation.launch_profile = value(),
                "--no-launch-profile" => invocation.no_launch_profile = true,
                "--no-build" => invocation.no_build = true,
                "-e" | "--environment" => {
                    if let Some((key, value)) = value().as_deref().and_then(|v| v.split_once('=')) {
                        invocation
                            .environment
                            .push((key.to_string(), value.to_string()));
                    }
                }
                // Skip the values of other options, so they aren't mistaken for the project.
                _ if OPTIONS_WITH_VALUE.contains(&name) => {
                    value();
                }
                _ if invocation.subcommand == Subcommand::Test
                    && invocation.project.is_none()
                    && is_project_like(&token) =>
                {
                    invocation.project = Some(token.clone());
                }
                // Everything else doesn't matter for debugging.
                _ => {}
            }
        }

        Some(invocation)
    }
}

/// Options of `dotnet run` / `dotnet test` that take a value, apart from the ones parsed above.
const OPTIONS_WITH_VALUE: &[&str] = &[
    "-v",
    "--verbosity",
    "-a",
    "--arch",
    "--os",
    "-r",
    "--runtime",
    "-s",
    "--settings",
    "-l",
    "--logger",
    "-o",
    "--output",
    "--results-directory",
    "--filter",
    "--test-adapter-path",
    "--diag",
    "--collect",
    "--blame-crash-dump-type",
    "--blame-hang-dump-type",
    "--blame-hang-timeout",
];

/// Whether a positional `dotnet test` argument looks like a project, solution or directory.
fn is_project_like(token: &str) -> bool {
    !token.starts_with('-')
        && (token.ends_with("proj")
            || token.ends_with(".sln")
            || token.ends_with(".slnx")
            || token.contains('/'))
}

/// What a launch profile adds to the launched program.
#[derive(Debug, Default, PartialEq)]
pub struct LaunchProfile {
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
}

/// Picks and applies a launch profile from the contents of `Properties/launchSettings.json`,
/// following `dotnet run`: an explicitly requested profile, or else the first profile with
/// `"commandName": "Project"`. `env` is used to expand `%VAR%` references.
pub fn launch_profile(
    launch_settings: Option<&str>,
    requested: Option<&str>,
    env: &HashMap<String, String>,
) -> Result<Option<LaunchProfile>, String> {
    let Some(launch_settings) = launch_settings else {
        return match requested {
            Some(name) => Err(format!(
                "Launch profile '{name}' requested, but the project has no Properties/launchSettings.json"
            )),
            None => Ok(None),
        };
    };

    let json: Value = serde_json::from_str(&strip_json_comments(launch_settings))
        .map_err(|e| format!("Failed to parse launchSettings.json: {e}"))?;
    let Some(profiles) = json.get("profiles").and_then(Value::as_object) else {
        return Ok(None);
    };

    let is_project =
        |profile: &Value| profile.get("commandName").and_then(Value::as_str) == Some("Project");

    let (name, profile) = match requested {
        Some(name) => {
            let profile = profiles.get(name).ok_or_else(|| {
                format!("Launch profile '{name}' not found in launchSettings.json")
            })?;
            if !is_project(profile) {
                let command_name = profile
                    .get("commandName")
                    .and_then(Value::as_str)
                    .unwrap_or("<none>");
                return Err(format!(
                    "Launch profile '{name}' has commandName '{command_name}', only 'Project' profiles can be debugged"
                ));
            }
            (name.to_string(), profile)
        }
        None => match profiles.iter().find(|(_, profile)| is_project(profile)) {
            Some((name, profile)) => (name.clone(), profile),
            None => return Ok(None),
        },
    };

    let string = |key: &str| profile.get(key).and_then(Value::as_str);

    let mut profile_env: Vec<(String, String)> = profile
        .get("environmentVariables")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| Some((key.clone(), expand_env_vars(value.as_str()?, env))))
        .collect();
    if let Some(urls) = string("applicationUrl") {
        if !profile_env.iter().any(|(key, _)| key == "ASPNETCORE_URLS") {
            profile_env.push(("ASPNETCORE_URLS".to_string(), urls.to_string()));
        }
    }
    profile_env.push(("DOTNET_LAUNCH_PROFILE".to_string(), name));

    Ok(Some(LaunchProfile {
        env: profile_env,
        args: string("commandLineArgs")
            .map(|args| split_command_line(&expand_env_vars(args, env)))
            .unwrap_or_default(),
        working_directory: string("workingDirectory").map(|dir| expand_env_vars(dir, env)),
    }))
}

/// Resolves `path` against `base` unless it's absolute.
pub fn resolve_path(base: &str, path: &str) -> String {
    if Path::new(path).is_absolute() {
        path.to_string()
    } else {
        Path::new(base).join(path).to_string_lossy().to_string()
    }
}

/// Strips one level of matching single or double quotes, e.g. from `'$ZED_FILE'` in tasks.json.
fn unquote(token: &str) -> &str {
    for quote in ['\'', '"'] {
        if let Some(inner) = token
            .strip_prefix(quote)
            .and_then(|token| token.strip_suffix(quote))
        {
            return inner;
        }
    }
    token
}

/// Splits `commandLineArgs` into arguments: whitespace separates, double quotes group,
/// and `\"` is a literal quote.
fn split_command_line(command_line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_arg = false;
    let mut in_quotes = false;
    let mut chars = command_line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
                in_arg = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                in_arg = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if in_arg {
                    args.push(std::mem::take(&mut current));
                    in_arg = false;
                }
            }
            c => {
                current.push(c);
                in_arg = true;
            }
        }
    }
    if in_arg {
        args.push(current);
    }
    args
}

/// Expands `%NAME%` references like `dotnet run` does. Unknown variables are left as they are.
fn expand_env_vars(value: &str, env: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if env.contains_key(&after[..end]) => {
                result.push_str(&env[&after[..end]]);
                rest = &after[end + 1..];
            }
            _ => {
                result.push('%');
                rest = after;
            }
        }
    }
    result.push_str(rest);
    result
}

/// Removes `//` and `/* */` comments and trailing commas, which launchSettings.json may contain
/// but serde_json doesn't accept. Strings are left untouched.
fn strip_json_comments(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut chars = json.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            result.push(c);
            match c {
                '\\' => result.extend(chars.next()),
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                result.push(c);
            }
            ('/', Some('/')) => while chars.next_if(|&c| c != '\n').is_some() {},
            ('/', Some('*')) => {
                chars.next();
                let mut previous = ' ';
                for c in chars.by_ref() {
                    if previous == '*' && c == '/' {
                        break;
                    }
                    previous = c;
                }
            }
            (']' | '}', _) => {
                // Drop a trailing comma before the closing bracket.
                let trimmed = result.trim_end().len();
                if result[..trimmed].ends_with(',') {
                    result.truncate(trimmed - 1);
                }
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn parses_run_with_options() {
        let invocation = DotnetInvocation::parse(
            "dotnet run",
            &args(&[
                "--project",
                "src/Api/Api.csproj",
                "--launch-profile",
                "test-Console",
                "--configuration",
                "DebugNoErrors",
                "-f=net9.0",
                "-p:Foo=Bar",
                "-e",
                "FEATURE=on",
                "--",
                "--app-arg",
                "x",
            ]),
        )
        .unwrap();
        assert_eq!(
            invocation,
            DotnetInvocation {
                subcommand: Subcommand::Run,
                project: Some("src/Api/Api.csproj".into()),
                configuration: Some("DebugNoErrors".into()),
                framework: Some("net9.0".into()),
                launch_profile: Some("test-Console".into()),
                properties: args(&["Foo=Bar"]),
                environment: vec![("FEATURE".into(), "on".into())],
                program_args: args(&["--app-arg", "x"]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn parses_split_command_quotes_and_short_forms() {
        let invocation = DotnetInvocation::parse(
            "dotnet",
            &args(&[
                "run",
                "-p",
                "'/a b/X.csproj'",
                "-lp",
                "dev",
                "-c",
                "Release",
            ]),
        )
        .unwrap();
        assert_eq!(invocation.project.as_deref(), Some("/a b/X.csproj"));
        assert_eq!(invocation.launch_profile.as_deref(), Some("dev"));
        assert_eq!(invocation.configuration.as_deref(), Some("Release"));
    }

    #[test]
    fn parses_test_with_positional_project() {
        let invocation = DotnetInvocation::parse(
            "dotnet test",
            &args(&["-v", "m", "'/x/T.csproj'", "--no-build"]),
        )
        .unwrap();
        assert_eq!(invocation.subcommand, Subcommand::Test);
        assert_eq!(invocation.project.as_deref(), Some("/x/T.csproj"));
        assert!(invocation.no_build);
    }

    #[test]
    fn ignores_other_commands() {
        assert_eq!(DotnetInvocation::parse("dotnet build", &[]), None);
        assert_eq!(
            DotnetInvocation::parse("sh", &args(&["-c", "dotnet run"])),
            None
        );
    }

    const LAUNCH_SETTINGS: &str = r#"{
      // comments are allowed
      "profiles": {
        "iis": { "commandName": "IISExpress" },
        "first": {
          "commandName": "Project",
          "commandLineArgs": "--from-profile \"quoted arg\" %HOME%",
          "applicationUrl": "http://localhost:5001;https://localhost:5002",
          "environmentVariables": { "ASPNETCORE_ENVIRONMENT": "Development", "P": "%HOME%-%NOPE%-$(HOME)" },
        },
        "test-Console": {
          "commandName": "Project",
          "workingDirectory": "../elsewhere",
          "applicationUrl": "http://localhost:5009",
          "environmentVariables": { "ASPNETCORE_URLS": "http://localhost:7000" }
        }
      }
    }"#;

    fn env() -> HashMap<String, String> {
        HashMap::from([("HOME".to_string(), "/home/me".to_string())])
    }

    #[test]
    fn picks_first_project_profile_like_dotnet_run() {
        let profile = launch_profile(Some(LAUNCH_SETTINGS), None, &env())
            .unwrap()
            .unwrap();
        assert_eq!(
            profile.args,
            args(&["--from-profile", "quoted arg", "/home/me"])
        );
        assert_eq!(
            profile.env,
            vec![
                (
                    "ASPNETCORE_ENVIRONMENT".to_string(),
                    "Development".to_string()
                ),
                ("P".to_string(), "/home/me-%NOPE%-$(HOME)".to_string()),
                (
                    "ASPNETCORE_URLS".to_string(),
                    "http://localhost:5001;https://localhost:5002".to_string()
                ),
                ("DOTNET_LAUNCH_PROFILE".to_string(), "first".to_string()),
            ]
        );
    }

    #[test]
    fn default_profile_follows_file_order_not_alphabetical_order() {
        let launch_settings = r#"{ "profiles": {
            "zz-first": { "commandName": "Project" },
            "aa-second": { "commandName": "Project" }
        } }"#;
        let profile = launch_profile(Some(launch_settings), None, &env())
            .unwrap()
            .unwrap();
        assert!(profile
            .env
            .contains(&("DOTNET_LAUNCH_PROFILE".to_string(), "zz-first".to_string())));
    }

    #[test]
    fn explicit_env_wins_over_application_url() {
        let profile = launch_profile(Some(LAUNCH_SETTINGS), Some("test-Console"), &env())
            .unwrap()
            .unwrap();
        assert_eq!(
            profile.env,
            vec![
                (
                    "ASPNETCORE_URLS".to_string(),
                    "http://localhost:7000".to_string()
                ),
                (
                    "DOTNET_LAUNCH_PROFILE".to_string(),
                    "test-Console".to_string()
                ),
            ]
        );
        assert_eq!(profile.working_directory.as_deref(), Some("../elsewhere"));
    }

    #[test]
    fn reports_unusable_profiles() {
        assert!(launch_profile(Some(LAUNCH_SETTINGS), Some("nope"), &env())
            .unwrap_err()
            .contains("not found"));
        assert!(launch_profile(Some(LAUNCH_SETTINGS), Some("iis"), &env())
            .unwrap_err()
            .contains("IISExpress"));
        assert!(launch_profile(None, Some("dev"), &env()).is_err());
        assert_eq!(launch_profile(None, None, &env()), Ok(None));
    }

    #[test]
    fn strips_comments_but_not_strings() {
        assert_eq!(
            strip_json_comments(r#"{"a": "http://x // y", /* c */ "b": [1,],}"#),
            r#"{"a": "http://x // y",  "b": [1]}"#
        );
    }
}
