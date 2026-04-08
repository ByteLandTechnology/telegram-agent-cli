---
name: "telegram-agent-cli"
description: "Telegram automation CLI with structured output, runtime-state inspection, interactive REPL, scenario testing, and MCP server for AI agents"
---

# telegram-agent-cli

## Description

Telegram automation CLI with structured output, runtime-state inspection, interactive REPL, scenario testing, and MCP server for AI agents

This skill reuses the approved description contract across Cargo metadata,
`SKILL.md`, README, and help text.

## Prerequisites

- Install `telegram-agent-cli` so it is available on `PATH` for the skill runtime.
- A Telegram API ID and API hash from https://my.telegram.org (for user accounts).
- A bot token from BotFather (for bot accounts).
- A Rust toolchain (`rustup`, `cargo`) is only needed when developing from source.

## Installation for Skill Runtime

Prefer a package-manager install that exposes the bare `telegram-agent-cli` command:

```sh
npm install -g telegram-agent-cli
pnpm add -g telegram-agent-cli
bun install -g telegram-agent-cli
telegram-agent-cli --help
```

If global installs are unavailable, use a one-shot package execution fallback:

```sh
npm exec --package=telegram-agent-cli -- telegram-agent-cli --help
pnpm dlx telegram-agent-cli --help
bunx telegram-agent-cli --help
```

The npm wrapper downloads a matching prebuilt binary during install or on first
run. After installation, the canonical skill interface remains the bare
`telegram-agent-cli` command.

## Invocation

```text
telegram-agent-cli [OPTIONS] <COMMAND>
telegram-agent-cli help [COMMAND_PATH ...] [--format yaml|json|toml]
telegram-agent-cli paths
telegram-agent-cli context show [--as <ACCOUNT>]
telegram-agent-cli account <COMMAND>
telegram-agent-cli send --as <ACCOUNT> --to <PEER> --text <TEXT>
telegram-agent-cli message <COMMAND>
telegram-agent-cli repl --as <ACCOUNT> --chat <PEER>
telegram-agent-cli run <SCENARIO_PATH>
telegram-agent-cli mcp
```

The canonical agent-facing contract uses the bare command name shown above.
`cargo run -- ...` and `./target/release/telegram-agent-cli ...` are developer-only
fallbacks for source checkouts and should not be treated as the final installed
skill interface.

### Global Options

| Flag              | Type                                              | Default | Description                                                        |
| ----------------- | ------------------------------------------------- | ------- | ------------------------------------------------------------------ |
| `--format`, `-f`  | `table` \| `yaml` \| `toml` \| `json` \| `ndjson` | `yaml`  | Structured output format for one-shot commands and structured help |
| `--help`, `-h`    | —                                                 | —       | Plain-text help only; never emits YAML/JSON/TOML                   |
| `--version`, `-V` | —                                                 | —       | Print version and exit                                             |

### Commands

| Command                  | Kind | Purpose                                                    |
| ------------------------ | ---- | ---------------------------------------------------------- |
| `help`                   | leaf | Return structured help for the requested command path      |
| `paths`                  | leaf | Inspect config, data, state, and cache directories         |
| `context show`           | leaf | Display the current Active Context and effective account   |
| `account add-user`       | leaf | Register a Telegram user account with API credentials      |
| `account add-bot`        | leaf | Register a Telegram bot account with a bot token           |
| `account list`           | leaf | List configured accounts and login state                   |
| `account use`            | leaf | Set the default account for future commands                |
| `account login`          | leaf | Authenticate an account by QR flow or credentials          |
| `account logout`         | leaf | Clear stored session for an account                        |
| `alias set`              | leaf | Bind a memorable alias to a resolved Telegram peer         |
| `alias list`             | leaf | List saved alias-to-peer mappings                          |
| `peer resolve`           | leaf | Resolve a username, alias, or ID into a Telegram peer      |
| `list contacts`          | leaf | List direct-contact peers for the selected account         |
| `list chats`             | leaf | List groups and channels for the selected account          |
| `doctor`                 | leaf | Inspect local configuration and storage state              |
| `export`                 | leaf | Export scenario run events as structured output            |
| `send`                   | leaf | Send a text message to a Telegram peer                     |
| `send-file`              | leaf | Send a file to a Telegram peer                             |
| `send-photo`             | leaf | Send a photo to a Telegram peer                            |
| `wait`                   | leaf | Wait for one matching message from a target chat           |
| `message recv`           | leaf | Read recent messages from a Telegram chat                  |
| `message follow`         | leaf | Stream new matching messages from a Telegram chat          |
| `message wait`           | leaf | Wait for one matching message (message family)             |
| `message click-button`   | leaf | Click an inline button in a bot message                    |
| `message list-actions`   | leaf | Discover inline buttons, reply keyboards, and bot commands |
| `message trigger-action` | leaf | Trigger an action discovered from list-actions             |
| `message unread`         | leaf | Show unread message statistics                             |
| `message forward`        | leaf | Forward messages between chats                             |
| `message edit`           | leaf | Edit an existing message's text                            |
| `message pin`            | leaf | Pin a message in a chat                                    |
| `message unpin`          | leaf | Unpin a pinned message                                     |
| `message download`       | leaf | Download media from a message                              |
| `bot set-commands`       | leaf | Set the bot command menu shown in chat                     |
| `bot set-info`           | leaf | Set the bot description and about text                     |
| `run`                    | leaf | Execute a scripted test scenario file                      |
| `repl`                   | leaf | Open an interactive REPL session for testing               |
| `mcp`                    | leaf | Start as an MCP tool server for AI agents over stdio       |

