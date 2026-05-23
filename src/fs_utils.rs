use std::fs;
use camino::Utf8Path;

pub fn dir_move_exact(source: &Utf8Path, dest: &Utf8Path) -> anyhow::Result<()> {
    if !source.exists() {
        return Ok(());
    }

    if dest.exists() {
        // Clear destination except protected files
        if dest.is_dir() {
            if let Ok(entries) = fs::read_dir(dest.as_std_path()) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    let entry_path_utf8 = camino::Utf8PathBuf::try_from(entry_path)
                        .map_err(|e| anyhow::anyhow!("Non-UTF-8 path: {}", e))?;

                    let name_str = entry_path_utf8.file_name().unwrap_or("");
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
        fs::create_dir_all(dest.as_std_path())?;
    }

    // Now copy everything from source to dest
    copy_dir_recursive(source, dest)?;
    fs::remove_dir_all(source.as_std_path())?;

    Ok(())
}

pub fn copy_dir_recursive(src: &Utf8Path, dst: &Utf8Path) -> anyhow::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if !dst.exists() {
        fs::create_dir_all(dst.as_std_path())?;
    }

    for entry in fs::read_dir(src.as_std_path())? {
        let entry = entry?;
        let ty = entry.file_type()?;

        let entry_path = entry.path();
        let entry_path_utf8 = camino::Utf8PathBuf::try_from(entry_path)
            .map_err(|e| anyhow::anyhow!("Non-UTF-8 path: {}", e))?;

        let dest_path = dst.join(entry_path_utf8.file_name().unwrap());

        if ty.is_dir() {
            copy_dir_recursive(&entry_path_utf8, &dest_path)?;
        } else {
            fs::copy(entry_path_utf8.as_std_path(), dest_path.as_std_path())?;
        }
    }
    Ok(())
}
