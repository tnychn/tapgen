use std::collections::HashMap;
use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::str::FromStr;

use anyhow::{bail, Context as _, Error, Result};
use chrono::prelude::*;
use clap::Args;
use minijinja::{Environment, Value};
use tapgen::metadata::Metadata;
use tempfile::NamedTempFile;
use walkdir::WalkDir;

use tapgen::template::{Output, Template};
use tapgen::variable::{Variable, VariableValue};

use crate::config::Config;
use crate::copy::copy_dir_all;
use crate::git::{self, Source as GitSource};
use crate::prefix::Source as PrefixSource;
use crate::prompt;

#[derive(Clone)]
enum Source {
    Path(PathBuf),
    Git(GitSource),
    Prefix(PrefixSource),
}

impl FromStr for Source {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(source) = GitSource::from_str(s) {
            return Ok(Self::Git(source));
        } else if let Ok(source) = PrefixSource::from_str(s) {
            return Ok(Self::Prefix(source));
        }
        Ok(Self::Path(PathBuf::from(s)))
    }
}

impl Source {
    fn kind(&self) -> &'static str {
        match self {
            Self::Path(_) => "path",
            Self::Git(source) => {
                if source.path.is_some() {
                    "git+path"
                } else {
                    "git"
                }
            }
            Self::Prefix(_) => "prefix",
        }
    }

    fn resolve(&self, prefix: impl AsRef<Path>) -> Result<PathBuf> {
        let mut path = match self {
            Self::Git(source) => source
                .resolve(prefix)
                .context(format!("failed to resolve git source: '{source}'"))?,
            Self::Prefix(source) => prefix.as_ref().join(source),
            Self::Path(path) => path.clone(),
        };
        if path.is_dir() {
            path.push("tapgen.toml");
        }
        path.canonicalize().context(format!(
            "failed to resolve path: '{}' (source kind: {})",
            path.display(),
            self.kind()
        ))
    }
}

#[derive(Clone, Args)]
pub(crate) struct Generate {
    #[arg(
        help = "Source of template to be generated from.",
        value_parser = Source::from_str,
    )]
    src: Source,
    #[arg(
        help = "Destination of generated output to be applied to.",
        default_value = std::env::current_dir()
            .expect("failed to locate current directory")
            .into_os_string(),
        )]
    dst: PathBuf,
    #[arg(short = 'O', long = "overwrite", help = "Overwrite existing files.")]
    overwrite: bool,
}

impl Generate {
    pub(crate) fn run(&self, config: &Config) -> Result<()> {
        let path = self.src.resolve(&config.prefix)?;
        let template = Template::load(&path)
            .context(format!("failed to load template from '{}'", path.display()))?;
        print_template_metadata(&template.metadata);
        {
            let script = template.root.join("tapgen.before.hook");
            if script.exists() {
                println!();
                if prompt::confirm("Run before hook?", Some(true)) {
                    let status = run_hook_script(&script, &template.root)?;
                    if !status.success() {
                        bail!("before hook failed with {status}")
                    }
                }
            }
        }
        println!();
        let mut values = HashMap::new();
        {
            if git::check_installed()? {
                values.insert(
                    String::from("_git"),
                    Value::from_serializable(&git::obtain_config()?),
                );
            }
        }
        {
            let now = Local::now();
            values.insert(
                String::from("_now"),
                Value::from_serializable(&HashMap::from([
                    ("year", now.year() as u32),
                    ("month", now.month()),
                    ("day", now.day()),
                    ("hour", now.hour()),
                    ("minute", now.minute()),
                    ("second", now.second()),
                ])),
            );
        }
        {
            for (name, variable) in &template.variables {
                if let Some(condition) = &variable.condition {
                    if !condition
                        .eval(&values)
                        .context(format!(
                            "failed to evaluate condition for variable: '{name}'"
                        ))?
                        .is_true()
                    {
                        continue;
                    }
                }
                let value = prompt_variable(variable);
                values.insert(name.clone(), value);
            }
        }
        println!();
        println!("Generating from template...");
        let output = template
            .generate(&values)
            .context("failed to generate from template")?;
        println!("Successfully generated output to temporary directory!");
        println!("=> '{}'", output.path().display());
        {
            let script = template.root.join("tapgen.after.hook");
            if script.exists() {
                println!();
                if prompt::confirm("Run after hook?", Some(true)) {
                    let status = run_hook_script(
                        render_hook_script_as_template(script, &template.environment, &values)?,
                        output.base(),
                    )?;
                    if !status.success() {
                        bail!("after hook failed with {status}")
                    }
                }
            }
        }
        {
            println!();
            inspect_output(&output);
            confirm_output(output, &self.dst, self.overwrite)?;
        }
        Ok(())
    }
}

