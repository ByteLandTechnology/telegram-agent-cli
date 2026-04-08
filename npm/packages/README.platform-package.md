# telegram-agent-cli platform binary

This package contains one prebuilt `telegram-agent-cli` binary for a single
operating-system and CPU target.

Install the main `telegram-agent-cli` package for normal usage. It declares the
matching platform package as an `optionalDependency` and executes the bundled
binary locally without downloading from GitHub Releases at runtime.
