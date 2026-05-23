1. **Modernize Data Structures in `src/build.rs`**
   - Update `resolved_files`, `validation_files`, `raw_scripts`, `resolved_hooks` to entirely use `camino::Utf8PathBuf` or `Vec<camino::Utf8PathBuf>`.
   - Ensure these vectors collect `Utf8PathBuf` throughout their creation and resolution logic.
   - For `src/build.rs`, adapt standard filesystem library interactions (e.g. `fs::read_dir`, `Command::current_dir`, etc.) using minimal boundary conversions (e.g., `.as_std_path()`). Annotate these adapter boundaries with `// TODO: migrate to camino` if they wrap un-migrated external API or raw system command interaction (the instructions state to do this for standard library as well when crossing boundary). Wait, the instruction says: `// TODO: migrate to camino` tracking comment for an un-migrated external API or raw system command interaction. But also mentions "where src/build.rs hooks into external or standard-library steps that still require standard paths, apply local adapters".

2. **Modernize `src/build_pre.rs` APIs and Logic**
   - Change `pre_flight_git_check` signature to: `pub fn pre_flight_git_check(config: &GlobalConfig, repo_dir: &camino::Utf8Path) -> anyhow::Result<()>`.
   - Change `run_pre_flight_checks` signature to take `resolved_files: &[camino::Utf8PathBuf]` and return `anyhow::Result<(Option<String>, Option<String>)>`.
   - Update `is_behind_remote` and any other helper functions to return `anyhow::Result<T>`.
   - Inside `build_pre.rs` function bodies, convert any `fs::read_dir` or other path creation into `Utf8PathBuf` as soon as possible via `try_from`, failing on invalid UTF-8 with a sentence case context like `anyhow!("Non-UTF-8 path encountered")`.
   - Convert all error returns to use `anyhow::bail!` and `.context()` with "Sentence case" context. Drop `String` error mappings.

3. **Align `src/build.rs` Errors**
   - Convert `post_build_sync` to return `anyhow::Result<()>`.
   - Convert `build_project` to return `anyhow::Result<()>`.
   - Convert helper functions in `src/build.rs` (like `resolve_script_extensions`, `check_explicit_missing`, etc.) to return `anyhow::Result<T>`.
   - Update `Result<(), String>` to `anyhow::Result<()>` throughout `src/build.rs`. Replace `.map_err(|e| e.to_string())` with `?` or `.context(...)` using Sentence case context.

4. **Verify Pre-commit Steps**
   - Call `pre_commit_instructions` tool to make sure proper testing, verifications, reviews and reflections are done.

5. **Submit Change**
   - Submit via `submit` tool.
