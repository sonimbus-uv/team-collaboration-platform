//! sqlx::test creates a separate migrated database per test. Login is never enabled.
use auth_service::{
    session_check_service::{SessionCheckError, SessionCheckService},
    session_check_types::{
        SessionCheckBatchRequest, SessionCheckInputError, SessionCheckRequest, MAX_SESSION_CHECKS,
    },
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use uuid::Uuid;

async fn fixture(
    pool: &PgPool,
    status: &str,
    verified: bool,
    expired: bool,
) -> SessionCheckRequest {
    let account_id: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(email,status,email_verified_at)
         VALUES($1,$2,CASE WHEN $3 THEN statement_timestamp() END) RETURNING id",
    )
    .bind(format!("{}@example.invalid", Uuid::new_v4()))
    .bind(status)
    .bind(verified)
    .fetch_one(pool)
    .await
    .unwrap();
    // Fixed elapsed intervals, no real waiting and no credential or open_session() needed.
    let sid = sqlx::query_scalar(
        "INSERT INTO auth.sessions(account_id,client_type,created_at,expires_at)
         VALUES($1,'desktop',statement_timestamp()-interval '2 days',
                statement_timestamp()+CASE WHEN $2 THEN interval '-1 day' ELSE interval '1 day' END)
         RETURNING id",
    )
    .bind(account_id)
    .bind(expired)
    .fetch_one(pool)
    .await
    .unwrap();
    SessionCheckRequest { sid, account_id }
}

async fn snapshot(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
            'accounts',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM auth.accounts a),
            'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM auth.sessions s),
            'refresh',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM auth.refresh_tokens r),
            'outbox',(SELECT jsonb_agg(to_jsonb(o) ORDER BY event_id) FROM auth.session_revocation_outbox o))",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn mixed_batch_matches_individual_rules_and_never_writes(pool: PgPool) {
    let active = fixture(&pool, "active", true, false).await;
    let expired = fixture(&pool, "active", true, true).await;
    let revoked = fixture(&pool, "active", true, false).await;
    sqlx::query("UPDATE auth.sessions SET revoked_at=statement_timestamp(),revoke_reason='logout' WHERE id=$1")
        .bind(revoked.sid).execute(&pool).await.unwrap();
    let unverified = fixture(&pool, "active", false, false).await;
    let mut requests = vec![
        active,
        expired,
        revoked,
        SessionCheckRequest {
            sid: Uuid::new_v4(),
            account_id: active.account_id,
        },
        SessionCheckRequest {
            sid: active.sid,
            account_id: expired.account_id,
        },
        unverified,
    ];
    for status in ["locked", "suspended", "deleted"] {
        requests.push(fixture(&pool, status, true, false).await);
    }
    while requests.len() < MAX_SESSION_CHECKS {
        requests.push(SessionCheckRequest {
            sid: Uuid::new_v4(),
            ..active
        });
    }
    // Include refresh state in the write-safety check, even though checks never read it.
    sqlx::query("INSERT INTO auth.refresh_tokens(session_id,token_hash,expires_at) SELECT id,$2,expires_at FROM auth.sessions WHERE id=$1")
        .bind(active.sid).bind(vec![42u8;32]).execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    // All connections used by the service are read-only: any accidental DML must fail.
    let readonly = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET default_transaction_read_only=on")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let service = SessionCheckService::new(readonly.clone());
    let batch = service
        .check_batch(SessionCheckBatchRequest {
            sessions: requests.clone(),
        })
        .await
        .unwrap();
    assert_eq!(batch.results.len(), requests.len());
    for (index, (request, result)) in requests.iter().zip(&batch.results).enumerate() {
        assert_eq!(result.sid, request.sid);
        assert_eq!(result.account_id, request.account_id);
        assert_eq!(result.active, index == 0);
        assert_eq!(result.session_expires_at.is_some(), index == 0);
        assert_eq!(&service.check(*request).await.unwrap(), result);
        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 4);
    }
    assert_eq!(snapshot(&pool).await, before);
    readonly.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn allowed_account_state_is_rechecked_on_every_query(pool: PgPool) {
    let request = fixture(&pool, "active", true, false).await;
    let service = SessionCheckService::new(pool.clone());
    assert!(service.check(request).await.unwrap().active);
    sqlx::query("UPDATE auth.accounts SET status='locked' WHERE id=$1")
        .bind(request.account_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!service.check(request).await.unwrap().active);
    sqlx::query("UPDATE auth.accounts SET status='active',email_verified_at=NULL WHERE id=$1")
        .bind(request.account_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!service.check(request).await.unwrap().active);
}

