use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Barrier, task::JoinSet};
use uuid::Uuid;

async fn apply_legacy_migrations(pool: &PgPool) {
    for migration in sqlx::migrate!("./migrations")
        .iter()
        .filter(|migration| migration.version < 11)
    {
        sqlx::raw_sql(migration.sql.as_ref())
            .execute(pool)
            .await
            .unwrap();
    }
}

fn assert_sqlstate<T>(result: Result<T, sqlx::Error>, expected: &str) {
    match result {
        Err(sqlx::Error::Database(error)) => assert_eq!(error.code().as_deref(), Some(expected)),
        Err(_) => panic!("Expected a database constraint error"),
        Ok(_) => panic!("Database accepted an invalid verified contact"),
    }
}

#[sqlx::test(migrations = false)]
async fn cutover_preserves_accounts_and_credentials_without_certifying_pending_addresses(
    pool: PgPool,
) {
    apply_legacy_migrations(&pool).await;
    let verified: Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email, email_verified_at) VALUES ('verified@example.invalid', '2026-01-01T00:00:00Z') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let pending: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(email) VALUES ('pending@example.invalid') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let blocked: Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email, status) VALUES ('blocked@example.invalid', 'locked') RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    for account_id in [verified, pending, blocked] {
        sqlx::query("INSERT INTO auth.password_credentials(account_id, password_hash) VALUES ($1, 'preserved-fixture-hash')")
            .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("SELECT auth.open_session($1, 'desktop', interval '1 day')")
            .bind(account_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', 'preexisting-identity')")
        .bind(pending)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'pending@example.invalid')")
        .bind(pending)
        .execute(&pool)
        .await
        .unwrap();
    let proof: Uuid = sqlx::query_scalar("INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac) VALUES ($1, 'pending@example.invalid', 1, $2) RETURNING id")
        .bind(pending)
        .bind(vec![0x42u8; 32])
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/0011_verified_contact_cutover.sql"
    ))
    .execute(&mut *transaction)
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    let contact: (String, String) = sqlx::query_as("SELECT email::text, to_char(email_verified_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') FROM auth.accounts WHERE id = $1")
        .bind(verified)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        contact,
        ("verified@example.invalid".into(), "2026-01-01".into())
    );
    for (account_id, status) in [(pending, "active"), (blocked, "locked")] {
        let row: (Option<String>, bool, String, String) = sqlx::query_as("SELECT a.email::text, a.email_verified_at IS NULL, a.status, p.password_hash FROM auth.accounts a JOIN auth.password_credentials p ON p.account_id = a.id WHERE a.id = $1")
            .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(row.0.is_none() && row.1);
        assert_eq!(row.2, status);
        assert_eq!(row.3, "preserved-fixture-hash");
        let allowed: bool = sqlx::query_scalar(
            "SELECT account_allowed FROM auth.session_account_status WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(!allowed);
    }
    let version: i64 = sqlx::query_scalar(
        "SELECT version FROM auth.account_email_proposals WHERE account_id = $1",
    )
    .bind(pending)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(version, 1);
    let preserved: bool = sqlx::query_scalar(
        "SELECT invalidated_at IS NULL FROM auth.contact_email_verification_attempts WHERE id = $1",
    )
    .bind(proof)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(preserved);
    let allowed: bool = sqlx::query_scalar(
        "SELECT account_allowed FROM auth.session_account_status WHERE account_id = $1",
    )
    .bind(verified)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(allowed);
}

