1. **Create `src/build_pre.rs` module:**
   - Define a function `run_pre_flight_checks(project_root: &Path, build_scripts: Option<&Vec<String>>, quarto_exists: bool, bookdown_exists: bool) -> Result<Option<String>, String>`
   - The function returns `Option<String>` which contains the determined python command (`Some(String)`) to use if python is needed, or `None` if python is not needed.
   - **Determine targets:**
     - If `build_scripts` is `Some`: Evaluate scripts via `resolve_explicit_scripts`. Check for `.Rmd`, `.qmd`, `.py`.
     - If `build_scripts` is `None`: Evaluate fallback via `resolve_fallback_scripts`. Check for `.Rmd`, `.qmd`, `.py`. (We will need to expose/call these resolution functions or replicate the logic to find what engines are actually needed).
       Wait, actually, the user says:
       "During build_pre, the utility must scan the final project configuration (e.g., _proj.yml or the specific file types being passed to the build command) to determine which dependency checks to activate:"
     - R needed: if `.Rmd`, `.rmd`, `bookdown`, `rmarkdown` detected, or if bookdown config exists.
     - Quarto needed: if `.qmd`, or `_quarto.yml` (quarto_project/quarto_document).
     - Python needed: if `.py`.
   - **R Checks:**
     - Check `Rscript --version`
     - Check namespace: `Rscript -e "if (!requireNamespace('{}', quietly = TRUE)) q(status = 1)"` (for `bookdown` and `rmarkdown` if those configs are present or `.Rmd` is present).
     - If missing namespace: remediate based on `PROJR_AUTO_INSTALL` or `std::io::stdin().is_terminal()`. Prompt to install, and if yes/true, run `install.packages()`.
   - **Quarto Checks:**
     - Check `quarto --version`. Error if missing.
   - **Python Checks:**
     - Find python using `find_python_command() -> Option<String>`. Error if missing.
2. **Update `src/lib.rs`:**
   - Add `pub mod build_pre;` and `pub use build_pre::*;`
3. **Update `src/build.rs`:**
   - Call `run_pre_flight_checks` right after resolving `quarto_exists` and `bookdown_exists` and `build_scripts` but BEFORE git commits or any writes.
   - `let resolved_python_cmd = run_pre_flight_checks(project_root, build_scripts.as_ref(), quarto_exists, bookdown_exists)?;`
   - Modify `execute_script` to accept the resolved python command: `fn execute_script(script_path: &Path, profile: Option<&str>, python_cmd: Option<&str>)`.
   - Update `execute_build_pipeline` and references to `execute_script` to pass the python command.
4. **Pre-commit and Test:**
   - Run tests, compile checks, and check git status.
5. **Submit:** Commit and submit.
