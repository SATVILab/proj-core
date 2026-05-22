1. **Target Files for Documentation Update**:
   - `docs/configuration.qmd`
   - `docs/cli/yml.qmd`
   - `docs/flow.qmd`

2. **`docs/configuration.qmd` Update**:
   - Add a new section under `## Top-Level Configuration` or as its own section to describe the new `parameters` block.
   - Mention that `_proj.yml` supports a `parameters` configuration block for arbitrary nested parameter evaluation.
   - Mention the supported aliases: `parameter`, `params`, `param`.
   - Mention that multiple alias conflicts within the same configuration file are structurally rejected during validation.

3. **`docs/cli/yml.qmd` Update**:
   - Under the `## Reading Parameters and Defaults` section, update point 1 or add a new bullet to clarify that the command dynamically resolves explicitly defined variables as well as arbitrary deeply nested `parameters` blocks defined in the file.
   - State that it protects against `parameters` alias conflicts.
   - Mention that an empty `parameters` block is automatically injected into the file if missing during a write/init process (based on `add_empty_parameters_block` logic seen in the PR).

4. **`docs/flow.qmd` Update**:
   - Under `## 1. Pre-Build Stage` -> `### 1.1 Config Audit`, add a note about the new validation logic: The configuration audit strictly evaluates `parameters` (and its aliases `parameter`, `params`, `param`) and throws a validation error if multiple aliases are found to prevent mapping conflicts.

5. **Execute Updates using sed or overwrite**:
   - Create a git merge diff to apply these changes accurately.

