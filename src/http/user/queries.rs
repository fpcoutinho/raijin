use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{ThemePreference, User};

use super::schema::UpdateProfileRequest;

pub async fn update_profile(
    pool: &PgPool,
    user_id: Uuid,
    body: &UpdateProfileRequest,
) -> Result<Option<User>, sqlx::Error> {
    let (set_full_name, full_name) = clearable(&body.full_name);
    let (set_title, professional_title) = clearable(&body.professional_title);
    let (set_avatar, avatar_url) = clearable(&body.avatar_url);

    sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET full_name = CASE WHEN $2::bool THEN $3::text ELSE full_name END,
            professional_title = CASE WHEN $4::bool THEN $5::text ELSE professional_title END,
            avatar_url = CASE WHEN $6::bool THEN $7::text ELSE avatar_url END,
            theme_preference = COALESCE($8::theme_preference, theme_preference)
        WHERE id = $1
        RETURNING
            id, email, password_hash, google_id, avatar_url,
            full_name, professional_title,
            theme_preference AS "theme_preference: ThemePreference",
            created_at, updated_at
        "#,
        user_id,
        set_full_name,
        full_name,
        set_title,
        professional_title,
        set_avatar,
        avatar_url,
        body.theme_preference as Option<ThemePreference>,
    )
    .fetch_optional(pool)
    .await
}

pub async fn update_password_hash(
    pool: &PgPool,
    user_id: Uuid,
    password_hash: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE users SET password_hash = $2 WHERE id = $1",
        user_id,
        password_hash,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

fn clearable(field: &Option<Option<String>>) -> (bool, Option<&str>) {
    match field {
        Some(value) => (true, value.as_deref().map(str::trim)),
        None => (false, None),
    }
}
