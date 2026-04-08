use crate::errors::{Result, TelegramCliError};
use grammers_client::message::{Button, Key, ReplyMarkup};

/// A single reply-keyboard button specification.
///
/// Parsed from shorthand strings:
/// - `"Yes"` → plain text button
/// - `"Share:phone"` → request phone
/// - `"Location:geo"` → request geo location
/// - `"Vote:poll"` → request poll
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyButtonSpec {
    pub label: String,
    pub kind: ReplyButtonKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyButtonKind {
    Text,
    RequestPhone,
    RequestGeo,
    RequestPoll,
}

impl TryFrom<&str> for ReplyButtonSpec {
    type Error = TelegramCliError;

    fn try_from(value: &str) -> Result<Self> {
        if let Some((label, kind_str)) = value.rsplit_once(':') {
            let kind = match kind_str {
                "phone" => ReplyButtonKind::RequestPhone,
                "geo" => ReplyButtonKind::RequestGeo,
                "poll" => ReplyButtonKind::RequestPoll,
                _ => {
                    return Err(TelegramCliError::Message(format!(
                        "unknown reply button kind '{kind_str}'; expected 'phone', 'geo', or 'poll'"
                    )))
                }
            };
            Ok(Self {
                label: label.to_string(),
                kind,
            })
        } else {
            Ok(Self {
                label: value.to_string(),
                kind: ReplyButtonKind::Text,
            })
        }
    }
}

/// A single inline button specification.
///
/// Parsed from shorthand strings:
/// - `"Click:callback:data"` → callback button
/// - `"Open:url:https://example.com"` → URL button
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineButtonSpec {
    pub label: String,
    pub kind: InlineButtonKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineButtonKind {
    Callback { data: String },
    Url { url: String },
}

impl TryFrom<&str> for InlineButtonSpec {
    type Error = TelegramCliError;

    fn try_from(value: &str) -> Result<Self> {
        let (label, rest) = value.split_once(':').ok_or_else(|| {
            TelegramCliError::Message("inline button must be 'label:kind:payload'".into())
        })?;
        let (kind, payload) = rest.split_once(':').ok_or_else(|| {
            TelegramCliError::Message("inline button must be 'label:kind:payload'".into())
        })?;
        let kind = match kind {
            "callback" => InlineButtonKind::Callback {
                data: payload.to_string(),
            },
            "url" => InlineButtonKind::Url {
                url: payload.to_string(),
            },
            _ => {
                return Err(TelegramCliError::Message(format!(
                    "unknown inline button kind '{kind}'; expected 'callback' or 'url'"
                )))
            }
        };
        Ok(Self {
            label: label.to_string(),
            kind,
        })
    }
}

/// Keyboard configuration to attach to a message.
#[derive(Debug, Clone, Default)]
pub struct ReplyMarkupConfig {
    pub reply_keyboard: Option<Vec<Vec<ReplyButtonSpec>>>,
    pub inline_keyboard: Option<Vec<Vec<InlineButtonSpec>>>,
}

impl ReplyMarkupConfig {
    /// Build from CLI flags. Returns error if both are specified.
    pub fn from_flags(
        reply_keyboard_json: Option<&str>,
        inline_keyboard_json: Option<&str>,
    ) -> Result<Option<Self>> {
        match (reply_keyboard_json, inline_keyboard_json) {
            (None, None) => Ok(None),
            (Some(_), Some(_)) => Err(TelegramCliError::Message(
                "--reply-keyboard and --inline-keyboard cannot be used together".into(),
            )),
            (Some(json), None) => {
                let rows: Vec<Vec<String>> = serde_json::from_str(json).map_err(|e| {
                    TelegramCliError::Message(format!("invalid --reply-keyboard JSON: {e}"))
                })?;
                let reply_keyboard = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|s| ReplyButtonSpec::try_from(s.as_str()))
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Some(Self {
                    reply_keyboard: Some(reply_keyboard),
                    inline_keyboard: None,
                }))
            }
            (None, Some(json)) => {
                let rows: Vec<Vec<String>> = serde_json::from_str(json).map_err(|e| {
                    TelegramCliError::Message(format!("invalid --inline-keyboard JSON: {e}"))
                })?;
                let inline_keyboard = rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|s| InlineButtonSpec::try_from(s.as_str()))
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Some(Self {
                    reply_keyboard: None,
                    inline_keyboard: Some(inline_keyboard),
                }))
            }
        }
    }

    /// Convert to grammers-client `ReplyMarkup`.
    /// Returns `None` if neither keyboard is configured.
    pub fn to_reply_markup(&self) -> Option<ReplyMarkup> {
        if let Some(rows) = &self.reply_keyboard {
            let keys: Vec<Vec<Key>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|spec| match spec.kind {
                            ReplyButtonKind::Text => Key::text(&spec.label),
                            ReplyButtonKind::RequestPhone => Key::request_phone(&spec.label),
                            ReplyButtonKind::RequestGeo => Key::request_geo(&spec.label),
                            ReplyButtonKind::RequestPoll => Key::request_poll(&spec.label),
                        })
                        .collect()
                })
                .collect();
            Some(ReplyMarkup::from_keys(&keys))
        } else if let Some(rows) = &self.inline_keyboard {
            let buttons: Vec<Vec<Button>> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|spec| match &spec.kind {
                            InlineButtonKind::Callback { data } => {
                                Button::data(&spec.label, data.as_bytes())
                            }
                            InlineButtonKind::Url { url } => Button::url(&spec.label, url),
                        })
                        .collect()
                })
                .collect();
            Some(ReplyMarkup::from_buttons(&buttons))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_reply_button() {
        let spec = ReplyButtonSpec::try_from("Hello").unwrap();
        assert_eq!(spec.label, "Hello");
        assert_eq!(spec.kind, ReplyButtonKind::Text);
    }

    #[test]
    fn parse_phone_reply_button() {
        let spec = ReplyButtonSpec::try_from("Share:phone").unwrap();
        assert_eq!(spec.label, "Share");
        assert_eq!(spec.kind, ReplyButtonKind::RequestPhone);
    }

    #[test]
    fn parse_geo_reply_button() {
        let spec = ReplyButtonSpec::try_from("Location:geo").unwrap();
        assert_eq!(spec.label, "Location");
        assert_eq!(spec.kind, ReplyButtonKind::RequestGeo);
    }

    #[test]
    fn parse_poll_reply_button() {
        let spec = ReplyButtonSpec::try_from("Vote:poll").unwrap();
        assert_eq!(spec.label, "Vote");
        assert_eq!(spec.kind, ReplyButtonKind::RequestPoll);
    }

    #[test]
    fn parse_unknown_reply_button_kind() {
        let err = ReplyButtonSpec::try_from("Test:bad").unwrap_err();
        assert!(err.to_string().contains("unknown reply button kind"));
    }

    #[test]
    fn parse_callback_inline_button() {
        let spec = InlineButtonSpec::try_from("OK:callback:done").unwrap();
        assert_eq!(spec.label, "OK");
        assert_eq!(
            spec.kind,
            InlineButtonKind::Callback {
                data: "done".into()
            }
        );
    }

    #[test]
    fn parse_url_inline_button() {
        let spec = InlineButtonSpec::try_from("Open:url:https://example.com").unwrap();
        assert_eq!(spec.label, "Open");
        assert_eq!(
            spec.kind,
            InlineButtonKind::Url {
                url: "https://example.com".into()
            }
        );
    }

    #[test]
    fn parse_inline_button_missing_payload() {
        let err = InlineButtonSpec::try_from("Bad:callback").unwrap_err();
        assert!(err.to_string().contains("label:kind:payload"));
    }

    #[test]
    fn parse_inline_button_missing_kind() {
        let err = InlineButtonSpec::try_from("Nocolon").unwrap_err();
        assert!(err.to_string().contains("label:kind:payload"));
    }

    #[test]
    fn from_flags_none() {
        assert!(ReplyMarkupConfig::from_flags(None, None).unwrap().is_none());
    }

    #[test]
    fn from_flags_both_set_rejects() {
        let err = ReplyMarkupConfig::from_flags(Some("[[\"A\"]]"), Some("[[\"B\"]]")).unwrap_err();
        assert!(err.to_string().contains("cannot be used together"));
    }

    #[test]
    fn from_flags_reply_keyboard() {
        let config = ReplyMarkupConfig::from_flags(Some(r#"[["Yes","No"],["Share:phone"]]"#), None)
            .unwrap()
            .unwrap();
        assert!(config.reply_keyboard.is_some());
        assert!(config.inline_keyboard.is_none());
        let rows = config.reply_keyboard.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0].label, "Yes");
        assert_eq!(rows[0][1].label, "No");
        assert_eq!(rows[1][0].kind, ReplyButtonKind::RequestPhone);
    }

    #[test]
    fn from_flags_inline_keyboard() {
        let config = ReplyMarkupConfig::from_flags(None, Some(r#"[["OK:callback:done"]]"#))
            .unwrap()
            .unwrap();
        assert!(config.reply_keyboard.is_none());
        assert!(config.inline_keyboard.is_some());
        let rows = config.inline_keyboard.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0].label, "OK");
    }

    #[test]
    fn from_flags_invalid_json() {
        let err = ReplyMarkupConfig::from_flags(Some("not-json"), None).unwrap_err();
        assert!(err.to_string().contains("invalid --reply-keyboard JSON"));
    }

    #[test]
    fn to_reply_markup_returns_none_for_default() {
        let config = ReplyMarkupConfig::default();
        assert!(config.to_reply_markup().is_none());
    }

    #[test]
    fn to_reply_markup_returns_some_for_reply_keyboard() {
        let config = ReplyMarkupConfig::from_flags(Some(r#"[["Yes"]]"#), None)
            .unwrap()
            .unwrap();
        assert!(config.to_reply_markup().is_some());
    }

    #[test]
    fn to_reply_markup_returns_some_for_inline_keyboard() {
        let config = ReplyMarkupConfig::from_flags(None, Some(r#"[["Click:callback:data"]]"#))
            .unwrap()
            .unwrap();
        assert!(config.to_reply_markup().is_some());
    }
}
