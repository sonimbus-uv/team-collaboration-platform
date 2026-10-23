use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn session_lifetime_identity_and_final_revocation_are_enforced(pool: PgPool) {
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(email) VALUES('constraints@example.invalid') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        sqlx::query("SELECT auth.open_session($1,'desktop',interval '21 days')")
            .bind(account)
            .execute(&pool)
            .await
            .is_err()
    );
    let sid: Uuid = sqlx::query_scalar("SELECT auth.open_session($1,'desktop',interval '20 days')")
        .bind(account)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(sqlx::query(
        "UPDATE auth.sessions SET expires_at=expires_at+interval '1 second' WHERE id=$1"
    )
    .bind(sid)
    .execute(&pool)
    .await
    .is_err());
    sqlx::query(
        "UPDATE auth.sessions SET revoked_at=clock_timestamp(),revoke_reason='logout' WHERE id=$1",
    )
    .bind(sid)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        sqlx::query("UPDATE auth.sessions SET revoked_at=NULL,revoke_reason=NULL WHERE id=$1")
            .bind(sid)
            .execute(&pool)
            .await
            .is_err()
    );
}
