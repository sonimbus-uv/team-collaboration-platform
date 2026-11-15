use sha2::{Digest, Sha256};
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgConnection, PgPool,
};
use uuid::Uuid;

fn assert_sqlstate<T>(result: Result<T, sqlx::Error>, expected: &str) {
    match result {
        Err(sqlx::Error::Database(error)) => assert_eq!(error.code().as_deref(), Some(expected)),
        Err(_) => panic!("Expected a database constraint or trigger error"),
        Ok(_) => panic!("Database accepted an invalid authentication mutation"),
    }
}

fn random_digest() -> Vec<u8> {
    Sha256::digest(Uuid::new_v4().as_bytes()).to_vec()
}

async fn create_account(pool: &PgPool, verified: bool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO auth.accounts(email, email_verified_at)
         VALUES (CASE WHEN $2 THEN $1::text END, CASE WHEN $2 THEN statement_timestamp() END) RETURNING id",
    )
    .bind(format!("{}@example.invalid", Uuid::new_v4()))
    .bind(verified)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn create_identity(pool: &PgPool, account_id: Uuid, provider: &str) -> (Uuid, String) {
    let subject = Uuid::new_v4().to_string();
    let id = sqlx::query_scalar(
        "INSERT INTO auth.oauth_identities(account_id, provider, provider_subject)
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(account_id)
    .bind(provider)
    .bind(&subject)
    .fetch_one(pool)
    .await
    .unwrap();
    (id, subject)
}

async fn open_legacy_session(pool: &PgPool, account_id: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT auth.open_session($1, 'desktop', interval '20 days')")
        .bind(account_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn create_login_attempt(pool: &PgPool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO auth.federated_auth_attempts
         (purpose, client_type, redirect_target, initiator_secret_hash, launch_ticket_hash,
          oauth_state_hash, oidc_nonce_hash, code_challenge)
         VALUES ('login', 'desktop', 'desktop', $1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(random_digest())
    .bind(random_digest())
    .bind(random_digest())
    .bind(random_digest())
    .bind(random_digest())
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn mark_attempt_verified(pool: &PgPool, attempt_id: Uuid, subject: &str) {
    sqlx::query(
        "UPDATE auth.federated_auth_attempts
         SET status = 'authorizing', authorization_started_at = clock_timestamp() WHERE id = $1",
    )
    .bind(attempt_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "WITH observed AS (SELECT clock_timestamp() AS instant)
         UPDATE auth.federated_auth_attempts SET status = 'verified', provider_subject = $2,
             identity_verified_at = observed.instant,
             identity_valid_until = observed.instant + interval '1 hour',
             completion_code_hash = $3, completion_issued_at = observed.instant,
             completion_expires_at = observed.instant + interval '60 seconds'
         FROM observed WHERE id = $1",
    )
    .bind(attempt_id)
    .bind(subject)
    .bind(random_digest())
    .execute(pool)
    .await
    .unwrap();
}

#[derive(Clone, Copy)]
struct LoginResult {
    account_id: Uuid,
    identity_id: Uuid,
    session_id: Uuid,
}

async fn insert_google_session(pool: &PgPool, account_id: Uuid, identity_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO auth.sessions(account_id, client_type, expires_at,
                                   authentication_method, oauth_identity_id)
         VALUES ($1, 'mobile', statement_timestamp() + interval '1 day', 'google', $2)
         RETURNING id",
    )
    .bind(account_id)
    .bind(identity_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn consume_login(
    connection: &mut PgConnection,
    attempt_id: Uuid,
    result: LoginResult,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE auth.federated_auth_attempts SET status = 'consumed', terminal_at = clock_timestamp(),
             result_account_id = $2, result_identity_id = $3, result_session_id = $4
         WHERE id = $1 AND status = 'verified' AND completion_expires_at > clock_timestamp()",
    )
    .bind(attempt_id)
    .bind(result.account_id)
    .bind(result.identity_id)
    .bind(result.session_id)
    .execute(connection)
    .await
    .map(|result| result.rows_affected())
}

#[sqlx::test(migrations = false)]
async fn expansion_preserves_preexisting_accounts_credentials_and_sessions(pool: PgPool) {
    for migration in [
        include_str!("../migrations/0001_initial_auth_schema.sql"),
        include_str!("../migrations/0002_session_revocation_outbox.sql"),
        include_str!("../migrations/0003_session_lifetime_and_locking.sql"),
        include_str!("../migrations/0004_session_elapsed_duration.sql"),
        include_str!("../migrations/0005_registration_attempts.sql"),
        include_str!("../migrations/0006_password_recovery.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    let verified_account = create_account(&pool, true).await;
    let pending_account: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(email) VALUES ('pending-legacy@example.invalid') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE auth.accounts SET status = 'locked' WHERE id = $1")
        .bind(pending_account)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO auth.password_credentials(account_id, password_hash) VALUES ($1, 'fixture-hash')")
        .bind(verified_account)
        .execute(&pool)
        .await
        .unwrap();
    open_legacy_session(&pool, verified_account).await;
    let session_id = open_legacy_session(&pool, verified_account).await;
    let snapshot = "SELECT jsonb_build_object(
        'accounts', (SELECT jsonb_agg(to_jsonb(a) - 'contact_email_version' ORDER BY id) FROM auth.accounts a),
        'passwords', (SELECT jsonb_agg(to_jsonb(p) ORDER BY account_id) FROM auth.password_credentials p),
        'sessions', (SELECT jsonb_agg(to_jsonb(s) - ARRAY['authentication_method', 'oauth_identity_id',
            'password_credential_version', 'last_authenticated_at', 'reauthentication_method',
            'reauthentication_password_version', 'reauthentication_oauth_identity_id'] ORDER BY id) FROM auth.sessions s),
        'outbox', (SELECT jsonb_agg(to_jsonb(o) ORDER BY event_id) FROM auth.session_revocation_outbox o))";
    let before: serde_json::Value = sqlx::query_scalar(snapshot).fetch_one(&pool).await.unwrap();
    for migration in [
        include_str!("../migrations/0007_contact_email_support.sql"),
        include_str!("../migrations/0008_session_authentication_evidence.sql"),
        include_str!("../migrations/0009_federated_authentication_attempts.sql"),
        include_str!("../migrations/0010_account_security_events.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    let after: serde_json::Value = sqlx::query_scalar(snapshot).fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    let evidence: (String, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT authentication_method, last_authenticated_at FROM auth.sessions WHERE id = $1",
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(evidence.0, "legacy");
    assert!(evidence.1.is_none());
    assert_sqlstate(
        sqlx::query("INSERT INTO auth.accounts(email) VALUES (NULL)")
            .execute(&pool)
            .await,
        "23502",
    );
    let email: String = sqlx::query_scalar("SELECT email::text FROM auth.accounts WHERE id = $1")
        .bind(verified_account)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_sqlstate(
        sqlx::query("INSERT INTO auth.accounts(email) VALUES ($1)")
            .bind(email.to_uppercase())
            .execute(&pool)
            .await,
        "23505",
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn proposals_do_not_reserve_email_and_replacements_invalidate_old_proofs(pool: PgPool) {
    let first = create_account(&pool, false).await;
    let second = create_account(&pool, false).await;
    for account_id in [first, second] {
        sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'pending@example.invalid')")
            .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    }
    let attempt_id: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac)
         VALUES ($1, 'pending@example.invalid', 1, $2) RETURNING id",
    )
        .bind(first)
        .bind(random_digest())
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE auth.account_email_proposals SET email = 'other@example.invalid' WHERE account_id = $1")
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();
    let invalidated: bool = sqlx::query_scalar("SELECT invalidated_at IS NOT NULL FROM auth.contact_email_verification_attempts WHERE id = $1")
        .bind(attempt_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(invalidated);
    assert_sqlstate(sqlx::query("UPDATE auth.contact_email_verification_attempts SET invalidated_at = NULL WHERE id = $1")
        .bind(attempt_id)
        .execute(&pool)
        .await, "23514");
    sqlx::query("DELETE FROM auth.account_email_proposals WHERE account_id = $1")
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();
    let version: i64 = sqlx::query_scalar("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'pending@example.invalid') RETURNING version")
        .bind(first)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, 4);
    assert_sqlstate(
        sqlx::query("UPDATE auth.accounts SET contact_email_version = 1 WHERE id = $1")
            .bind(first)
            .execute(&pool)
            .await,
        "23514",
    );
    assert_sqlstate(sqlx::query("INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac) VALUES ($1, 'pending@example.invalid', 1, $2)")
        .bind(first)
        .bind(random_digest())
        .execute(&pool)
        .await, "23514");
}

#[sqlx::test(migrations = "./migrations")]
async fn contact_attempts_enforce_bounds_and_cannot_consume_after_five_failures(pool: PgPool) {
    let account_id = create_account(&pool, false).await;
    sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'contact@example.invalid')")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    for invalid_mac in [vec![0u8; 31], vec![0u8; 33]] {
        assert_sqlstate(sqlx::query("INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac) VALUES ($1, 'contact@example.invalid', 1, $2)")
            .bind(account_id)
        .bind(invalid_mac)
        .execute(&pool)
        .await, "23514");
    }
    let attempt_id: Uuid = sqlx::query_scalar("INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac) VALUES ($1, 'contact@example.invalid', 1, $2) RETURNING id")
        .bind(account_id)
        .bind(random_digest())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_sqlstate(sqlx::query("INSERT INTO auth.contact_email_verification_attempts(account_id, email, contact_email_version, code_mac) VALUES ($1, 'contact@example.invalid', 1, $2)")
        .bind(account_id)
        .bind(random_digest())
        .execute(&pool)
        .await, "23505");
    assert_sqlstate(sqlx::query("UPDATE auth.contact_email_verification_attempts SET code_mac = $2, code_version = 1, last_sent_at = clock_timestamp() WHERE id = $1")
        .bind(attempt_id)
        .bind(random_digest())
        .execute(&pool)
        .await, "23514");
    sqlx::query(
        "UPDATE auth.contact_email_verification_attempts SET failed_attempts = 5 WHERE id = $1",
    )
    .bind(attempt_id)
    .execute(&pool)
    .await
    .unwrap();
    assert_sqlstate(
        sqlx::query(
            "UPDATE auth.contact_email_verification_attempts SET failed_attempts = 0 WHERE id = $1",
        )
        .bind(attempt_id)
        .execute(&pool)
        .await,
        "23514",
    );
    assert_sqlstate(sqlx::query("UPDATE auth.contact_email_verification_attempts SET consumed_at = clock_timestamp() WHERE id = $1")
        .bind(attempt_id)
        .execute(&pool)
        .await, "23514");
}

#[sqlx::test(migrations = "./migrations")]
async fn session_evidence_is_account_bound_immutable_and_not_refreshed_by_activity(pool: PgPool) {
    let account_id = create_account(&pool, true).await;
    let session_id = open_legacy_session(&pool, account_id).await;
    let other_account = create_account(&pool, true).await;
    let (other_identity, _) = create_identity(&pool, other_account, "google").await;
    assert_sqlstate(sqlx::query("INSERT INTO auth.sessions(account_id, client_type, expires_at, authentication_method, oauth_identity_id) VALUES ($1, 'mobile', statement_timestamp() + interval '1 day', 'google', $2)")
        .bind(account_id)
        .bind(other_identity)
        .execute(&pool)
        .await, "23503");
    let (wrong_provider, _) = create_identity(&pool, account_id, "other").await;
    assert_sqlstate(sqlx::query("INSERT INTO auth.sessions(account_id, client_type, expires_at, authentication_method, oauth_identity_id) VALUES ($1, 'mobile', statement_timestamp() + interval '1 day', 'google', $2)")
        .bind(account_id)
        .bind(wrong_provider)
        .execute(&pool)
        .await, "23503");
    assert_sqlstate(sqlx::query("INSERT INTO auth.sessions(account_id, client_type, expires_at, authentication_method) VALUES ($1, 'mobile', statement_timestamp() + interval '1 day', 'password')")
        .bind(account_id)
        .execute(&pool)
        .await, "23514");
    assert_sqlstate(sqlx::query("UPDATE auth.sessions SET last_authenticated_at = clock_timestamp(), reauthentication_method = 'password' WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await, "23514");
    sqlx::query("UPDATE auth.sessions SET last_authenticated_at = clock_timestamp(), reauthentication_method = 'password', reauthentication_password_version = 1 WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();
    let before: DateTime<Utc> =
        sqlx::query_scalar("SELECT last_authenticated_at FROM auth.sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE auth.sessions SET last_seen_at = clock_timestamp() WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();
    let after: DateTime<Utc> =
        sqlx::query_scalar("SELECT last_authenticated_at FROM auth.sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    assert_sqlstate(sqlx::query("UPDATE auth.sessions SET authentication_method = 'password', password_credential_version = 1 WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await, "P0001");
    sqlx::query("UPDATE auth.sessions SET revoked_at = clock_timestamp(), revoke_reason = 'logout' WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_sqlstate(
        sqlx::query(
            "UPDATE auth.sessions SET last_authenticated_at = clock_timestamp() WHERE id = $1",
        )
        .bind(session_id)
        .execute(&pool)
        .await,
        "23514",
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn federated_result_is_immutable_and_only_one_concurrent_exchange_consumes(pool: PgPool) {
    let account_id = create_account(&pool, true).await;
    let (identity_id, subject) = create_identity(&pool, account_id, "google").await;
    let session_id = insert_google_session(&pool, account_id, identity_id).await;
    let attempt_id = create_login_attempt(&pool).await;
    assert_sqlstate(
        sqlx::query("UPDATE auth.federated_auth_attempts SET status = 'verified' WHERE id = $1")
            .bind(attempt_id)
            .execute(&pool)
            .await,
        "23514",
    );
    mark_attempt_verified(&pool, attempt_id, &subject).await;
    assert_sqlstate(sqlx::query("UPDATE auth.federated_auth_attempts SET status = 'failed', terminal_at = clock_timestamp(), provider_subject = 'replacement' WHERE id = $1")
        .bind(attempt_id)
        .execute(&pool)
        .await, "23514");
    let result = LoginResult {
        account_id,
        identity_id,
        session_id,
    };
    let first = async {
        let mut connection = pool.acquire().await.unwrap();
        consume_login(&mut connection, attempt_id, result)
            .await
            .unwrap()
    };
    let second = async {
        let mut connection = pool.acquire().await.unwrap();
        consume_login(&mut connection, attempt_id, result)
            .await
            .unwrap()
    };
    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(first, second)
    })
    .await
    .unwrap();
    assert_eq!(first + second, 1);
    assert_sqlstate(sqlx::query("UPDATE auth.federated_auth_attempts SET status = 'verified', terminal_at = NULL, result_account_id = NULL, result_identity_id = NULL, result_session_id = NULL WHERE id = $1")
        .bind(attempt_id)
        .execute(&pool)
        .await, "23514");
}

#[sqlx::test(migrations = "./migrations")]
async fn finalization_rollback_restores_proof_and_removes_new_account_identity_and_session(
    pool: PgPool,
) {
    let attempt_id = create_login_attempt(&pool).await;
    let subject = Uuid::new_v4().to_string();
    mark_attempt_verified(&pool, attempt_id, &subject).await;
    let mut transaction = pool.begin().await.unwrap();
    let account_id: Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email, email_verified_at) VALUES ($1, clock_timestamp()) RETURNING id")
        .bind(format!("{}@example.invalid", Uuid::new_v4()))
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    let identity_id: Uuid = sqlx::query_scalar("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', $2) RETURNING id")
        .bind(account_id)
        .bind(subject)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    let session_id: Uuid = sqlx::query_scalar("INSERT INTO auth.sessions(account_id, client_type, expires_at, authentication_method, oauth_identity_id) VALUES ($1, 'desktop', statement_timestamp() + interval '1 day', 'google', $2) RETURNING id")
        .bind(account_id)
        .bind(identity_id)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    assert_eq!(
        consume_login(
            &mut transaction,
            attempt_id,
            LoginResult {
                account_id,
                identity_id,
                session_id
            }
        )
        .await
        .unwrap(),
        1
    );
    transaction.rollback().await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT status FROM auth.federated_auth_attempts WHERE id = $1")
            .bind(attempt_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "verified");
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.accounts WHERE id = $1")
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(accounts, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn notification_outbox_accepts_only_verified_contact_and_deduplicates_link_events(
    pool: PgPool,
) {
    let account_id = create_account(&pool, true).await;
    let (identity_id, _) = create_identity(&pool, account_id, "google").await;
    let event_id: Uuid = sqlx::query_scalar("INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2) RETURNING id")
        .bind(account_id)
        .bind(identity_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_sqlstate(sqlx::query("INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2)")
        .bind(account_id)
        .bind(identity_id)
        .execute(&pool)
        .await, "23505");
    assert_sqlstate(sqlx::query("INSERT INTO auth.security_notification_outbox(event_id, recipient_email) VALUES ($1, 'unverified-provider@example.invalid')")
        .bind(event_id)
        .execute(&pool)
        .await, "23514");
    let outbox_id: Uuid = sqlx::query_scalar("INSERT INTO auth.security_notification_outbox(event_id, recipient_email) SELECT $1, email FROM auth.accounts WHERE id = $2 RETURNING id")
        .bind(event_id)
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_sqlstate(sqlx::query("INSERT INTO auth.security_notification_outbox(event_id, recipient_email) SELECT $1, email FROM auth.accounts WHERE id = $2")
        .bind(event_id)
        .bind(account_id)
        .execute(&pool)
        .await, "23505");
    assert_sqlstate(sqlx::query("UPDATE auth.security_notification_outbox SET recipient_email = 'other@example.invalid' WHERE id = $1")
        .bind(outbox_id)
        .execute(&pool)
        .await, "23514");
    let pending = create_account(&pool, false).await;
    let (pending_identity, _) = create_identity(&pool, pending, "google").await;
    let pending_event: Uuid = sqlx::query_scalar("INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2) RETURNING id")
        .bind(pending)
        .bind(pending_identity)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_sqlstate(sqlx::query("INSERT INTO auth.security_notification_outbox(event_id, recipient_email) SELECT $1, email FROM auth.accounts WHERE id = $2")
        .bind(pending_event)
        .bind(pending)
        .execute(&pool)
        .await, "23514");
}

#[sqlx::test(migrations = "./migrations")]
async fn link_attempts_bind_initiating_session_and_committed_security_event_to_account(
    pool: PgPool,
) {
    let first = create_account(&pool, true).await;
    let second = create_account(&pool, true).await;
    let first_session = open_legacy_session(&pool, first).await;
    let second_session = open_legacy_session(&pool, second).await;
    let insert_attempt = "INSERT INTO auth.federated_auth_attempts
        (purpose, client_type, redirect_target, initiator_secret_hash, launch_ticket_hash,
         oauth_state_hash, oidc_nonce_hash, code_challenge, initiator_account_id,
         initiator_session_id, initiator_authenticated_at, initiator_authentication_method,
         initiator_credential_version)
        VALUES ('link', 'desktop', 'desktop', $1, $2, $3, $4, $5, $6, $7,
                statement_timestamp(), 'password', 1) RETURNING id";
    assert_sqlstate(
        sqlx::query_scalar::<_, Uuid>(insert_attempt)
            .bind(random_digest())
            .bind(random_digest())
            .bind(random_digest())
            .bind(random_digest())
            .bind(random_digest())
            .bind(first)
            .bind(second_session)
            .fetch_one(&pool)
            .await,
        "23503",
    );
    let attempt_id: Uuid = sqlx::query_scalar(insert_attempt)
        .bind(random_digest())
        .bind(random_digest())
        .bind(random_digest())
        .bind(random_digest())
        .bind(random_digest())
        .bind(first)
        .bind(first_session)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (first_identity, subject) = create_identity(&pool, first, "google").await;
    let (second_identity, _) = create_identity(&pool, second, "google").await;
    mark_attempt_verified(&pool, attempt_id, &subject).await;
    let wrong_event: Uuid = sqlx::query_scalar("INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2) RETURNING id")
        .bind(second)
        .bind(second_identity)
        .fetch_one(&pool)
        .await
        .unwrap();
    let consume = "UPDATE auth.federated_auth_attempts SET status = 'consumed',
        terminal_at = clock_timestamp(), result_account_id = $2,
        result_identity_id = $3, result_security_event_id = $4 WHERE id = $1";
    assert_sqlstate(
        sqlx::query(consume)
            .bind(attempt_id)
            .bind(first)
            .bind(first_identity)
            .bind(wrong_event)
            .execute(&pool)
            .await,
        "23503",
    );
    let event_id: Uuid = sqlx::query_scalar("INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2) RETURNING id")
        .bind(first)
        .bind(first_identity)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query(consume)
            .bind(attempt_id)
            .bind(first)
            .bind(first_identity)
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap()
            .rows_affected(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn federated_attempts_reject_expiration_and_result_identity_substitution(pool: PgPool) {
    let expired: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.federated_auth_attempts
         (purpose, client_type, redirect_target, initiator_secret_hash, launch_ticket_hash,
          oauth_state_hash, oidc_nonce_hash, code_challenge, created_at, expires_at)
         VALUES ('login', 'desktop', 'desktop', $1, $2, $3, $4, $5,
                 statement_timestamp() - interval '11 minutes',
                 statement_timestamp() - interval '1 minute') RETURNING id",
    )
    .bind(random_digest())
    .bind(random_digest())
    .bind(random_digest())
    .bind(random_digest())
    .bind(random_digest())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_sqlstate(sqlx::query("UPDATE auth.federated_auth_attempts SET status = 'authorizing', authorization_started_at = clock_timestamp() WHERE id = $1")
        .bind(expired)
        .execute(&pool)
        .await, "23514");
    let attempt_id = create_login_attempt(&pool).await;
    let account_id = create_account(&pool, true).await;
    let (identity_id, _) = create_identity(&pool, account_id, "google").await;
    let session_id = insert_google_session(&pool, account_id, identity_id).await;
    mark_attempt_verified(&pool, attempt_id, "different-verified-subject").await;
    let mut connection = pool.acquire().await.unwrap();
    assert_sqlstate(
        consume_login(
            &mut connection,
            attempt_id,
            LoginResult {
                account_id,
                identity_id,
                session_id,
            },
        )
        .await,
        "23514",
    );
}
