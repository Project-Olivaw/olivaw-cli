//! Project scaffolds for `olivaw init`, embedded with `include_dir!`.
//!
//! Templates are real, compiling projects. The only substitution is the
//! literal token `{{project_name}}` — anything needing more templating than
//! that is designed wrong (per CLAUDE.md).

use std::borrow::Cow;

use anyhow::Context;

use crate::plan::{FileWrite, Plan, WriteKind};
use crate::project::RelPath;

static TEMPLATES: include_dir::Dir<'static> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Target {
    Esp32,
    Rp2040,
    Linux,
}

impl Target {
    pub fn as_str(self) -> &'static str {
        match self {
            Target::Esp32 => "esp32",
            Target::Rp2040 => "rp2040",
            Target::Linux => "linux",
        }
    }
}

/// Build the scaffold plan for `name` at `target`: every file under
/// `templates/common/` plus `templates/<target>/`, with `{{project_name}}`
/// substituted. Manifest bookkeeping (olivaw.toml) is handled by the caller.
pub fn scaffold_plan(name: &str, target: Target) -> anyhow::Result<Plan> {
    let mut plan = Plan::new();
    for dir_name in ["common", target.as_str()] {
        let dir = TEMPLATES
            .get_dir(dir_name)
            .with_context(|| format!("missing embedded template dir '{dir_name}' — this is a build bug in olivaw"))?;
        collect_files(dir, dir_name, name, &mut plan)?;
    }
    Ok(plan)
}

fn collect_files(
    dir: &include_dir::Dir<'static>,
    strip_prefix: &str,
    project_name: &str,
    plan: &mut Plan,
) -> anyhow::Result<()> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(sub) => {
                collect_files(sub, strip_prefix, project_name, plan)?;
            }
            include_dir::DirEntry::File(file) => {
                let rel = file
                    .path()
                    .strip_prefix(strip_prefix)
                    .context("template path outside its root")?
                    .to_str()
                    .context("template path is not UTF-8")?;
                // "gitignore" is stored unhidden so it survives packaging;
                // restore the dot on the way out.
                let rel = if rel == "gitignore" { ".gitignore" } else { rel };
                let dest = RelPath::new(rel)
                    .with_context(|| format!("invalid template path '{rel}'"))?;
                let contents: Cow<'static, [u8]> = match std::str::from_utf8(file.contents()) {
                    Ok(text) if text.contains("{{project_name}}") => {
                        Cow::Owned(text.replace("{{project_name}}", project_name).into_bytes())
                    }
                    _ => Cow::Borrowed(file.contents()),
                };
                plan.file_writes.push(FileWrite {
                    dest,
                    contents,
                    kind: WriteKind::Create,
                });
            }
        }
    }
    Ok(())
}
