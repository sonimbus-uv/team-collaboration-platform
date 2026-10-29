use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_SESSION_CHECKS: usize = 100;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct SessionCheckRequest {
    pub sid: Uuid,
    pub account_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCheckBatchRequest {
    pub sessions: Vec<SessionCheckRequest>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SessionCheckResult {
    pub sid: Uuid,
    pub account_id: Uuid,
    pub active: bool,
    pub session_expires_at: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SessionCheckBatchResult {
    pub results: Vec<SessionCheckResult>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionCheckInputError {
    EmptyBatch,
    BatchTooLarge,
    DuplicatePair,
}
