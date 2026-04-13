use crate::app::AppContext;
use crate::cli::{
    DaemonCommand, DaemonRestartArgs, DaemonServeArgs, DaemonStartArgs, DaemonStatusArgs,
    DaemonStopArgs,
};
use crate::commands::mcp;
use crate::errors::{Result, TelegramCliError};
use crate::output::contract::NextStep;
use crate::output::Format;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::net::TcpListener;

const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(100);
const DAEMON_SHUTDOWN_METHOD: &str = "daemon/shutdown";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DaemonMetadata {
    pid: u32,
    port: u16,
    started_at: String,
    log_path: PathBuf,
}

#[derive(Debug, Clone)]
struct DaemonPaths {
    root: PathBuf,
    metadata: PathBuf,
    log: PathBuf,
}

#[derive(Debug, Clone)]
struct DaemonInspection {
    metadata: Option<DaemonMetadata>,
    paths: DaemonPaths,
    running: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DaemonStatusView {
    state: String,
    running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_path: Option<PathBuf>,
    metadata_path: PathBuf,
}

pub async fn run(context: &AppContext, command: DaemonCommand, format: Format) -> Result<()> {
    match command {
        DaemonCommand::Start(args) => start(context, args, format).await,
        DaemonCommand::Stop(args) => stop(context, args, format).await,
        DaemonCommand::Restart(args) => restart(context, args, format).await,
        DaemonCommand::Status(args) => status(context, args, format).await,
        DaemonCommand::Serve(args) => serve(context, args).await,
    }
}

fn daemon_paths(context: &AppContext) -> DaemonPaths {
    let root = context.paths.state_dir.join("daemon");
    DaemonPaths {
        metadata: root.join("server.json"),
        log: root.join("server.log"),
        root,
    }
}

fn endpoint_for_port(port: u16) -> String {
    format!("127.0.0.1:{port}")
}

fn socket_addr_for_port(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

fn parse_duration(value: &str, label: &str) -> Result<Duration> {
    humantime::parse_duration(value).map_err(|error| {
        TelegramCliError::Message(format!("invalid {label} duration '{value}': {error}"))
    })
}

fn load_metadata(path: &Path) -> Result<Option<DaemonMetadata>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)?;
    let metadata = serde_json::from_str::<DaemonMetadata>(&contents).map_err(|error| {
        TelegramCliError::Message(format!(
            "failed to parse daemon metadata at {}: {error}",
            path.display()
        ))
    })?;

    Ok(Some(metadata))
}

fn write_metadata(path: &Path, metadata: &DaemonMetadata) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string_pretty(metadata).map_err(|error| {
        TelegramCliError::Message(format!("failed to serialize daemon metadata: {error}"))
    })?;
    fs::write(path, format!("{contents}\n"))?;
    Ok(())
}

fn remove_metadata_if_exists(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn ping_port(port: u16, timeout: Duration) -> bool {
    send_rpc_request(port, "ping", timeout).is_ok()
}

fn send_shutdown_request(port: u16, timeout: Duration) -> Result<()> {
    let _ = send_rpc_request(port, DAEMON_SHUTDOWN_METHOD, timeout)?;
    Ok(())
}

fn send_rpc_request(port: u16, method: &str, timeout: Duration) -> Result<Value> {
    let address = socket_addr_for_port(port);
    let mut stream = TcpStream::connect_timeout(&address, timeout).map_err(|error| {
        TelegramCliError::Message(format!(
            "failed to connect to daemon at {}: {error}",
            endpoint_for_port(port)
        ))
    })?;

    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": {},
    });
    writeln!(stream, "{request}")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let read = reader.read_line(&mut line)?;
    if read == 0 {
        return Err(TelegramCliError::Message(
            "daemon closed the connection before sending a response".into(),
        ));
    }

    let response: Value = serde_json::from_str(line.trim()).map_err(|error| {
        TelegramCliError::Message(format!("failed to parse daemon response: {error}"))
    })?;

    if let Some(error) = response.get("error") {
        return Err(TelegramCliError::Message(format!(
            "daemon returned an error: {error}"
        )));
    }

    Ok(response
        .get("result")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default())))
}

fn inspect_daemon(context: &AppContext) -> Result<DaemonInspection> {
    let paths = daemon_paths(context);
    let metadata = load_metadata(&paths.metadata)?;
    let running = metadata
        .as_ref()
        .map(|entry| ping_port(entry.port, Duration::from_millis(500)))
        .unwrap_or(false);

    Ok(DaemonInspection {
        metadata,
        paths,
        running,
    })
}

