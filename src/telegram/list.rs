#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Contact {
    pub id: i64,
    pub phone: Option<String>,
    pub display_name: String,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatKind {
    Group,
    Channel,
}

impl ChatKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Channel => "channel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Chat {
    pub id: i64,
    pub kind: ChatKind,
    pub display_name: String,
    pub username: Option<String>,
}