fn print_template_metadata(metadata: &Metadata) {
    println!(
        "You are currently using '{}' by {}.",
        metadata.name, metadata.author
    );
    if let Some(description) = &metadata.description {
        println!("{description}");
    }
    if let Some(url) = &metadata.url {
        println!("> {url}");
    }
}

fn run_hook_script(path: impl AsRef<Path>, cwd: impl AsRef<Path>) -> Result<ExitStatus> {
    let path = path.as_ref();
    Command::new(path)
        .current_dir(&cwd)
        .status()
        .context(format!("failed to run hook script: '{}'", path.display()))
}

fn render_hook_script_as_template(
    path: impl AsRef<Path>,
    env: &Environment<'static>,
    values: &HashMap<String, Value>,
) -> Result<NamedTempFile> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)
        .context(format!("failed to read hook script: '{}'", path.display()))?;
    let template = env.template_from_str(&source).context(format!(
        "failed to load hook script as template: '{}'",
        path.display()
    ))?;
    let file = NamedTempFile::with_prefix("").context("failed to create temporary file")?;
    template.render_to_write(values, &file).context(format!(
        "failed to render hook script as template: '{}'",
        path.display()
    ))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let perms = Permissions::from_mode(0o744);
        file.as_file()
            .set_permissions(perms)
            .context("failed to set temporary file permission")?;
    }
    Ok(file)
}

fn prompt_variable(variable: &Variable) -> Value {
    match &variable.value {
        VariableValue::String {
            default,
            pattern,
            choices,
        } => {
            let default = if default.is_empty() {
                None
            } else {
                Some(default.clone())
            };
            if let Some(choices) = choices {
                Value::from(prompt::select(&variable.prompt, choices, default))
            } else {
                let validator = pattern.as_ref().map(|pattern| {
                    |input: &String| {
                        if !pattern.is_match(input) {
                            let pattern = pattern.as_str();
                            bail!("input does not match pattern: `{pattern}`")
                        }
                        Ok(())
                    }
                });
                Value::from(prompt::input(&variable.prompt, default, validator))
            }
        }
        VariableValue::Array { default, choices } => Value::from(prompt::multi_select(
            &variable.prompt,
            choices,
            Some(default),
        )),
        VariableValue::Integer { default, range } => Value::from(prompt::input(
            &variable.prompt,
            Some(*default),
            Some(|input: &i64| {
                if let Some((min, max)) = range {
                    if input < min || input > max {
                        bail!("input out of range: [{min}, {max}]")
                    }
                }
                Ok(())
            }),
        )),
        VariableValue::Boolean { default } => {
            Value::from(prompt::confirm(&variable.prompt, Some(*default)))
        }
    }
}

fn inspect_output(output: &Output) {
    // TODO: improve output readability
    println!("[Output]");
    let walker = WalkDir::new(output.base());
    for entry in walker {
        let entry = entry.unwrap();
        let depth = entry.depth();
        let indent = " ".repeat(depth * 4);
        println!("│ {}{}", indent, entry.file_name().to_string_lossy());
    }
}

fn confirm_output(output: Output, dst: impl AsRef<Path>, force: bool) -> Result<()> {
    let tempdir = output.into_tempdir();
    if prompt::confirm(
        if force {
            "Apply output (force overwrite)?"
        } else {
            "Apply output?"
        },
        Some(true),
    ) {
        let (c, o, s) =
            copy_dir_all(&dst, tempdir, &dst, force).context("failed to apply output")?;
        println!("Successfully applied output to destination!");
        println!("Created {c} files. Overwritten {o} files. Skipped {s} files.");
    } else {
        tempdir.close().context("failed to dispose output")?;
        println!("Disposed output!");
    }
    Ok(())
}
