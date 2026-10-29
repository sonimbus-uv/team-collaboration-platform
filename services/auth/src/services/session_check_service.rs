use std::collections::HashSet;

use crate::{
    session_check_types::{
        SessionCheckBatchRequest, SessionCheckBatchResult, SessionCheckInputError,
        SessionCheckRequest, SessionCheckResult, MAX_SESSION_CHECKS,
    },
    session_checks_repository::{SessionCheckRow, SessionChecksRepository},
};
use sqlx::PgPool;

#[derive(Debug)]
pub enum SessionCheckError {
    InvalidInput(SessionCheckInputError),
    Database(sqlx::Error),
    /// A repository result cannot be associated with every requested pair.
    IncompleteResult,
}

#[derive(Clone)]
pub struct SessionCheckService {
    repository: SessionChecksRepository,
}

impl SessionCheckService {
    /// The pool must connect to Auth's authoritative PostgreSQL primary, never a replica.
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: SessionChecksRepository::new(pool),
        }
    }

    pub async fn check(
        &self,
        request: SessionCheckRequest,
    ) -> Result<SessionCheckResult, SessionCheckError> {
        let mut results = self.evaluate(&[request]).await?;

        results.pop().ok_or(SessionCheckError::IncompleteResult)
    }

    pub async fn check_batch(
        &self,
        request: SessionCheckBatchRequest,
    ) -> Result<SessionCheckBatchResult, SessionCheckError> {
        validate_batch(&request.sessions).map_err(SessionCheckError::InvalidInput)?;
        Ok(SessionCheckBatchResult {
            results: self.evaluate(&request.sessions).await?,
        })
    }

    async fn evaluate(
        &self,
        requests: &[SessionCheckRequest],
    ) -> Result<Vec<SessionCheckResult>, SessionCheckError> {
        let rows = self
            .repository
            .check(requests)
            .await
            .map_err(SessionCheckError::Database)?;
        interpret(requests, rows)
    }
}

fn validate_batch(requests: &[SessionCheckRequest]) -> Result<(), SessionCheckInputError> {
    if requests.is_empty() {
        return Err(SessionCheckInputError::EmptyBatch);
    }
    if requests.len() > MAX_SESSION_CHECKS {
        return Err(SessionCheckInputError::BatchTooLarge);
    }

    let mut pairs = HashSet::with_capacity(requests.len());

    if requests.iter().any(|request| !pairs.insert(*request)) {
        return Err(SessionCheckInputError::DuplicatePair);
    }

    Ok(())
}

fn interpret(
    requests: &[SessionCheckRequest],
    rows: Vec<SessionCheckRow>,
) -> Result<Vec<SessionCheckResult>, SessionCheckError> {
    if rows.len() != requests.len()
        || rows
            .iter()
            .zip(requests)
            .any(|(row, request)| row.sid != request.sid || row.account_id != request.account_id)
    {
        return Err(SessionCheckError::IncompleteResult);
    }
    Ok(rows
        .into_iter()
        .map(|row| {
            let active = row.account_allowed
                && row.unrevoked
                && row
                    .expires_at
                    .is_some_and(|expires| expires > row.observed_at);
            SessionCheckResult {
                sid: row.sid,
                account_id: row.account_id,
                active,
                session_expires_at: if active {
                    row.expires_at
                        .map(|expires| expires.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string())
                } else {
                    None
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::types::chrono::DateTime;
    use std::time::Duration;
    use uuid::Uuid;

    fn request() -> SessionCheckRequest {
        SessionCheckRequest {
            sid: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn exact_expiry_and_all_negative_conditions_are_inactive_without_expiration() {
        let request = request();
        let now = DateTime::from_timestamp(1_800_000_000, 123_456_000).unwrap();
        for (allowed, unrevoked, expiry, expected) in [
            (true, true, Some(now + Duration::from_micros(1)), true),
            (true, true, Some(now), false),
            (true, true, Some(now - Duration::from_micros(1)), false),
            (false, true, Some(now + Duration::from_secs(1)), false),
            (true, false, Some(now + Duration::from_secs(1)), false),
            (false, false, None, false),
        ] {
            let row = SessionCheckRow {
                sid: request.sid,
                account_id: request.account_id,
                account_allowed: allowed,
                unrevoked,
                expires_at: expiry,
                observed_at: now,
            };
            let result = interpret(&[request], vec![row]).unwrap().pop().unwrap();
            assert_eq!(result.active, expected);
            assert_eq!(result.session_expires_at.is_some(), expected);
            if expected {
                assert_eq!(
                    result.session_expires_at.as_deref(),
                    Some("2027-01-15T08:00:00.123457Z")
                );
            }
        }
    }

    #[test]
    fn missing_or_misassociated_rows_never_produce_results() {
        let request = request();
        assert!(matches!(
            interpret(&[request], vec![]),
            Err(SessionCheckError::IncompleteResult)
        ));
        let row = SessionCheckRow {
            sid: Uuid::new_v4(),
            account_id: request.account_id,
            account_allowed: true,
            unrevoked: true,
            expires_at: None,
            observed_at: DateTime::from_timestamp(0, 0).unwrap(),
        };
        assert!(matches!(
            interpret(&[request], vec![row]),
            Err(SessionCheckError::IncompleteResult)
        ));
    }

    #[test]
    fn batch_limits_and_duplicate_pair_validation() {
        assert_eq!(validate_batch(&[]), Err(SessionCheckInputError::EmptyBatch));
        let first = request();
        assert_eq!(
            validate_batch(&[first, first]),
            Err(SessionCheckInputError::DuplicatePair)
        );
        let different_account = SessionCheckRequest {
            account_id: Uuid::new_v4(),
            ..first
        };
        assert!(validate_batch(&[first, different_account]).is_ok());
        let mut requests: Vec<_> = (0..MAX_SESSION_CHECKS).map(|_| request()).collect();
        assert!(validate_batch(&requests).is_ok());
        requests.push(request());
        assert_eq!(
            validate_batch(&requests),
            Err(SessionCheckInputError::BatchTooLarge)
        );
    }

    #[test]
    fn malformed_pairs_and_batch_elements_are_rejected_before_service() {
        let first = request();
        let valid = serde_json::json!({"sid":first.sid,"account_id":first.account_id});
        for invalid in [
            serde_json::json!({"sid":"invalid","account_id":first.account_id}),
            serde_json::json!({"sid":first.sid}),
            serde_json::json!({"sid":first.sid,"account_id":null}),
            serde_json::json!({"sid":first.sid,"account_id":first.account_id,"token":"secret"}),
        ] {
            assert!(serde_json::from_value::<SessionCheckRequest>(invalid.clone()).is_err());
            assert!(serde_json::from_value::<SessionCheckBatchRequest>(
                serde_json::json!({"sessions":[valid.clone(),invalid]})
            )
            .is_err());
        }
    }
}
