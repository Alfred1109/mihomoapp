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
