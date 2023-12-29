use std::collections::HashMap;
use std::fs::{self, Permissions};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr as _;

use anyhow::{bail, Context as _, Result};
use clap::Args;
use minijinja::Environment;
use tapgen::metadata::Metadata;
use tempfile::{NamedTempFile, TempPath};
use walkdir::WalkDir;

use tapgen::template::{Output, Template};
use tapgen::variable::{Variable, VariableValue};

use crate::config::Config;
use crate::git::{self, Source as GitSource};
use crate::prompt;

#[derive(Clone, Args)]
pub(crate) struct Generate {
    #[arg(help = "Source of template to be generated from.")]
    src: String,
    #[arg(
        help = "Destination of generated output to be applied to.",
        default_value = std::env::current_dir()
            .expect("failed to locate current directory")
            .into_os_string(),
        )]
    dst: PathBuf,
}

impl Generate {
    pub(crate) fn run(&self, config: &Config) -> Result<()> {
        let mut path = if let Ok(source) = GitSource::from_str(&self.src) {
            source
                .resolve(&config.prefix)
                .context(format!("failed to resolve git source: {source}"))?
        } else {
            PathBuf::from(&self.src)
                .canonicalize()
                .context(format!("failed to resolve path source: {}", self.src))?
        };
        if path.is_dir() {
            path.push("tapgen.toml");
        }

        let template = Template::load(&path)
            .context(format!("failed to load template from {}", path.display()))?;

        print_template_metadata(&template.metadata);

        let script = template.root.join("tapgen.before.hook");
        if script.exists() {
            println!();
            run_hook_script(&script, &template.root)?;
        }

        let mut values = HashMap::new();
        if git::check_installed()? {
            values.insert(
                String::from("_git"),
                minijinja::Value::from_serializable(&git::obtain_config()?),
            );
        }
        println!();
        for (name, variable) in &template.variables {
            if let Some(condition) = &variable.condition {
                let value = condition.eval(&values).context(format!(
                    "failed to evaluate condition for variable: '{name}'"
                ))?;
                if !value.is_true() {
                    continue;
                }
            }
            let value = prompt_variable(variable);
            values.insert(name.clone(), value);
        }

        let output = template
            .generate(&values)
            .context("failed to generate from template")?;

        println!();
        inspect_output(&output);

        let base = self.dst.join(output.basename());
        if confirm_output(output, &self.dst)? {
            let script = template.root.join("tapgen.after.hook");
            if script.exists() {
                println!();
                let path = render_hook_script_as_template(script, &template.environment, &values)?;
                run_hook_script(path, base)?;
            }
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

fn run_hook_script(path: impl AsRef<Path>, cwd: impl AsRef<Path>) -> Result<()> {
    if prompt::confirm("Run hook script?", true) {
        let path = path.as_ref();
        Command::new(path)
            .current_dir(&cwd)
            .status()
            .context(format!("failed to run hook script: {}", path.display()))?;
    }
    Ok(())
}

fn render_hook_script_as_template(
    path: impl AsRef<Path>,
    env: &Environment<'static>,
    values: &HashMap<String, minijinja::Value>,
) -> Result<TempPath> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)
        .context(format!("failed to read hook script: {}", path.display()))?;
    let template = env.template_from_str(&source).context(format!(
        "failed to load hook script as template: {}",
        path.display()
    ))?;
    let file = NamedTempFile::with_prefix("").context("failed to create temporary file")?;
    template.render_to_write(values, &file).context(format!(
        "failed to render hook script as template: {}",
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
    Ok(file.into_temp_path())
}

fn prompt_variable(variable: &Variable) -> minijinja::Value {
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
                minijinja::Value::from(prompt::select(&variable.prompt, choices, default))
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
                minijinja::Value::from(prompt::input(&variable.prompt, default, validator))
            }
        }
        VariableValue::Array { default, choices } => minijinja::Value::from(prompt::multi_select(
            &variable.prompt,
            choices,
            Some(default),
        )),
        VariableValue::Integer { default, range } => minijinja::Value::from(prompt::input(
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
            minijinja::Value::from(prompt::confirm(&variable.prompt, *default))
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

fn confirm_output(output: Output, dst: impl AsRef<Path>) -> Result<bool> {
    if prompt::confirm("Apply output?", true) {
        output.apply(dst).context("failed to apply output")?;
        println!("Successfully applied output to destination!");
        Ok(true)
    } else {
        output.dispose().context("failed to dispose output")?;
        println!("Disposed output!");
        Ok(false)
    }
}
