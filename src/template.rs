use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use glob::Pattern;
use indexmap::IndexMap;
use minijinja::{Environment, Value};
use tempfile::TempDir;
use toml::Table;
use walkdir::{DirEntry, WalkDir};

use crate::metadata::Metadata;
use crate::utils::{self, Error, Result};
use crate::variable::Variable;

pub struct Template {
    pub path: PathBuf,
    pub root: PathBuf,
    pub base: PathBuf,

    pub metadata: Metadata,
    pub variables: IndexMap<String, Variable>,

    pub entries: BTreeMap<usize, Vec<DirEntry>>,
    pub environment: Environment<'static>,
}

impl Template {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = fs::canonicalize(&path)?;
        let contents = fs::read_to_string(&path)?;

        let metadata = toml::from_str::<Metadata>(&contents)?;

        let table = contents.parse::<Table>()?;
        let mut variables = IndexMap::new();
        for (name, value) in table {
            if !(name.starts_with("__") && name.ends_with("__")) {
                let variable = value.try_into::<Variable>()?;
                let variable = variable.validate().map_err(|err| Error::ValidateVariable {
                    name: name.clone(),
                    source: err,
                })?;
                variables.insert(name, variable);
            }
        }

        let root = path.parent().unwrap().to_path_buf();
        let base = root
            .join(&metadata.base)
            .canonicalize()
            .map_err(Error::UncanonicalizableBasePath)?;

        let entries = BTreeMap::new();
        let mut environment = Environment::new();
        environment.add_function("year", || String::from("2023")); // DEBUG

        Self {
            path,
            root,
            base,
            metadata,
            variables,
            entries,
            environment,
        }
        .init()
    }

    fn init(mut self) -> Result<Self> {
        let walker = WalkDir::new(&self.base).sort_by_file_name();
        for entry in walker {
            let entry = entry.map_err(|err| err.into_io_error().unwrap())?;
            let path = entry.path().strip_prefix(&self.root).unwrap();
            if self.metadata.exclude.matches_path_any(path) {
                continue;
            }
            if entry.file_type().is_file() {
                let buf = fs::read(entry.path())?;
                let name = utils::path_to_string(path);
                if utils::is_binary_buf(&buf) {
                    self.metadata.copy.push(Pattern::new(&name).unwrap())
                } else if !self.metadata.copy.matches_path_any(path) {
                    let source = String::from_utf8(buf).expect("file encoding should be utf-8");
                    self.environment.add_template_owned(name, source)?;
                }
            }
            let depth = path.components().count();
            self.entries.entry(depth).or_default().push(entry);
        }
        Ok(self)
    }

    fn render_path(
        &self,
        path: impl AsRef<Path>,
        values: &HashMap<String, Value>,
    ) -> Result<String, minijinja::Error> {
        let source = utils::path_to_string(path);
        let source = source.escape_default().collect::<String>();
        self.environment.render_str(&source, values)
    }

    fn render_template(
        &self,
        name: impl AsRef<Path>,
        dst: impl AsRef<Path>,
        values: &HashMap<String, Value>,
    ) -> Result<()> {
        let name = utils::path_to_string(name);
        let template = self.environment.get_template(&name)?;
        let file = File::create(dst)?;
        template.render_to_write(values, file)?;
        Ok(())
    }

    pub fn generate(&self, values: &HashMap<String, Value>) -> Result<Output> {
        let mut basename: Option<String> = None;
        let tempdir = TempDir::with_prefix("tapgen-")?;
        for entry in self.entries.values().flatten() {
            let raw_name = entry.path().strip_prefix(&self.root).unwrap();
            let rendered_name = self.render_path(raw_name, values)?;
            let rendered_path = tempdir.path().join(&rendered_name);
            if entry.path() == self.base {
                basename = Some(rendered_name);
            }
            if entry.file_type().is_file() {
                if self.metadata.copy.matches_path_any(entry.path()) {
                    fs::copy(entry.path(), rendered_path)?;
                } else {
                    self.render_template(raw_name, rendered_path, values)?;
                }
            } else if entry.file_type().is_dir() {
                fs::create_dir_all(rendered_path)?;
            }
        }
        Ok(Output {
            tempdir,
            basename: basename.expect("basename should be determined"),
        })
    }
}

pub struct Output {
    tempdir: TempDir,
    basename: String,
}

impl Output {
    pub fn basename(&self) -> &str {
        &self.basename
    }

    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }

    pub fn base(&self) -> PathBuf {
        self.tempdir.path().join(self.basename())
    }

    pub fn dispose(self) -> Result<()> {
        Ok(self.tempdir.close()?)
    }

    pub fn apply(self, dst: impl AsRef<Path>) -> Result<()> {
        utils::copy_dir_all(self.path(), dst)?;
        self.dispose()
    }
}