#[tokio::test]
async fn invalid_batches_are_rejected_before_database_access() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:1/test")
        .unwrap();
    pool.close().await;
    let service = SessionCheckService::new(pool);
    let request = SessionCheckRequest {
        sid: Uuid::new_v4(),
        account_id: Uuid::new_v4(),
    };
    for (sessions, error) in [
        (vec![], SessionCheckInputError::EmptyBatch),
        (
            vec![request, request],
            SessionCheckInputError::DuplicatePair,
        ),
        (
            vec![request; MAX_SESSION_CHECKS + 1],
            SessionCheckInputError::BatchTooLarge,
        ),
    ] {
        assert!(
            matches!(service.check_batch(SessionCheckBatchRequest { sessions }).await,
            Err(SessionCheckError::InvalidInput(actual)) if actual == error)
        );
    }
    assert!(matches!(
        service.check(request).await,
        Err(SessionCheckError::Database(sqlx::Error::PoolClosed))
    ));
    assert!(matches!(
        service
            .check_batch(SessionCheckBatchRequest {
                sessions: vec![request]
            })
            .await,
        Err(SessionCheckError::Database(sqlx::Error::PoolClosed))
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn pool_timeout_and_sql_errors_fail_entire_operation(pool: PgPool) {
    let active = fixture(&pool, "active", true, false).await;
    let missing = SessionCheckRequest {
        sid: Uuid::new_v4(),
        ..active
    };
    let constrained = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(1))
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let held = constrained.acquire().await.unwrap();
    let service = SessionCheckService::new(constrained.clone());
    assert!(matches!(
        service.check(active).await,
        Err(SessionCheckError::Database(sqlx::Error::PoolTimedOut))
    ));
    assert!(matches!(
        service
            .check_batch(SessionCheckBatchRequest {
                sessions: vec![active, missing]
            })
            .await,
        Err(SessionCheckError::Database(sqlx::Error::PoolTimedOut))
    ));
    drop(held);
    constrained.close().await;
    // Break only this sqlx::test database. fetch_all must discard any partial rows.
    sqlx::query("ALTER TABLE auth.sessions RENAME TO unavailable_sessions")
        .execute(&pool)
        .await
        .unwrap();
    let service = SessionCheckService::new(pool);
    assert!(matches!(
        service.check(active).await,
        Err(SessionCheckError::Database(_))
    ));
    assert!(matches!(
        service
            .check_batch(SessionCheckBatchRequest {
                sessions: vec![active, missing]
            })
            .await,
        Err(SessionCheckError::Database(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn database_statement_timeout_is_not_an_inactive_session(pool: PgPool) {
    let active = fixture(&pool, "active", true, false).await;
    let limited = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET statement_timeout='50ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let service = SessionCheckService::new(limited.clone());
    assert!(service.check(active).await.unwrap().active);
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE auth.sessions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    for result in [
        service.check(active).await.map(|r| vec![r]),
        service
            .check_batch(SessionCheckBatchRequest {
                sessions: vec![active],
            })
            .await
            .map(|r| r.results),
    ] {
        assert!(
            matches!(result, Err(SessionCheckError::Database(sqlx::Error::Database(error)))
            if error.code().as_deref() == Some("57014"))
        );
    }
    blocker.rollback().await.unwrap();
    assert!(service.check(active).await.unwrap().active);
    limited.close().await;
}
