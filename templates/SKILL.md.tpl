---
name: '{{SKILL_NAME}}'
description: '{{DESCRIPTION}}'
---

# {{SKILL_NAME}}

## Description

{{DESCRIPTION}}

This generated Skill reuses the approved description contract across Cargo
metadata, `SKILL.md`, README, help text, and the repository's GitHub Release
surface.

## Prerequisites

- A working Rust toolchain (`rustup`, `cargo`) to compile and test the binary.
- No additional system dependencies are required for the default scaffold.
- The optional REPL feature adds interactive history and completion using the
  Rust crate ecosystem only.
- Cloned release installs use the repository's `scripts/install-current-release.sh`
  helper to fetch the matching GitHub Release binary for the checked out
  version.

## Invocation

```text
{{SKILL_NAME}} [OPTIONS] <COMMAND>
{{SKILL_NAME}} help [COMMAND_PATH ...] [--format yaml|json|toml]
{{SKILL_NAME}} run [OPTIONS] <INPUT>
{{SKILL_NAME}} paths [OPTIONS]
{{SKILL_NAME}} context <show|use> [OPTIONS]
```

The canonical agent-facing contract uses the bare command name shown above.
`cargo run -- ...` and `./target/release/{{SKILL_NAME}} ...` are local
developer execution forms and should be documented in `README.md`, not treated
as the final installed skill interface.

### Global Options

| Flag              | Type                       | Default                   | Description                                                        |
| ----------------- | -------------------------- | ------------------------- | ------------------------------------------------------------------ |
| `--format`, `-f`  | `yaml` \| `json` \| `toml` | `yaml`                    | Structured output format for one-shot commands and structured help |
| `--help`, `-h`    | —                          | —                         | Plain-text help only; never emits YAML/JSON/TOML                   |
| `--config-dir`    | `PATH`                     | platform default          | Override the configuration directory                               |
| `--data-dir`      | `PATH`                     | platform default          | Override the durable data directory                                |
| `--state-dir`     | `PATH`                     | derived from data         | Override the runtime state directory                               |
| `--cache-dir`     | `PATH`                     | platform default          | Override the cache directory                                       |
| `--log-dir`       | `PATH`                     | `state/logs` when enabled | Override the optional log directory                                |
| `--version`, `-V` | —                          | —                         | Print version and exit                                             |

### Commands

| Command        | Kind | Purpose                                                      |
| -------------- | ---- | ------------------------------------------------------------ |
| `help`         | leaf | Return structured help for the requested command path        |
| `run`          | leaf | Execute the generated leaf command                           |
| `paths`        | leaf | Inspect config/data/state/cache and optional log directories |
| `context show` | leaf | Display the current Active Context and effective values      |
| `context use`  | leaf | Persist selectors or ambient cues as the Active Context      |

## Input

- The scaffolded CLI does not read default-mode input from `stdin`.
- `run` requires one positional `<INPUT>` argument.
- `context use` accepts one or more `--selector KEY=VALUE` flags and may also
  accept `--cwd PATH` as an ambient cue.
- `help` accepts an optional command path such as `run` or `context use`.

## Output

Standard command results are written to `stdout`. Errors and diagnostics are
written to `stderr`.

### Help Channels

- `--help` is the plain-text help channel. It always prints text and exits `0`.
- `help` is the structured help channel. It supports `yaml`, `json`, and
  `toml`, with YAML as the default.
- Top-level invocation and non-leaf invocation (for example `context`) display
  plain-text help automatically and exit `0`.

### Structured Results

The default one-shot result format is YAML.

Example `run` result:

```yaml
status: ok
message: Hello from {{SKILL_NAME}}
input: demo-input
effective_context:
  workspace: demo
```

Example `paths` result:

```yaml
config_dir: /home/user/.config/{{SKILL_NAME}}
data_dir: /home/user/.local/share/{{SKILL_NAME}}
state_dir: /home/user/.local/share/{{SKILL_NAME}}/state
cache_dir: /home/user/.cache/{{SKILL_NAME}}
scope: user_scoped_default
override_mechanisms:
  - --config-dir
  - --data-dir
  - --state-dir
  - --cache-dir
  - --log-dir
```

### Runtime Directories and Active Context

- `paths` exposes the runtime directory family: `config`, `data`, `state`,
  `cache`, and optional `logs`.
- Defaults are user-scoped unless explicitly overridden.
- `context show` exposes the persisted and effective Active Context.
- Explicit per-invocation selectors on `run` override the persisted Active
  Context for that invocation only.

### Release Install Surface

- Repository releases publish version-matched archives to GitHub Releases.
- `scripts/install-current-release.sh` installs the archive for the checked out
  release tag instead of following an unrelated latest pointer.
- The repository also publishes `release-evidence.json`, which links the repo
  version, git tag, release page, archive filenames, and checksums.

### Optional Features

- Streaming may be added later with `--stream`.
- REPL mode may be added later with `--repl`. When enabled, REPL help remains
  plain text only and the default REPL presentation is human-oriented.
- Package-local packaging-ready metadata or support fixtures may be added by a
  supported capability later, but repository-owned CI workflows and release
  automation are not copied into generated skill packages by default. If the
  target project later adopts the `cli-forge-publish` release asset pack,
  those files belong at repository root rather than inside the shipped skill
  package.

## Errors

| Exit Code | Meaning                              |
| --------- | ------------------------------------ |
| `0`       | Success or plain-text help           |
| `1`       | Unexpected runtime failure           |
| `2`       | Structured usage or validation error |

Structured errors preserve the selected output format and include at least
stable `code` and `message` fields.

Example structured error (`--format json`):

```json
{
  "code": "run.missing_input",
  "message": "the run command requires <INPUT>; use --help for plain-text help",
  "source": "leaf_validation",
  "format": "json"
}
```

## Examples

Plain-text discovery:

```text
$ {{SKILL_NAME}}
NAME
  {{SKILL_NAME}} - {{DESCRIPTION}}
```

Structured help:

```text
$ {{SKILL_NAME}} help run --format yaml
```

Install the matching released binary from a cloned checkout:

```text
$ git checkout v{{VERSION}}
$ ./scripts/install-current-release.sh {{VERSION}}
```

Persist Active Context:

```text
$ {{SKILL_NAME}} context use --selector workspace=demo --selector provider=staging
```

Run with one explicit override:

```text
$ {{SKILL_NAME}} run demo-input --selector provider=preview
```

---

_Created: {{CURRENT_DATE}}_
