use std::fs;
use std::path::Path;

pub fn dir_move_exact(source: &Path, dest: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        return Ok(());
    }

    if dest.exists() {
        // Clear destination except protected files
        if dest.is_dir() {
            if let Ok(entries) = fs::read_dir(dest) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    // Protected File Exclusion Guard
                    if name_str == "CHANGELOG.md" || name_str == ".gitignore" || name_str == "README.md" {
                        continue;
                    }
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        fs::remove_dir_all(entry.path())?;
                    } else {
                        fs::remove_file(entry.path())?;
                    }
                }
            }
        }
    } else {
        fs::create_dir_all(dest)?;
    }

    // Now copy everything from source to dest
    copy_dir_recursive(source, dest)?;
    fs::remove_dir_all(source)?;

    Ok(())
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