fn daemon_state_label(inspection: &DaemonInspection) -> &'static str {
    match (&inspection.metadata, inspection.running) {
        (_, true) => "running",
        (Some(_), false) => "stale",
        (None, false) => "stopped",
    }
}

fn daemon_status_view(inspection: &DaemonInspection) -> DaemonStatusView {
    let endpoint = inspection
        .metadata
        .as_ref()
        .map(|metadata| endpoint_for_port(metadata.port));
    let log_path = inspection
        .metadata
        .as_ref()
        .map(|metadata| metadata.log_path.clone());

    DaemonStatusView {
        state: daemon_state_label(inspection).to_string(),
        running: inspection.running,
        pid: inspection.metadata.as_ref().map(|metadata| metadata.pid),
        endpoint,
        started_at: inspection
            .metadata
            .as_ref()
            .map(|metadata| metadata.started_at.clone()),
        log_path,
        metadata_path: inspection.paths.metadata.clone(),
    }
}

fn status_summary(inspection: &DaemonInspection) -> &'static str {
    match daemon_state_label(inspection) {
        "running" => "Daemon is running.",
        "stale" => "Daemon metadata exists, but the server is not responding.",
        _ => "Daemon is not running.",
    }
}

fn status_next_steps(inspection: &DaemonInspection) -> Vec<NextStep> {
    match daemon_state_label(inspection) {
        "running" => vec![
            NextStep {
                action: "inspect_daemon".into(),
                command: "telegram-agent-cli daemon status".into(),
            },
            NextStep {
                action: "stop_daemon".into(),
                command: "telegram-agent-cli daemon stop".into(),
            },
        ],
        "stale" => vec![
            NextStep {
                action: "restart_daemon".into(),
                command: "telegram-agent-cli daemon restart".into(),
            },
            NextStep {
                action: "inspect_log".into(),
                command: "telegram-agent-cli daemon status --format json".into(),
            },
        ],
        _ => vec![
            NextStep {
                action: "start_daemon".into(),
                command: "telegram-agent-cli daemon start".into(),
            },
            NextStep {
                action: "inspect_mcp_help".into(),
                command: "telegram-agent-cli mcp --help".into(),
            },
        ],
    }
}

async fn start(context: &AppContext, args: DaemonStartArgs, format: Format) -> Result<()> {
    let timeout = parse_duration(&args.timeout, "startup")?;
    let (inspection, already_running) = start_daemon(context, timeout, &args.timeout).await?;
    let view = daemon_status_view(&inspection);
    let summary = if already_running {
        "Daemon is already running."
    } else {
        "Daemon is running in the background."
    };
    format.print_result(
        "telegram-agent-cli daemon start",
        summary,
        &view,
        vec![
            NextStep {
                action: "inspect_daemon".into(),
                command: "telegram-agent-cli daemon status".into(),
            },
            NextStep {
                action: "stop_daemon".into(),
                command: "telegram-agent-cli daemon stop".into(),
            },
        ],
    )
}

async fn stop(context: &AppContext, args: DaemonStopArgs, format: Format) -> Result<()> {
    let timeout = parse_duration(&args.timeout, "shutdown")?;
    let (inspection, was_running) = stop_daemon(context, timeout, &args.timeout).await?;
    let view = daemon_status_view(&inspection);
    let summary = if was_running {
        "Daemon was stopped."
    } else {
        "Daemon is not running."
    };
    format.print_result(
        "telegram-agent-cli daemon stop",
        summary,
        &view,
        vec![
            NextStep {
                action: "start_daemon".into(),
                command: "telegram-agent-cli daemon start".into(),
            },
            NextStep {
                action: "inspect_status".into(),
                command: "telegram-agent-cli daemon status".into(),
            },
        ],
    )
}

async fn restart(context: &AppContext, args: DaemonRestartArgs, format: Format) -> Result<()> {
    let timeout = parse_duration(&args.timeout, "restart")?;
    let _ = stop_daemon(context, timeout, &args.timeout).await?;
    let (inspection, _) = start_daemon(context, timeout, &args.timeout).await?;
    let view = daemon_status_view(&inspection);
    format.print_result(
        "telegram-agent-cli daemon restart",
        "Daemon was restarted.",
        &view,
        vec![
            NextStep {
                action: "inspect_daemon".into(),
                command: "telegram-agent-cli daemon status".into(),
            },
            NextStep {
                action: "stop_daemon".into(),
                command: "telegram-agent-cli daemon stop".into(),
            },
        ],
    )
}

