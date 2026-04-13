# telegram-agent-cli

Telegram automation CLI for testing bots, running scenarios, and exposing
Telegram operations as MCP tools for AI agents.

Structured output in YAML (default), JSON, TOML, table, or NDJSON.
Encrypted local storage. Interactive REPL. Managed daemon mode. Scenario runner.

## Install

For skill/runtime usage, install the npm package so `telegram-agent-cli` is available
on `PATH`:

```sh
npm install -g telegram-agent-cli
pnpm add -g telegram-agent-cli
bun install -g telegram-agent-cli
telegram-agent-cli --help
```

If you cannot do a global install, use a one-shot package execution fallback:

```sh
npm exec --package=telegram-agent-cli -- telegram-agent-cli --help
pnpm dlx telegram-agent-cli --help
bunx telegram-agent-cli --help
```

Build from source only when developing locally:

```sh
cargo build --release
./target/release/telegram-agent-cli --help
```

The npm package now resolves a matching platform subpackage that already
contains the native binary for your machine. It does not download from GitHub
Releases during install or first run. Supported npm targets:

- `darwin-arm64`
- `darwin-x64`
- `linux-arm64`
- `linux-x64`
- `windows-arm64`
- `windows-x64`

If your package manager skips optional dependencies, or if you install on an
unsupported target, the wrapper package may install without a usable binary.
Reinstall with optional dependencies enabled, install the matching platform
package explicitly, or build from source with Cargo instead.

## Releases

GitHub Releases still publish versioned CLI archives and checksum files for
supported targets. In parallel, the npm release now publishes one main wrapper
package plus matching platform binary packages for supported targets:

- `telegram-agent-cli`
- `telegram-agent-cli-darwin-arm64`
- `telegram-agent-cli-darwin-x64`
- `telegram-agent-cli-linux-arm64`
- `telegram-agent-cli-linux-x64`
- `telegram-agent-cli-windows-arm64`
- `telegram-agent-cli-windows-x64`

Windows release artifacts now use the GNU LLVM target triples
`x86_64-pc-windows-gnullvm` and `aarch64-pc-windows-gnullvm`, so local Windows
install helpers and release evidence align with the packaged npm binaries.

CI now treats `semantic-release` as the single release orchestrator and the
single release pipeline. On pushes to `main`, the `Release` workflow runs on a
macOS runner, installs the cross-build toolchains once, and lets
`semantic-release` drive the rest of the flow. During the semantic-release
`prepare` phase it stamps the next version, builds all supported platform
binaries, and stages `dist/` plus `dist/npm`. During `publish` it writes
`release-evidence.json`, verifies the package set, publishes the GitHub Release
assets, publishes all platform npm packages, and finally publishes the root npm
package that references them through `optionalDependencies`.

The npm publish step is fixed to GitHub Actions Trusted Publisher (OIDC) in the
release workflow. It relies on GitHub-hosted runners plus `id-token: write`,
does not require `NPM_TOKEN`, and bootstraps npm CLI `11.5.1+` only for the
live `npm publish` calls instead of replacing the runner's bundled npm.

Once Trusted Publisher is verified for every package, npm recommends restricting
package settings to "Require two-factor authentication and disallow tokens" and
revoking old automation tokens.

For npm publication rehearsal, stamp a version and build the staged package set
first, then run:

```sh
node ./npm/prepare-release.cjs <version>
npm run release:build-all -- --version <version>
npm run release:npm:verify -- --version <version> --git-tag v<version> --staged-dist ./dist/npm
npm run release:npm:dry-run -- --version <version> --git-tag v<version> --staged-dist ./dist/npm
```

These commands keep the root package, platform packages, optional dependency
versions, and staged artifacts aligned before any live npm publish happens.

For manual artifact-only rehearsals, use the `Build Artifacts` workflow
dispatch and provide the version explicitly. It reuses the same cross-build and
staging script as the formal `Release` workflow on `main`.

The repository remains the source of truth for the skill itself. The final
shipped skill contract is the bare `telegram-agent-cli` command, while local
development may use `cargo run -- ...` and built-binary verification may use
`./target/release/telegram-agent-cli ...`.

For clone-first release installs, use a released checkout and the repo-native
install helper:

```sh
git clone https://github.com/ByteLandTechnology/telegram-agent-cli.git
cd telegram-agent-cli
git checkout v<version>
./scripts/install-current-release.sh <version>
```

Each release should keep the Git tag, GitHub Release page, binary archives,
checksum files, `release-evidence.json`, and `.release-manifest.json` aligned
to the same version.

## Quick Start

```sh
# Register a bot account
telegram-agent-cli account add-bot --name mybot --token-env BOT_TOKEN

# Authenticate
telegram-agent-cli account login mybot --qr

# Send a message
telegram-agent-cli send --as mybot --to @user --text "hello"

# Open an interactive session
telegram-agent-cli repl --as mybot --chat @user

# Start MCP server for AI agents
telegram-agent-cli mcp
```

## Output Format

All commands support `--format`:

```sh
telegram-agent-cli paths                                    # YAML (default)
telegram-agent-cli paths --format json                      # JSON
telegram-agent-cli paths --format toml                      # TOML
telegram-agent-cli message follow --as mybot --chat @user --format ndjson  # NDJSON stream
```

Structured results use a `ResultEnvelope` with `command`, `status`, `summary`,
`data`, `next_steps`, and `errors`. Sensitive fields are redacted automatically.

