pub mod changelog;
pub mod flags;
pub mod logs;
pub mod migrations;
pub mod secrets;
pub mod tests;
pub mod todos;
pub mod version;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
    Skip,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub critical: bool,
    pub detail: Option<String>,
    pub extra: Option<String>,
}

impl CheckResult {
    #[allow(dead_code)]
    pub fn pass(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Pass,
            critical: false,
            detail: None,
            extra: None,
        }
    }

    pub fn pass_with(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Pass,
            critical: false,
            detail: Some(detail.into()),
            extra: None,
        }
    }

    pub fn fail(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Fail,
            critical: true,
            detail: Some(detail.into()),
            extra: None,
        }
    }

    pub fn fail_soft(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Fail,
            critical: false,
            detail: Some(detail.into()),
            extra: None,
        }
    }

    pub fn warn(name: &str, detail: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Warn,
            critical: false,
            detail: Some(detail.into()),
            extra: None,
        }
    }

    pub fn skip(name: &str, reason: impl Into<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Skip,
            critical: false,
            detail: Some(reason.into()),
            extra: None,
        }
    }

    pub fn with_extra(mut self, extra: impl Into<String>) -> Self {
        self.extra = Some(extra.into());
        self
    }
}
