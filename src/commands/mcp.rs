use crate::app::AppContext;
use crate::errors::{Result, TelegramCliError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::time::Duration;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct Request {
    #[allow(dead_code)]
    pub(crate) jsonrpc: String,
    #[serde(default)]
    pub(crate) id: Option<Value>,
    pub(crate) method: String,
    #[serde(default)]
    pub(crate) params: Option<Value>,
}

#[derive(Serialize)]
pub(crate) struct Response {
    pub(crate) jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<RpcError>,
}

#[derive(Serialize)]
pub(crate) struct RpcError {
    pub(crate) code: i64,
    pub(crate) message: String,
}

impl Response {
    pub(crate) fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub(crate) fn error(id: Option<Value>, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError { code, message }),
        }
    }
}

// ---------------------------------------------------------------------------
// MCP server entry point
// ---------------------------------------------------------------------------

const PROTOCOL_VERSION: &str = "2024-11-05";

pub async fn run(context: &AppContext) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request = match parse_request_line(&line) {
            Ok(request) => request,
            Err(response) => {
                write_response(&mut stdout, &response)?;
                continue;
            }
        };

        // Notifications (no id) don't get a response.
        if request.id.is_none() {
            continue;
        }

        let response = handle(context, &request).await;
        write_response(&mut stdout, &response)?;
    }

    Ok(())
}

pub(crate) fn parse_request_line(line: &str) -> std::result::Result<Request, Box<Response>> {
    serde_json::from_str(line).map_err(|error| {
        Box::new(Response::error(
            None,
            -32700,
            format!("Parse error: {error}"),
        ))
    })
}

pub(crate) fn render_response(resp: &Response) -> Result<String> {
    serde_json::to_string(resp)
        .map_err(|e| TelegramCliError::Message(format!("JSON serialization failed: {e}")))
}

