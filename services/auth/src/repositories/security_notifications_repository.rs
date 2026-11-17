use sqlx::PgPool;
use uuid::Uuid;

use super::sessions_repository::Tx;

#[derive(sqlx::FromRow)]
pub struct SecurityNotification {
    pub id: Uuid,
    pub recipient_email: String,
    pub attempt_count: i32,
}

#[derive(Clone)]
pub struct SecurityNotificationsRepository {
    pub pool: PgPool,
}

impl SecurityNotificationsRepository {
    pub(super) async fn enqueue(
        tx: &mut Tx<'_>,
        event_id: Uuid,
        account_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
                INSERT INTO auth.security_notification_outbox(event_id, recipient_email)
                SELECT $1, email FROM auth.accounts WHERE id = $2 AND email_verified_at IS NOT NULL
                ON CONFLICT (event_id) DO NOTHING
                "#,
        )
        .bind(event_id)
        .bind(account_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn claim(&self) -> Result<Option<SecurityNotification>, sqlx::Error> {
        sqlx::query_as(
            r#"
            WITH candidate AS (
                SELECT n.id FROM auth.security_notification_outbox n
                JOIN auth.account_security_events e ON e.id = n.event_id
                JOIN auth.accounts a ON a.id = e.account_id
                WHERE n.delivered_at IS NULL AND n.attempt_count < 10 AND n.next_attempt_at <= clock_timestamp()
                    AND a.email = n.recipient_email AND a.email_verified_at IS NOT NULL
                    AND e.event_type = 'google_identity_linked'
                ORDER BY n.next_attempt_at, n.id LIMIT 1 FOR UPDATE OF n SKIP LOCKED
            )
            UPDATE auth.security_notification_outbox n SET attempt_count = attempt_count + 1,
                last_attempt_at = clock_timestamp(), next_attempt_at = clock_timestamp() + interval '60 seconds',
                last_error_kind = NULL
            FROM candidate WHERE n.id = candidate.id RETURNING n.id, n.recipient_email::text, n.attempt_count
            "#,
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delivered(
        &self,
        notification: &SecurityNotification,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE auth.security_notification_outbox SET delivered_at = clock_timestamp(), last_error_kind = NULL
            WHERE id = $1 AND attempt_count = $2 AND delivered_at IS NULL
            "#,
        )
        .bind(notification.id)
        .bind(notification.attempt_count)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn retry(
        &self,
        notification: &SecurityNotification,
        error_kind: &str,
    ) -> Result<bool, sqlx::Error> {
        let delay_seconds = retry_delay_seconds(notification.attempt_count);
        let result = sqlx::query(
            r#"
            UPDATE auth.security_notification_outbox SET last_error_kind = $3,
                next_attempt_at = clock_timestamp() + make_interval(secs => $4)
            WHERE id = $1 AND attempt_count = $2 AND delivered_at IS NULL
            "#,
        )
        .bind(notification.id)
        .bind(notification.attempt_count)
        .bind(error_kind)
        .bind(f64::from(delay_seconds))
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }
}

fn retry_delay_seconds(attempt_count: i32) -> u32 {
    const INITIAL_DELAY_SECONDS: u32 = 30;
    const MAX_DELAY_SECONDS: u32 = 3600;
    const MAX_BACKOFF_EXPONENT: i32 = 7;

    let exponent = attempt_count
        .saturating_sub(1)
        .clamp(0, MAX_BACKOFF_EXPONENT) as u32;
    INITIAL_DELAY_SECONDS
        .saturating_mul(2u32.saturating_pow(exponent))
        .min(MAX_DELAY_SECONDS)
}

#[cfg(test)]
mod tests {
    use super::retry_delay_seconds;

    #[test]
    fn retry_backoff_is_bounded_for_every_attempt_count() {
        for (attempt_count, expected_seconds) in [
            (i32::MIN, 30),
            (0, 30),
            (1, 30),
            (2, 60),
            (7, 1920),
            (8, 3600),
            (i32::MAX, 3600),
        ] {
            assert_eq!(retry_delay_seconds(attempt_count), expected_seconds);
        }
    }
}
