#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    SessionImport,
    Interactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginRequest {
    Bot,
    User(UserLoginRequest),
    UserQr,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserLoginRequest {
    pub code: Option<String>,
    pub password: Option<String>,
}
