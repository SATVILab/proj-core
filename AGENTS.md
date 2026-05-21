# Agent Instructions

The `_reference` directory and its contents are purely for reference purposes. Agents should read them to understand how the original `projr` package worked, but MUST NOT modify, manipulate, delete, or rename any files or folders inside the `_reference` directory.

We ultimately want to have this project wrapped as Python and R language wrappers. CRAN is quite strict, so we want to be sure that our Rust binary compiles on their environments. Please use more solid and older crates if we don't strictly need the features of extremely new ones, so that the code will compile on older systems like Debian Stable or RHEL.
