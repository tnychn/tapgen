// ===== ROADMAP =====
//
// DONE: load git config into context
//
// DONE: implement hook commands
//
// TODO: implement git checkout after clone
//
// TODO: implement git pull for outdated template
//
// TODO: implement cli variable value override
//
// TODO: implement remote subdirectory template
//
// TODO: implement custom base path shorthand
//
// TODO: implement generation replay
//
// TODO: implement templated defaults
//
// TODO: implement extensible custom template functions
//
// TODO: conditional generation of file/directory

mod config;
mod generate;
mod git;
mod prompt;

use std::fs;

use anyhow::Result;
use clap::{ArgAction, Parser};

use crate::config::Config;
use crate::generate::Generate;

#[derive(Parser)]
#[command(version)]
#[command(verbatim_doc_comment)]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(arg_required_else_help = true)]
#[command(about = "Tony's Almighty Project Generator")]
#[command(author = "Tony Chan <tnychn@protonmail.com>")]
struct Cli {
    #[command(flatten)]
    generate: Generate,

    #[arg(
        short = 'h',
        long = "help",
        help = "Print this help message.",
        action = ArgAction::Help,
    )]
    help: Option<bool>,

    #[arg(
        short = 'V',
        long = "version",
        help = "Print version information.",
        action = ArgAction::Version,
    )]
    version: Option<bool>,
}

pub(crate) struct App {
    cli: Cli,
    config: Config,
}

impl App {
    fn init() -> Self {
        let cli = Cli::parse();
        let config = Config::init().expect("failed to initialize config");
        fs::create_dir_all(&config.prefix).expect("failed to create prefix directory");
        Self { cli, config }
    }
}

fn main() -> Result<()> {
    App::init().generate()
}
