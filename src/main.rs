//! `olivaw` — vendored, editable Rust components for robotics.
//!
//! Each command copies real, working source code into the user's project.
//! It never adds a crate dependency and never writes outside the project
//! directory (the only sanctioned exception is the registry cache under
//! `~/.olivaw/cache`).

mod checksum;
mod commands;
mod plan;
mod project;
mod registry;
mod resolve;
mod suggest;
mod templates;
mod ui;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::templates::Target;
use crate::ui::Ui;

#[derive(Parser)]
#[command(
    name = "olivaw",
    version,
    about = "Vendored, editable Rust components for robotics",
    long_about = "Vendors robotics components (sensors, drivers, kinematics, SLAM) \
                  into your Rust project as source code you own — like shadcn/ui, \
                  not a package manager."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// Never touch the network; use the cached or embedded registry.
    #[arg(long, global = true)]
    offline: bool,

    /// Fetch the registry at this git tag instead of the built-in default.
    #[arg(long, global = true, value_name = "TAG", env = "OLIVAW_REGISTRY_TAG")]
    registry_tag: Option<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scaffold a new robotics project
    Init {
        /// Project name; creates ./<name>/ (defaults to scaffolding into the
        /// current directory, named after it)
        #[arg(long)]
        name: Option<String>,
        /// Hardware target for the scaffold
        #[arg(long, value_enum)]
        target: Target,
        /// Overwrite scaffold files that already exist
        #[arg(long)]
        force: bool,
    },
    /// Vendor a component into the project
    Add {
        /// Component path, e.g. "sensors/mpu6050"
        component: String,
        /// Directory (relative to the project root) to prefix destinations with
        #[arg(long, value_name = "DIR")]
        path: Option<String>,
        /// Overwrite existing files and skip confirmation prompts
        #[arg(long)]
        force: bool,
    },
    /// Show available components
    List {
        /// Only show this category, e.g. "sensors"
        category: Option<String>,
    },
    /// Show a component's description, dependencies and hardware notes
    Info {
        /// Component path, e.g. "sensors/mpu6050"
        component: String,
    },
    /// Re-fetch a component. Diffs against local state first and refuses to
    /// clobber your edits without --force.
    Update {
        /// Component path, e.g. "sensors/mpu6050"
        component: String,
        /// Overwrite locally modified files without prompting
        #[arg(long)]
        force: bool,
        /// Show what would change without writing anything
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify installed components against olivaw.toml; report drift
    Check {
        /// Print nothing when everything is clean
        #[arg(long)]
        quiet: bool,
    },
}

/// Registry-loading options derived from global flags.
#[derive(Clone, Debug)]
pub struct RegistryOpts {
    pub offline: bool,
    pub tag_override: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let ui = Ui::new();
    let reg_opts = RegistryOpts {
        offline: cli.offline,
        tag_override: cli.registry_tag.clone(),
    };

    let result = match cli.cmd {
        Cmd::Init { name, target, force } => {
            commands::init::run(&ui, name.as_deref(), target, force)
        }
        Cmd::Add {
            component,
            path,
            force,
        } => commands::add::run(&ui, &reg_opts, &component, path.as_deref(), force),
        Cmd::List { category } => commands::list::run(&ui, &reg_opts, category.as_deref()),
        Cmd::Info { component } => commands::info::run(&ui, &reg_opts, &component),
        Cmd::Update {
            component,
            force,
            dry_run,
        } => commands::update::run(&ui, &reg_opts, &component, force, dry_run),
        Cmd::Check { quiet } => commands::check::run(&ui, quiet),
    };

    match result {
        Ok(code) => code,
        Err(err) => {
            // {:#} renders the whole anyhow context chain on one line.
            eprintln!("{}", ui.error_line(&format!("{err:#}")));
            ExitCode::FAILURE
        }
    }
}