fn write_response(stdout: &mut io::Stdout, resp: &Response) -> Result<()> {
    let json = render_response(resp)?;
    writeln!(stdout, "{json}")?;
    stdout.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Method dispatch
// ---------------------------------------------------------------------------

pub(crate) async fn handle(context: &AppContext, req: &Request) -> Response {
    match req.method.as_str() {
        "initialize" => handle_initialize(req),
        "ping" => Response::success(req.id.clone(), json!({})),
        "tools/list" => handle_tools_list(req),
        "tools/call" => handle_tools_call(context, req).await,
        _ => Response::error(
            req.id.clone(),
            -32601,
            format!("Method not found: {}", req.method),
        ),
    }
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

fn handle_initialize(req: &Request) -> Response {
    Response::success(
        req.id.clone(),
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "telegram-agent-cli",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

// ---------------------------------------------------------------------------
// tools/list
// ---------------------------------------------------------------------------

fn handle_tools_list(req: &Request) -> Response {
    Response::success(req.id.clone(), json!({ "tools": tool_definitions() }))
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "send_message",
            "description": "Send a text message to a Telegram peer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name (omit for default)", "default": "default" },
                    "chat": { "type": "string", "description": "Target: alias, @username, or numeric peer ID" },
                    "text": { "type": "string", "description": "Message text to send" }
                },
                "required": ["chat", "text"]
            }
        }),
        json!({
            "name": "send_file",
            "description": "Upload and send a file to a Telegram peer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Target chat identifier" },
                    "path": { "type": "string", "description": "Local file path to upload" },
                    "caption": { "type": "string", "description": "Optional caption for the file" }
                },
                "required": ["chat", "path"]
            }
        }),
        json!({
            "name": "recv_messages",
            "description": "Read recent messages from a Telegram chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Target chat identifier" },
                    "limit": { "type": "integer", "description": "Max messages to return (default 20)", "default": 20 }
                },
                "required": ["chat"]
            }
        }),
        json!({
            "name": "wait_for_message",
            "description": "Block until a matching incoming message arrives in a chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Target chat identifier" },
                    "text": { "type": "string", "description": "Require exact text match" },
                    "text_contains": { "type": "string", "description": "Require text to contain this substring" },
                    "timeout": { "type": "string", "description": "Max wait duration, e.g. '30s'", "default": "30s" }
                },
                "required": ["chat"]
            }
        }),
        json!({
            "name": "resolve_peer",
            "description": "Resolve a username, alias, or numeric ID into a Telegram peer record.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "query": { "type": "string", "description": "Alias, @username, or numeric peer ID" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "list_contacts",
            "description": "List the direct contacts of a Telegram account.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" }
                }
            }
        }),
        json!({
            "name": "list_chats",
            "description": "List groups and channels available to a Telegram account.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" }
                }
            }
        }),
        json!({
            "name": "list_accounts",
            "description": "List all configured telegram-agent-cli accounts and their login state.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "click_button",
            "description": "Click an inline button in a bot message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat containing the button" },
                    "button": { "type": "string", "description": "Button label or callback data" },
                    "message_id": { "type": "integer", "description": "Specific message ID (optional)" },
                    "wait_timeout": { "type": "string", "description": "How long to wait for a response", "default": "5s" }
                },
                "required": ["chat", "button"]
            }
        }),
        json!({
            "name": "list_actions",
            "description": "Discover interactive actions (inline buttons, reply keyboards, bot commands) in a chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat to inspect" },
                    "message_id": { "type": "integer", "description": "Inspect a specific message (optional)" },
                    "limit": { "type": "integer", "description": "Number of recent messages to scan", "default": 20 }
                },
                "required": ["chat"]
            }
        }),
        json!({
            "name": "trigger_action",
            "description": "Trigger a previously discovered interactive action by ID or label.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat containing the action" },
                    "action": { "type": "string", "description": "Action ID or label from list_actions" },
                    "message_id": { "type": "integer", "description": "Specific message ID (optional)" },
                    "wait_timeout": { "type": "string", "description": "How long to wait for a response", "default": "5s" }
                },
                "required": ["chat", "action"]
            }
        }),
        json!({
            "name": "send_photo",
            "description": "Send a photo to a Telegram peer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Target: alias, @username, or numeric peer ID" },
                    "path": { "type": "string", "description": "Local file path of the photo" },
                    "caption": { "type": "string", "description": "Optional caption for the photo" }
                },
                "required": ["chat", "path"]
            }
        }),
        json!({
            "name": "forward_messages",
            "description": "Forward messages from one chat to another.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "from_chat": { "type": "string", "description": "Source chat identifier" },
                    "to_chat": { "type": "string", "description": "Destination chat identifier" },
                    "message_ids": { "type": "string", "description": "Comma-separated message IDs to forward" }
                },
                "required": ["from_chat", "to_chat", "message_ids"]
            }
        }),
        json!({
            "name": "edit_message",
            "description": "Edit the text of an existing message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat containing the message" },
                    "message_id": { "type": "integer", "description": "Message ID to edit" },
                    "text": { "type": "string", "description": "New text content" }
                },
                "required": ["chat", "message_id", "text"]
            }
        }),
        json!({
            "name": "pin_message",
            "description": "Pin a message in a chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat containing the message" },
                    "message_id": { "type": "integer", "description": "Message ID to pin" }
                },
                "required": ["chat", "message_id"]
            }
        }),
        json!({
            "name": "unpin_message",
            "description": "Unpin a pinned message in a chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat containing the message" },
                    "message_id": { "type": "integer", "description": "Message ID to unpin" }
                },
                "required": ["chat", "message_id"]
            }
        }),
        json!({
            "name": "download_media",
            "description": "Download media attached to a message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "account": { "type": "string", "description": "Account name", "default": "default" },
                    "chat": { "type": "string", "description": "Chat containing the message" },
                    "message_id": { "type": "integer", "description": "Message ID with media to download" },
                    "output": { "type": "string", "description": "Local file path to save the media" }
                },
                "required": ["chat", "message_id"]
            }
        }),
    ]
}

// ---------------------------------------------------------------------------
// tools/call
// ---------------------------------------------------------------------------

async fn handle_tools_call(context: &AppContext, req: &Request) -> Response {
    let params = req.params.as_ref().unwrap_or(&Value::Null);
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));

    match call_tool(context, tool_name, &args).await {
        Ok(text) => Response::success(
            req.id.clone(),
            json!({ "content": [{ "type": "text", "text": text }] }),
        ),
        Err(e) => Response::success(
            req.id.clone(),
            json!({
                "content": [{ "type": "text", "text": format!("Error: {e}") }],
                "isError": true
            }),
        ),
    }
}

