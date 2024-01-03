use std::{fs, path::Path};

use anyhow::{Context as _, Result};

use crate::prompt;

pub(crate) fn copy_dir_all(
    dstroot: impl AsRef<Path>,
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    force: bool,
) -> Result<(u32, u32, u32)> {
    let (mut creates, mut overwrites, mut skips) = (0, 0, 0);
    fs::create_dir_all(&dst).context(format!(
        "failed to create destination directory: '{}'",
        dst.as_ref().display()
    ))?;
    for entry in fs::read_dir(&src).context(format!(
        "failed to read source directory: '{}'",
        src.as_ref().display()
    ))? {
        let entry = entry.unwrap();
        let to = dst.as_ref().join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            let (c, o, s) = copy_dir_all(dstroot.as_ref(), entry.path(), to, force)?;
            creates += c;
            overwrites += o;
            skips += s;
        } else {
            if to.exists() {
                if force
                    || prompt::confirm(
                        format!(
                            "Overwrite '{}'?",
                            to.strip_prefix(dstroot.as_ref()).unwrap().display()
                        ),
                        None,
                    )
                {
                    overwrites += 1;
                } else {
                    skips += 1;
                }
            } else {
                creates += 1;
            }
            fs::copy(entry.path(), to)
                .context(format!("failed to copy file: '{}'", entry.path().display()))?;
        }
    }
    Ok((creates, overwrites, skips))
}
