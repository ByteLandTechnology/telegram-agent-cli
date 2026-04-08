[package]
name = "{{SKILL_NAME}}"
version = "{{VERSION}}"
edition = "{{RUST_EDITION}}"
description = "{{DESCRIPTION}}"
authors = ["{{AUTHOR}}"]

# The package description is the canonical one-line purpose summary approved by
# the cli-forge description stage. Keep README/help/SKILL.md aligned with it.

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
anyhow = "1"
directories = "5"
rustyline = "14"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3"