async fn call_tool(context: &AppContext, name: &str, args: &Value) -> Result<String> {
    let account = args
        .get("account")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let account = &context.resolve_account_name(account)?;

    match name {
        "send_message" => {
            let chat = require_str(args, "chat")?;
            let text = require_str(args, "text")?;
            let peer = context.resolve_peer(account, chat).await?;
            let msg = context
                .telegram
                .send_text(account, peer.peer_id, text, None, None)
                .await?;
            Ok(format!(
                "Message sent to {} (message_id: {})",
                peer.display_name, msg.message_id
            ))
        }

        "send_file" => {
            let chat = require_str(args, "chat")?;
            let path_str = require_str(args, "path")?;
            let caption = args.get("caption").and_then(|v| v.as_str());
            let peer = context.resolve_peer(account, chat).await?;
            let path = std::path::Path::new(path_str);
            let msg = context
                .telegram
                .send_file(account, peer.peer_id, path, caption, None, None)
                .await?;
            Ok(format!(
                "File sent to {} (message_id: {})",
                peer.display_name, msg.message_id
            ))
        }

        "recv_messages" => {
            let chat = require_str(args, "chat")?;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let peer = context.resolve_peer(account, chat).await?;
            let messages = context
                .telegram
                .recent_messages(account, peer.peer_id, limit, None, false)
                .await?;
            Ok(serde_json::to_string_pretty(&messages)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "wait_for_message" => {
            let chat = require_str(args, "chat")?;
            let timeout_str = args
                .get("timeout")
                .and_then(|v| v.as_str())
                .unwrap_or("30s");
            let timeout = humantime::parse_duration(timeout_str).unwrap_or(Duration::from_secs(30));
            let filter = crate::telegram::MessageFilter {
                text_equals: args.get("text").and_then(|v| v.as_str()).map(String::from),
                text_contains: args
                    .get("text_contains")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                ..Default::default()
            };
            let peer = context.resolve_peer(account, chat).await?;
            let msg = context
                .telegram
                .wait_for_message(account, peer.peer_id, &filter, timeout)
                .await?;
            Ok(serde_json::to_string_pretty(&msg)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "resolve_peer" => {
            let query = require_str(args, "query")?;
            let peer = context.resolve_peer(account, query).await?;
            Ok(serde_json::to_string_pretty(&peer)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "list_contacts" => {
            let contacts = context.telegram.list_contacts(account).await?;
            Ok(serde_json::to_string_pretty(&contacts)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "list_chats" => {
            let chats = context.telegram.list_chats(account).await?;
            Ok(serde_json::to_string_pretty(&chats)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "list_accounts" => {
            let accounts = context.repo.list_accounts()?;
            Ok(serde_json::to_string_pretty(&accounts)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "click_button" => {
            let chat = require_str(args, "chat")?;
            let button = require_str(args, "button")?;
            let message_id = args.get("message_id").and_then(|v| v.as_i64());
            let timeout_str = args
                .get("wait_timeout")
                .and_then(|v| v.as_str())
                .unwrap_or("5s");
            let timeout = humantime::parse_duration(timeout_str).unwrap_or(Duration::from_secs(5));
            let peer = context.resolve_peer(account, chat).await?;
            let response = context
                .telegram
                .click_button(account, peer.peer_id, button, message_id, timeout)
                .await?;
            match response {
                Some(msg) => Ok(serde_json::to_string_pretty(&msg)
                    .map_err(|e| TelegramCliError::Message(e.to_string()))?),
                None => Ok("Button clicked, no follow-up message received.".into()),
            }
        }

        "list_actions" => {
            let chat = require_str(args, "chat")?;
            let message_id = args.get("message_id").and_then(|v| v.as_i64());
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let peer = context.resolve_peer(account, chat).await?;
            let actions = context
                .telegram
                .list_actions(account, peer.peer_id, message_id, limit)
                .await?;
            Ok(serde_json::to_string_pretty(&actions)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "trigger_action" => {
            let chat = require_str(args, "chat")?;
            let action = require_str(args, "action")?;
            let message_id = args.get("message_id").and_then(|v| v.as_i64());
            let timeout_str = args
                .get("wait_timeout")
                .and_then(|v| v.as_str())
                .unwrap_or("5s");
            let timeout = humantime::parse_duration(timeout_str).unwrap_or(Duration::from_secs(5));
            let peer = context.resolve_peer(account, chat).await?;
            let result = context
                .telegram
                .trigger_action(account, peer.peer_id, action, message_id, timeout)
                .await?;
            Ok(serde_json::to_string_pretty(&result)
                .map_err(|e| TelegramCliError::Message(e.to_string()))?)
        }

        "send_photo" => {
            let chat = require_str(args, "chat")?;
            let path_str = require_str(args, "path")?;
            let caption = args.get("caption").and_then(|v| v.as_str());
            let peer = context.resolve_peer(account, chat).await?;
            let path = std::path::Path::new(path_str);
            let msg = context
                .telegram
                .send_photo(account, peer.peer_id, path, caption, None, None)
                .await?;
            Ok(format!(
                "Photo sent to {} (message_id: {})",
                peer.display_name, msg.message_id
            ))
        }

        "forward_messages" => {
            let from_chat = require_str(args, "from_chat")?;
            let to_chat = require_str(args, "to_chat")?;
            let ids_str = require_str(args, "message_ids")?;
            let from_peer = context.resolve_peer(account, from_chat).await?;
            let to_peer = context.resolve_peer(account, to_chat).await?;
            let message_ids: Vec<i32> = ids_str
                .split(',')
                .map(|s| {
                    s.trim().parse::<i32>().map_err(|e| {
                        TelegramCliError::Message(format!("invalid message ID '{}': {e}", s.trim()))
                    })
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let forwarded = context
                .telegram
                .forward_messages(account, from_peer.peer_id, to_peer.peer_id, &message_ids)
                .await?;
            Ok(format!(
                "Forwarded {} message(s) to {}",
                forwarded.len(),
                to_peer.display_name
            ))
        }

        "edit_message" => {
            let chat = require_str(args, "chat")?;
            let message_id = args
                .get("message_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    TelegramCliError::Message("Missing required parameter: message_id".into())
                })? as i32;
            let text = require_str(args, "text")?;
            let peer = context.resolve_peer(account, chat).await?;
            context
                .telegram
                .edit_message(account, peer.peer_id, message_id, text)
                .await?;
            Ok(format!(
                "Message {} edited in {}",
                message_id, peer.display_name
            ))
        }

        "pin_message" => {
            let chat = require_str(args, "chat")?;
            let message_id = args
                .get("message_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    TelegramCliError::Message("Missing required parameter: message_id".into())
                })? as i32;
            let peer = context.resolve_peer(account, chat).await?;
            context
                .telegram
                .pin_message(account, peer.peer_id, message_id)
                .await?;
            Ok(format!(
                "Message {} pinned in {}",
                message_id, peer.display_name
            ))
        }

        "unpin_message" => {
            let chat = require_str(args, "chat")?;
            let message_id = args
                .get("message_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    TelegramCliError::Message("Missing required parameter: message_id".into())
                })? as i32;
            let peer = context.resolve_peer(account, chat).await?;
            context
                .telegram
                .unpin_message(account, peer.peer_id, message_id)
                .await?;
            Ok(format!(
                "Message {} unpinned in {}",
                message_id, peer.display_name
            ))
        }

        "download_media" => {
            let chat = require_str(args, "chat")?;
            let message_id = args
                .get("message_id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    TelegramCliError::Message("Missing required parameter: message_id".into())
                })? as i32;
            let output = args.get("output").and_then(|v| v.as_str());
            let peer = context.resolve_peer(account, chat).await?;
            let output_path = match output {
                Some(path) => std::path::PathBuf::from(path),
                None => std::env::current_dir().map_err(|e| {
                    TelegramCliError::Message(format!("failed to resolve cwd: {e}"))
                })?,
            };
            let downloaded = context
                .telegram
                .download_media(account, peer.peer_id, message_id, &output_path)
                .await?;
            if downloaded {
                Ok(format!("Media downloaded to {}", output_path.display()))
            } else {
                Ok("No downloadable media found in the message.".into())
            }
        }

        _ => Err(TelegramCliError::Message(format!("Unknown tool: {name}"))),
    }
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| TelegramCliError::Message(format!("Missing required parameter: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definitions_have_required_fields() {
        let tools = tool_definitions();
        assert!(!tools.is_empty());
        for tool in &tools {
            assert!(tool.get("name").is_some(), "tool missing name");
            assert!(
                tool.get("description").is_some(),
                "tool missing description"
            );
            assert!(
                tool.get("inputSchema").is_some(),
                "tool missing inputSchema"
            );
        }
    }

    #[test]
    fn initialize_response_contains_server_info() {
        let req = Request {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "initialize".into(),
            params: None,
        };
        let resp = handle_initialize(&req);
        let result = resp.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], "telegram-agent-cli");
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
    }

    #[test]
    fn tools_list_returns_all_tools() {
        let req = Request {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "tools/list".into(),
            params: None,
        };
        let resp = handle_tools_list(&req);
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert!(tools.len() >= 17, "expected at least 17 tools");
    }
}