## Input

- Most commands do not read from `stdin` in one-shot mode.
- `repl` opens an interactive readline session on `stdin`.
- `mcp` reads JSON-RPC requests from `stdin` and writes responses to `stdout`.
- Account selection uses `--as <NAME>` (default: `"default"`, which resolves to
  the persisted default account).
- Peer targets accept aliases, usernames (`@user`), or numeric IDs.
- Sensitive credentials (tokens, sessions, passwords) can be provided via
  environment variables (`--token-env`, `--session-env`, `--code-env`,
  `--password-env`).

## Output

Standard command results are written to `stdout`. Errors and diagnostics are
written to `stderr`.

### Help Channels

- `--help` is the plain-text help channel. It always prints text and exits `0`.
- `help` is the structured help channel. It supports `yaml`, `json`, `toml`,
  and `table`, with YAML as the default.
- Top-level invocation displays plain-text help automatically and exits `0`.

### Structured Results

The default one-shot result format is YAML. Every result is wrapped in a
`ResultEnvelope` with `command`, `status`, `summary`, `data`, `next_steps`,
and `errors` fields.

Example `send` result:

```yaml
command: telegram-agent-cli send
status: ok
summary: Message sent
data:
  message_id: 42
  peer: qa-bot
next_steps:
  - action: inspect_reply
    command: telegram-agent-cli message recv --as alice --chat qa-bot --limit 1
errors: []
```

Example `paths` result:

```yaml
command: telegram-agent-cli paths
status: ok
summary: Runtime paths
data:
  config_dir: /home/user/.config/telegram-cli
  data_dir: /home/user/.local/share/telegram-cli
  state_dir: /home/user/.local/share/telegram-cli/state
  cache_dir: /home/user/.cache/telegram-cli
next_steps: []
errors: []
```

### Streaming

`message follow` produces a sequence of `StreamEventEnvelope` records, each with
`command`, `event`, `sequence`, `status`, `summary`, `data`, `next_steps`, and
`final` fields. NDJSON is the recommended format for streaming consumers:

```sh
telegram-agent-cli message follow --as alice --chat qa-bot --format ndjson
```

### Active Context

Most commands include an `active_context` block showing the persisted and
effective account:

```yaml
context:
  persisted_context: alice
  effective_context: alice
  override_applied: false
  mutation_path: "telegram-agent-cli account use <name>"
  requires_context: true
```

- `--as <NAME>` overrides the persisted context for one invocation.
- `account use <NAME>` persists the default account.
- `context show` inspects the current Active Context without side effects.

### REPL Mode

`repl` opens an interactive session. REPL help remains plain text only and the
default session view prioritizes readability over raw YAML.

```sh
telegram-agent-cli repl --as alice --chat qa-bot
```

Inside the REPL, use `/help` to discover available slash commands.

### MCP Server

`mcp` starts telegram-agent-cli as a Model Context Protocol tool server over stdio,
exposing Telegram operations as tools for AI agents:

```sh
telegram-agent-cli mcp
```

Configure in Claude Desktop or other MCP clients:

```json
{ "command": "telegram-agent-cli", "args": ["mcp"] }
```

## Errors

| Exit Code | Meaning                              |
| --------- | ------------------------------------ |
| `0`       | Success or plain-text help           |
| `1`       | Unexpected runtime failure           |
| `2`       | Structured usage or validation error |

Structured errors preserve the selected output format and include at least
stable `code` and `message` fields. Sensitive fields (tokens, sessions,
passwords, API hashes) are automatically redacted from output.

Example structured error (`--format json`):

```json
{
  "command": "telegram-agent-cli send",
  "status": "error",
  "summary": "Command failed.",
  "data": null,
  "next_steps": [
    { "action": "inspect_help", "command": "telegram-agent-cli help send" }
  ],
  "errors": [
    { "code": "message_error", "message": "account alice was not found" }
  ]
}
```

## Examples

Plain-text discovery:

```text
$ telegram-agent-cli
NAME
  telegram-agent-cli - Telegram CLI for automation and bot testing
```

Structured help:

```text
$ telegram-agent-cli help send --format yaml
```

Send a message:

```text
$ telegram-agent-cli send --as alice --to @user --text "hello"
```

Wait for a bot response:

```text
$ telegram-agent-cli wait --as alice --chat qa-bot --text-contains Welcome --timeout 5s
```

Run a test scenario:

```text
$ telegram-agent-cli run fixtures/scenarios/echo.yaml
```

Export scenario results:

```text
$ telegram-agent-cli export --run-id latest --format ndjson
```

Interactive REPL:

```text
$ telegram-agent-cli repl --as alice --chat qa-bot
```

MCP server for AI agents:

```text
$ telegram-agent-cli mcp
```

---

_Created: 2026-04-02_