async fn status(context: &AppContext, _args: DaemonStatusArgs, format: Format) -> Result<()> {
    let inspection = inspect_daemon(context)?;
    let view = daemon_status_view(&inspection);
    format.print_result(
        "telegram-agent-cli daemon status",
        status_summary(&inspection),
        &view,
        status_next_steps(&inspection),
    )
}

async fn serve(context: &AppContext, args: DaemonServeArgs) -> Result<()> {
    let metadata_path = args.metadata_path;
    if let Some(parent) = metadata_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let metadata = DaemonMetadata {
        pid: std::process::id(),
        port,
        started_at: Utc::now().to_rfc3339(),
        log_path: metadata_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("server.log"),
    };
    write_metadata(&metadata_path, &metadata)?;

    let result = async {
        loop {
            let (socket, _) = listener.accept().await?;
            if handle_connection(context, socket).await? {
                break;
            }
        }

        Ok(())
    }
    .await;

    let _ = remove_metadata_if_exists(&metadata_path);
    result
}

async fn start_daemon(
    context: &AppContext,
    timeout: Duration,
    timeout_label: &str,
) -> Result<(DaemonInspection, bool)> {
    let mut inspection = inspect_daemon(context)?;

    if inspection.running {
        return Ok((inspection, true));
    }

    fs::create_dir_all(&inspection.paths.root)?;
    if inspection.metadata.is_some() {
        remove_metadata_if_exists(&inspection.paths.metadata)?;
    }

    let current_exe = std::env::current_exe().map_err(|error| {
        TelegramCliError::Message(format!("failed to locate current executable: {error}"))
    })?;
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&inspection.paths.log)?;

    let mut child = Command::new(current_exe);
    child
        .arg("daemon")
        .arg("__serve")
        .arg("--metadata-path")
        .arg(&inspection.paths.metadata)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file));

    let mut child = child.spawn().map_err(|error| {
        TelegramCliError::Message(format!("failed to start background daemon: {error}"))
    })?;

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(TelegramCliError::Message(format!(
                "daemon exited before becoming ready with status {status}; inspect {}",
                inspection.paths.log.display()
            )));
        }

        inspection = inspect_daemon(context)?;
        if inspection.running {
            return Ok((inspection, false));
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(TelegramCliError::Message(format!(
                "daemon did not become ready within {}; inspect {}",
                timeout_label,
                inspection.paths.log.display()
            )));
        }

        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }
}

async fn stop_daemon(
    context: &AppContext,
    timeout: Duration,
    timeout_label: &str,
) -> Result<(DaemonInspection, bool)> {
    let inspection = inspect_daemon(context)?;

    if !inspection.running {
        if inspection.metadata.is_some() {
            remove_metadata_if_exists(&inspection.paths.metadata)?;
        }
        return Ok((inspect_daemon(context)?, false));
    }

    let metadata = inspection.metadata.as_ref().ok_or_else(|| {
        TelegramCliError::Message("daemon metadata is missing while the daemon is running".into())
    })?;

    send_shutdown_request(metadata.port, timeout)?;

    let started = Instant::now();
    loop {
        let next = inspect_daemon(context)?;
        if !next.running {
            if next.metadata.is_some() {
                remove_metadata_if_exists(&next.paths.metadata)?;
            }
            return Ok((inspect_daemon(context)?, true));
        }

        if started.elapsed() >= timeout {
            return Err(TelegramCliError::Message(format!(
                "daemon did not stop within {}; run telegram-agent-cli daemon status to inspect it",
                timeout_label
            )));
        }

        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }
}

async fn handle_connection(context: &AppContext, socket: tokio::net::TcpStream) -> Result<bool> {
    let (reader, mut writer) = socket.into_split();
    let mut lines = AsyncBufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let request = match mcp::parse_request_line(&line) {
            Ok(request) => request,
            Err(response) => {
                write_async_response(&mut writer, &response).await?;
                continue;
            }
        };

        if request.id.is_none() {
            continue;
        }

        if request.method == DAEMON_SHUTDOWN_METHOD {
            let response = mcp::Response::success(
                request.id.clone(),
                json!({
                    "status": "shutting_down",
                }),
            );
            write_async_response(&mut writer, &response).await?;
            return Ok(true);
        }

        let response = mcp::handle(context, &request).await;
        write_async_response(&mut writer, &response).await?;
    }

    Ok(false)
}

async fn write_async_response<W>(writer: &mut W, response: &mcp::Response) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let json = mcp::render_response(response)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
