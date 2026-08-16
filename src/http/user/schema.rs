use serde::{Deserialize, Deserializer};

use crate::domain::ThemePreference;
use crate::http::error::ApiError;

/// Campo ausente fica inalterado. `Option<Option<T>>` nos campos que aceitam
/// null distingue "não mandou" de "mandou null pra limpar".
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub full_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub professional_title: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub avatar_url: Option<Option<String>>,
    pub theme_preference: Option<ThemePreference>,
}

impl UpdateProfileRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        blank_check(&self.full_name, "Informe um nome válido.")?;
        blank_check(&self.professional_title, "Informe um título válido.")?;

        if let Some(Some(url)) = &self.avatar_url {
            if !url.starts_with("https://") {
                return Err(ApiError::Unprocessable(
                    "O endereço do avatar deve começar com https://.".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub const MIN_PASSWORD_LENGTH: usize = 8;

impl UpdatePasswordRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.new_password.chars().count() < MIN_PASSWORD_LENGTH {
            return Err(ApiError::Unprocessable(format!(
                "A nova senha deve ter pelo menos {MIN_PASSWORD_LENGTH} caracteres."
            )));
        }
        if self.new_password == self.current_password {
            return Err(ApiError::Unprocessable(
                "A nova senha deve ser diferente da atual.".to_string(),
            ));
        }
        Ok(())
    }
}

fn blank_check(field: &Option<Option<String>>, message: &str) -> Result<(), ApiError> {
    match field {
        Some(Some(value)) if value.trim().is_empty() => {
            Err(ApiError::Unprocessable(message.to_string()))
        }
        _ => Ok(()),
    }
}

/// Sem isso o serde colapsaria `null` explícito em `None`, indistinguível de
/// campo ausente — e o PATCH perderia o "limpe este campo".
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(Some)
}
