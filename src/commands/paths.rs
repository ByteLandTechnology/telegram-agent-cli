use crate::app::AppContext;
use crate::config::paths::RuntimePathSource;
use crate::errors::Result;
use crate::output::contract::{NextStep, RuntimeDirectoryView, RuntimePathsView};
use crate::output::Format;
use std::path::Path;

pub fn run(context: &AppContext, format: Format) -> Result<()> {
    let data = RuntimePathsView {
        config: runtime_directory(
            &context.paths.config_dir,
            context.paths.config_source,
            "Configuration files, account metadata, and local CLI settings.",
        ),
        data: runtime_directory(
            &context.paths.data_dir,
            context.paths.data_source,
            "Persistent application data such as the encrypted account database.",
        ),
        state: runtime_directory(
            &context.paths.state_dir,
            context.paths.state_source,
            "Mutable session state, REPL history, and local runtime artifacts.",
        ),
        cache: runtime_directory(
            &context.paths.cache_dir,
            context.paths.cache_source,
            "Disposable caches and ephemeral data that can be rebuilt locally.",
        ),
    };

    format.print_result(
        "telegram-agent-cli paths",
        "Runtime directories collected.",
        &data,
        vec![
            NextStep {
                action: "inspect_context".into(),
                command: "telegram-agent-cli context show".into(),
            },
            NextStep {
                action: "inspect_diagnostics".into(),
                command: "telegram-agent-cli doctor".into(),
            },
        ],
    )
}

fn runtime_directory(
    path: &Path,
    source: RuntimePathSource,
    description: &str,
) -> RuntimeDirectoryView {
    RuntimeDirectoryView {
        path: path.to_path_buf(),
        source: source.as_str().to_string(),
        description: description.to_string(),
        user_scoped: true,
    }
}
