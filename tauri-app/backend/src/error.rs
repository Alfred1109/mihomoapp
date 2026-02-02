use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("进程错误: {0}")]
    ProcessError(String),

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("文件系统错误: {0}")]
    FileSystemError(String),

    #[error("权限错误: {0}")]
    PermissionError(String),

    #[error("验证错误: {0}")]
    ValidationError(String),

    #[error("订阅错误: {0}")]
    SubscriptionError(String),

    #[error("备份错误: {0}")]
    BackupError(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML 错误: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("JSON 错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP 错误: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Anyhow 错误: {0}")]
    AnyhowError(#[from] anyhow::Error),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::ConfigError(_) => "CONFIG_ERROR",
            AppError::ProcessError(_) => "PROCESS_ERROR",
            AppError::NetworkError(_) => "NETWORK_ERROR",
            AppError::FileSystemError(_) => "FILESYSTEM_ERROR",
            AppError::PermissionError(_) => "PERMISSION_ERROR",
            AppError::ValidationError(_) => "VALIDATION_ERROR",
            AppError::SubscriptionError(_) => "SUBSCRIPTION_ERROR",
            AppError::BackupError(_) => "BACKUP_ERROR",
            AppError::IoError(_) => "IO_ERROR",
            AppError::YamlError(_) => "YAML_ERROR",
            AppError::JsonError(_) => "JSON_ERROR",
            AppError::HttpError(_) => "HTTP_ERROR",
            AppError::AnyhowError(_) => "INTERNAL_ERROR",
            AppError::Other(_) => "UNKNOWN_ERROR",
        }
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Other(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        ApiError::new(error.code(), error.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::new("INTERNAL_ERROR", error.to_string())
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;