#[sqlx::test(migrations = false)]
async fn inconsistent_legacy_addresses_abort_cutover_without_partial_changes(pool: PgPool) {
    apply_legacy_migrations(&pool).await;
    sqlx::query("INSERT INTO auth.accounts(email) VALUES (' padded@example.invalid ')")
        .execute(&pool)
        .await
        .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    assert_sqlstate(
        sqlx::raw_sql(include_str!(
            "../migrations/0011_verified_contact_cutover.sql"
        ))
        .execute(&mut *transaction)
        .await,
        "23514",
    );
    transaction.rollback().await.unwrap();
    let nullable: String = sqlx::query_scalar("SELECT is_nullable FROM information_schema.columns WHERE table_schema = 'auth' AND table_name = 'accounts' AND column_name = 'email'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(nullable, "NO");
    let proposals: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.account_email_proposals")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(proposals, 0);
    let address: String = sqlx::query_scalar("SELECT email::text FROM auth.accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(address, " padded@example.invalid ");
}

#[sqlx::test(migrations = "./migrations")]
async fn contact_pair_is_required_and_pending_addresses_are_not_exclusive(pool: PgPool) {
    assert_sqlstate(
        sqlx::query("INSERT INTO auth.accounts(email) VALUES ('unverified@example.invalid')")
            .execute(&pool)
            .await,
        "23514",
    );
    assert_sqlstate(
        sqlx::query("INSERT INTO auth.accounts(email_verified_at) VALUES (clock_timestamp())")
            .execute(&pool)
            .await,
        "23514",
    );
    for _ in 0..2 {
        let account_id: Uuid =
            sqlx::query_scalar("INSERT INTO auth.accounts DEFAULT VALUES RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap();
        sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'shared@example.invalid')")
            .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    }
    let accounts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth.accounts WHERE email IS NULL AND email_verified_at IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(accounts, 2);
}

#[derive(Clone, Copy)]
enum ClaimSource {
    Registration,
    Contact,
    Google,
}

async fn claim_contact(
    pool: PgPool,
    barrier: Arc<Barrier>,
    source: ClaimSource,
    contact_account: Uuid,
    contact_proof: Uuid,
) -> Result<Uuid, sqlx::Error> {
    barrier.wait().await;
    let mut transaction = pool.begin().await?;
    let account_id: Uuid = match source {
        ClaimSource::Contact => contact_account,
        _ => {
            sqlx::query_scalar("INSERT INTO auth.accounts DEFAULT VALUES RETURNING id")
                .fetch_one(&mut *transaction)
                .await?
        }
    };
    match source {
        ClaimSource::Registration => {
            sqlx::query("INSERT INTO auth.password_credentials(account_id, password_hash) VALUES ($1, 'winning-registration-hash')")
                .bind(account_id)
        .execute(&mut *transaction)
        .await?;
        }
        ClaimSource::Google => {
            sqlx::query("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', $2)")
                .bind(account_id)
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *transaction)
        .await?;
        }
        ClaimSource::Contact => {}
    }
    sqlx::query("UPDATE auth.accounts SET email = 'Race@Example.Invalid', email_verified_at = clock_timestamp() WHERE id = $1")
        .bind(account_id)
        .execute(&mut *transaction)
        .await?;
    if matches!(source, ClaimSource::Contact) {
        sqlx::query("UPDATE auth.contact_email_verification_attempts SET consumed_at = clock_timestamp() WHERE id = $1")
            .bind(contact_proof)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(account_id)
}

#[sqlx::test(migrations = "./migrations")]
async fn registration_contact_and_google_share_one_atomic_case_insensitive_owner(pool: PgPool) {
    let contact_account: Uuid =
        sqlx::query_scalar("INSERT INTO auth.accounts DEFAULT VALUES RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'race@example.invalid')")
        .bind(contact_account)
        .execute(&pool)
        .await
        .unwrap();
    let contact_proof: Uuid = sqlx::query_scalar("INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac) VALUES ($1, 'race@example.invalid', 1, $2) RETURNING id")
        .bind(contact_account)
        .bind(vec![0x42u8; 32])
        .fetch_one(&pool)
        .await
        .unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let mut tasks = JoinSet::new();
    for source in [
        ClaimSource::Registration,
        ClaimSource::Contact,
        ClaimSource::Google,
    ] {
        tasks.spawn(claim_contact(
            pool.clone(),
            barrier.clone(),
            source,
            contact_account,
            contact_proof,
        ));
    }
    let winner = tokio::time::timeout(Duration::from_secs(10), async {
        barrier.wait().await;
        let mut winners = Vec::new();
        while let Some(result) = tasks.join_next().await {
            match result.unwrap() {
                Ok(account_id) => winners.push(account_id),
                Err(sqlx::Error::Database(error)) => {
                    assert_eq!(error.code().as_deref(), Some("23505"));
                    assert_eq!(error.constraint(), Some("accounts_email_unique"));
                }
                Err(_) => panic!("Unexpected failure during contact assignment race"),
            }
        }
        assert_eq!(winners.len(), 1);
        winners[0]
    })
    .await
    .unwrap();
    let owners: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth.accounts WHERE email = 'race@example.invalid'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owners, 1);
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(accounts, if winner == contact_account { 1 } else { 2 });
    let consumed: bool = sqlx::query_scalar("SELECT consumed_at IS NOT NULL FROM auth.contact_email_verification_attempts WHERE id = $1")
        .bind(contact_proof)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(consumed, winner == contact_account);
    let partial_credentials: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.password_credentials p JOIN auth.accounts a ON a.id = p.account_id WHERE a.email IS NULL")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(partial_credentials, 0);
    let partial_identities: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.oauth_identities i JOIN auth.accounts a ON a.id = i.account_id WHERE a.email IS NULL")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(partial_identities, 0);
}