When you use the managed daemon lifecycle, daemon metadata is stored at
`state_dir/daemon/server.json` and daemon logs are written to
`state_dir/daemon/server.log`.

## Account Management

```sh
telegram-agent-cli account add-user --name alice --api-id 12345 --api-hash <hash>
telegram-agent-cli account add-bot --name mybot --token-env BOT_TOKEN
telegram-agent-cli account login alice --qr
telegram-agent-cli account login alice --code-env TELEGRAM_CLI_LOGIN_CODE --password-env TELEGRAM_CLI_2FA_PASSWORD
telegram-agent-cli account list
telegram-agent-cli account use mybot        # set default
telegram-agent-cli account logout mybot
```

Switch accounts per-invocation with `--as`:

```sh
telegram-agent-cli send --as alice --to @user --text "hello"
telegram-agent-cli send --as mybot --to @user --text "hello"
```

Inspect the current context:

```sh
telegram-agent-cli context show
telegram-agent-cli context show --as mybot
```

## Messaging

```sh
telegram-agent-cli send --as mybot --to @user --text "hello"
telegram-agent-cli send-file --as mybot --to @user report.pdf --caption "Q1 report"
telegram-agent-cli send-photo --as mybot --to @user screenshot.png

# With inline/reply keyboards
telegram-agent-cli send --as mybot --to @user --text "Choose:" \
  --inline-keyboard '[["Yes:callback:yes","No:callback:no"]]'
```

### Reading Messages

```sh
telegram-agent-cli message recv --as mybot --chat @user --limit 20
telegram-agent-cli message recv --as mybot --chat @user --unread-only
telegram-agent-cli message unread --as mybot --chat @user
```

### Waiting & Streaming

```sh
# Block until a matching message arrives
telegram-agent-cli wait --as mybot --chat @user --text-contains Welcome --timeout 5s

# Stream matching messages (NDJSON recommended)
telegram-agent-cli message follow --as mybot --chat @user --format ndjson \
  --text-contains response --limit 5 --timeout 30s
```

### Bot Interaction

```sh
# Discover buttons and actions
telegram-agent-cli message list-actions --as mybot --chat @user

# Click an inline button
telegram-agent-cli message click-button --as mybot --chat @user Start --wait-timeout 5s

# Trigger a discovered action
telegram-agent-cli message trigger-action --as mybot --chat @user \
  bot-command:my_bot:start --wait-timeout 5s
```

### Other Message Operations

```sh
telegram-agent-cli message forward --as mybot --from @src --to @dst --message-ids 42,43
telegram-agent-cli message edit --as mybot --chat @user --message-id 42 --text "updated"
telegram-agent-cli message pin --as mybot --chat @user --message-id 42
telegram-agent-cli message unpin --as mybot --chat @user --message-id 42
telegram-agent-cli message download --as mybot --chat @user --message-id 42 --output file.jpg
```

## Bot Configuration

```sh
telegram-agent-cli bot set-commands --as mybot \
  --commands '/start|Start the bot,/help|Show help'

telegram-agent-cli bot set-info --as mybot \
  --description "A helpful bot" --about "v1.0"
```

## Peers & Aliases

```sh
telegram-agent-cli peer resolve --as mybot @user
telegram-agent-cli alias set --as mybot qa-bot qa_bot
telegram-agent-cli alias list
telegram-agent-cli list contacts --as mybot
telegram-agent-cli list chats --as mybot
```

## Scenario Automation

Define test scenarios in YAML and replay them:

```sh
telegram-agent-cli run scenarios/echo.yaml
telegram-agent-cli export --run-id latest
telegram-agent-cli export --run-id latest --format ndjson
```

## REPL

Interactive session with a chat. History, tab completion, and plain-text
output for readability:

```sh
telegram-agent-cli repl --as mybot --chat @user
```

REPL commands: `/send`, `/recv`, `/wait`, `/actions`, `/click`, `/trigger`,
`/unread`, `/help`, `/exit`.

## MCP Server

Expose Telegram operations as tools for AI agents over stdio:

```sh
telegram-agent-cli mcp
```

Configure in Claude Desktop or other MCP clients:

```json
{ "command": "telegram-agent-cli", "args": ["mcp"] }
```

## Daemon Mode

Run the same JSON-RPC/MCP tool surface as a managed background daemon:

```sh
telegram-agent-cli daemon start
telegram-agent-cli daemon status --format json
telegram-agent-cli daemon stop
```

The daemon binds a local loopback TCP endpoint, records its runtime metadata
under the telegram-agent-cli state directory, and preserves the shared managed
contract: `daemon start`, `daemon stop`, `daemon restart`, and
`daemon status`. Foreground attached execution remains out of scope; use
`telegram-agent-cli mcp` when you want a stdio-bound server in the current
terminal.

## Diagnostics

```sh
telegram-agent-cli paths                    # runtime directories
telegram-agent-cli doctor                   # full diagnostic check
telegram-agent-cli doctor --format json     # machine-readable
```

## Help

```sh
telegram-agent-cli --help                   # plain text
telegram-agent-cli help send                # structured (YAML)
telegram-agent-cli help send --format json  # structured (JSON)
```

## Development

```sh
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Run locally without a release build:

```sh
cargo run -- send --as mybot --to @user --text "hello"
```
