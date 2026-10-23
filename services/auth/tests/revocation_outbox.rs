use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn revocation_intent_is_atomic_public_and_once_per_session(pool: PgPool) {
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(email) VALUES('outbox@example.invalid') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let sid: Uuid = sqlx::query_scalar("SELECT auth.open_session($1,'desktop',interval '20 days')")
        .bind(account)
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "UPDATE auth.sessions SET revoked_at=clock_timestamp(),revoke_reason='logout' WHERE id=$1",
    )
    .bind(sid)
    .execute(&mut *tx)
    .await
    .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.session_revocation_outbox")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 1);
    tx.rollback().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.session_revocation_outbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("UPDATE auth.sessions SET revoked_at=clock_timestamp(),revoke_reason='logout' WHERE id=$1 AND revoked_at IS NULL").bind(sid).execute(&pool).await.unwrap();
    sqlx::query("UPDATE auth.sessions SET last_seen_at=clock_timestamp() WHERE id=$1")
        .bind(sid)
        .execute(&pool)
        .await
        .unwrap();
    let events: Vec<serde_json::Value> =
        sqlx::query_scalar("SELECT event FROM auth.session_revocation_outbox")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event["type"], "session.revoked");
    assert_eq!(event["version"], 1);
    assert_eq!(event["payload"]["reason"], "logout");
    assert_eq!(event["payload"]["sid"], sid.to_string());
    assert_eq!(event["payload"]["account_id"], account.to_string());
    assert_eq!(event["payload"].as_object().unwrap().len(), 6);
    assert!(Uuid::parse_str(event["event_id"].as_str().unwrap()).is_ok());
}
