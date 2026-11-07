use sqlx::{postgres::PgQueryResult, PgPool};
use uuid::Uuid;

fn assert_database_rejection(result: Result<PgQueryResult, sqlx::Error>, expected_code: &str) {
    match result {
        Err(sqlx::Error::Database(error)) => {
            assert_eq!(error.code().as_deref(), Some(expected_code))
        }
        Err(_) => panic!("Expected a database constraint or trigger rejection"),
        Ok(_) => panic!("Database accepted an invalid session mutation"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn session_lifetime_identity_and_final_revocation_are_enforced(pool: PgPool) {
    let account_id: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(
            email
        ) VALUES(
            'constraints@example.invalid'
        ) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_database_rejection(
        sqlx::query("SELECT auth.open_session($1,'desktop', interval '21 days')")
            .bind(account_id)
            .execute(&pool)
            .await,
        "P0001",
    );
    let session_id: Uuid =
        sqlx::query_scalar("SELECT auth.open_session($1,'desktop', interval '20 days')")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_database_rejection(
        sqlx::query(
            "UPDATE auth.sessions SET expires_at=expires_at+interval '1 second' WHERE id=$1",
        )
        .bind(session_id)
        .execute(&pool)
        .await,
        "P0001",
    );

    assert_database_rejection(
        sqlx::query("UPDATE auth.sessions SET client_type='mobile' WHERE id=$1")
            .bind(session_id)
            .execute(&pool)
            .await,
        "P0001",
    );

    sqlx::query(
        "UPDATE auth.sessions SET revoked_at=clock_timestamp(),revoke_reason='logout' WHERE id=$1",
    )
    .bind(session_id)
    .execute(&pool)
    .await
    .unwrap();
    assert_database_rejection(
        sqlx::query("UPDATE auth.sessions SET revoked_at=NULL,revoke_reason=NULL WHERE id=$1")
            .bind(session_id)
            .execute(&pool)
            .await,
        "P0001",
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn absolute_lifetime_rejects_calendar_days_that_add_an_extra_dst_hour(pool: PgPool) {
    let account_id: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(
            email
        ) VALUES(
            'dst@example.invalid'
        ) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    // Fall-back adds 1 hour to twenty calendar days. Exact timestamps make this reproducible.

    assert_database_rejection(
        sqlx::query(
            "INSERT INTO auth.sessions(
                account_id,
                client_type,
                created_at,
                expires_at
            ) VALUES(
                $1,
                'desktop',
                '2026-10-23 12:00:00-04',
                '2026-11-12 12:00:00-05')",
        )
        .bind(account_id)
        .execute(&pool)
        .await,
        "23514",
    );

    sqlx::query(
        "INSERT INTO auth.sessions(
            account_id,client_type,
            created_at,expires_at
        ) VALUES(
            $1,
            'desktop',
            '2026-10-23 12:00:00-04',
            '2026-11-12 11:00:00-05')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .unwrap();
}
