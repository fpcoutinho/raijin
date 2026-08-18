use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{ThemePreference, User};

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT
            id, email, password_hash, google_id, avatar_url,
            full_name, professional_title,
            theme_preference AS "theme_preference: ThemePreference",
            created_at, updated_at
        FROM users
        WHERE email = $1
        "#,
        email,
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT
            id, email, password_hash, google_id, avatar_url,
            full_name, professional_title,
            theme_preference AS "theme_preference: ThemePreference",
            created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_google_id(
    pool: &PgPool,
    google_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT
            id, email, password_hash, google_id, avatar_url,
            full_name, professional_title,
            theme_preference AS "theme_preference: ThemePreference",
            created_at, updated_at
        FROM users
        WHERE google_id = $1
        "#,
        google_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn insert_user_with_password(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    full_name: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, password_hash, full_name)
        VALUES ($1, $2, $3)
        RETURNING
            id, email, password_hash, google_id, avatar_url,
            full_name, professional_title,
            theme_preference AS "theme_preference: ThemePreference",
            created_at, updated_at
        "#,
        email,
        password_hash,
        full_name,
    )
    .fetch_one(pool)
    .await
}

/// Atualiza dados de um usuário já vinculado ao Google. Buscado por google_id
/// (não por email) porque o e-mail primário da conta Google pode mudar.
pub async fn update_google_user(
    pool: &PgPool,
    google_id: &str,
    email: &str,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET email = $2, avatar_url = COALESCE($3, avatar_url)
        WHERE google_id = $1
        RETURNING
            id, email, password_hash, google_id, avatar_url,
            full_name, professional_title,
            theme_preference AS "theme_preference: ThemePreference",
            created_at, updated_at
        "#,
        google_id,
        email,
        avatar_url,
    )
    .fetch_one(pool)
    .await
}

/// Vincula o Google a um e-mail existente (ou cria a conta). O `bool` diz se
/// havia uma senha antes: verificado vence não-verificado, então o Google
/// assume a conta e o `password_hash` é anulado — quem cadastrou senha num
/// e-mail alheio perde o acesso aqui.
pub async fn link_google_to_existing(
    pool: &PgPool,
    email: &str,
    google_id: &str,
    avatar_url: Option<&str>,
) -> Result<(User, bool), sqlx::Error> {
    let row = sqlx::query!(
        r#"
        WITH existing AS (
            SELECT password_hash IS NOT NULL AS had_password
            FROM users
            WHERE email = $1
        ), upserted AS (
            INSERT INTO users (email, google_id, avatar_url)
            VALUES ($1, $2, $3)
            ON CONFLICT (email) DO UPDATE
            SET google_id = EXCLUDED.google_id,
                password_hash = NULL,
                avatar_url = COALESCE(EXCLUDED.avatar_url, users.avatar_url)
            RETURNING
                id, email, password_hash, google_id, avatar_url,
                full_name, professional_title, theme_preference,
                created_at, updated_at
        )
        SELECT
            u.id, u.email, u.password_hash, u.google_id, u.avatar_url,
            u.full_name, u.professional_title,
            u.theme_preference AS "theme_preference: ThemePreference",
            u.created_at, u.updated_at,
            COALESCE((SELECT had_password FROM existing), false) AS "had_password!"
        FROM upserted u
        "#,
        email,
        google_id,
        avatar_url,
    )
    .fetch_one(pool)
    .await?;

    let user = User {
        id: row.id,
        email: row.email,
        password_hash: row.password_hash,
        google_id: row.google_id,
        avatar_url: row.avatar_url,
        full_name: row.full_name,
        professional_title: row.professional_title,
        theme_preference: row.theme_preference,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };
    Ok((user, row.had_password))
}

pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        user_id,
        token_hash,
        expires_at,
    )
    .fetch_one(pool)
    .await
}

/// Idempotente de propósito: o handler de logout responde 204 tenha ou não
/// afetado linha, pra não virar oráculo de "esse token existe".
pub async fn revoke_refresh_token_by_hash(
    pool: &PgPool,
    token_hash: &[u8],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
        token_hash,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn revoke_all_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn delete_expired_refresh_tokens(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query!("DELETE FROM refresh_tokens WHERE expires_at < now() - interval '30 days'",)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

#[derive(Debug)]
pub enum RotationOutcome {
    Rotated {
        user_id: Uuid,
    },
    /// Revogado há pouco: duas abas renovando juntas, não roubo. Emite token
    /// novo sem derrubar a cadeia. Não dá pra devolver o mesmo substituto
    /// porque só o hash dele existe no banco.
    GraceReplay {
        user_id: Uuid,
    },
    Reused {
        user_id: Uuid,
    },
    Invalid,
}

/// `FOR UPDATE` faz dois refreshes concorrentes serializarem em vez de ambos
/// passarem.
pub async fn rotate_refresh_token(
    pool: &PgPool,
    old_hash: &[u8],
    new_hash: &[u8],
    expires_at: DateTime<Utc>,
    grace: std::time::Duration,
) -> Result<RotationOutcome, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let Some(current) = sqlx::query!(
        r#"
        SELECT id, user_id, expires_at, revoked_at, replaced_by
        FROM refresh_tokens
        WHERE token_hash = $1
        FOR UPDATE
        "#,
        old_hash,
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(RotationOutcome::Invalid);
    };

    if current.expires_at <= Utc::now() {
        return Ok(RotationOutcome::Invalid);
    }

    let grace_cutoff = Utc::now() - chrono::Duration::from_std(grace).unwrap_or_default();
    if let Some(revoked_at) = current.revoked_at {
        // `replaced_by` ausente significa que esta linha morreu por
        // `revoke_all_refresh_tokens` (reuso detectado, takeover do Google)
        // ou por logout — não por rotação normal de um sucessor só. Tratar
        // como `Invalid`, sem cascata: sem essa distinção, um token que a
        // gente acabou de marcar como roubado ainda emitia sessão nova se
        // reapresentado dentro dos 10s seguintes (a graça foi pensada só pra
        // multi-aba, não pra perdoar reuso recém-detectado), e cascatear
        // `Reused` aqui de novo derrubaria sessões legítimas de outro
        // dispositivo só por causa de um cookie obsoleto de logout.
        if current.replaced_by.is_none() {
            return Ok(RotationOutcome::Invalid);
        }

        if revoked_at < grace_cutoff {
            // Passou da graça com sucessor definido: sinal genuíno de roubo —
            // alguém apresentou um elo do meio da cadeia depois que ela já
            // seguiu adiante.
            return Ok(RotationOutcome::Reused {
                user_id: current.user_id,
            });
        }

        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            current.user_id,
            new_hash,
            expires_at,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(RotationOutcome::GraceReplay {
            user_id: current.user_id,
        });
    }

    let new_id = sqlx::query_scalar!(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        current.user_id,
        new_hash,
        expires_at,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $2 WHERE id = $1",
        current.id,
        new_id,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(RotationOutcome::Rotated {
        user_id: current.user_id,
    })
}
