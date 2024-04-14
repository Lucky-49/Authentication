use argon2::password_hash::rand_core::{OsRng, RngCore};
use chrono::{Duration, Local};
use deadpool_redis::redis::AsyncCommands;
use pasetors::claims::{Claims, ClaimsValidationRules};
use pasetors::keys::SymmetricKey;
use pasetors::local;
use pasetors::token::UntrustedToken;
use pasetors::version4::V4;
use serde_json::json;
use crate::settings::get_settings;

/// Сохраняем префикс сеансового ключа как const, чтобы в нем не было опечаток везде, где он используется.
const SESSION_KEY_PREFIX: &str = "valid_session_key_for_{}";

/// Выдает пользователю токен pasetor. В токене закодирован идентификатор пользователя и ключ сеанса.
/// Этот ключ используется для уничтожения токена как только он будет подтвержден.
/// В зависимости от его использования, у выданного токена срок жизни не более часа.
/// Что означает, что он уничтожается по истечении срока его службы.

#[tracing::instrument(name = "Issue pasetors token", skip(redis_connection))]
pub async fn issue_confirmation_token_pasetors(
    user_id: uuid::Uuid,
    redis_connection: &mut deadpool_redis::redis::aio::Connection,
    is_for_password_change: Option<bool>,
) -> Result<String, deadpool_redis::redis::RedisError> {
    // Генерируем 128 байт случайных данных для сеансового ключа
    let session_key: String = {
        let mut buff = [0_u8; 128];
        OsRng.fill_bytes(&mut buff);
        hex::encode(buff)
    };

    let redis_key = {
        if is_for_password_change.is_some() {
            format!(
                "{}{} is_for_password_change",
                SESSION_KEY_PREFIX, session_key
            )
        } else {
            format!("{}{}", SESSION_KEY_PREFIX, session_key)
        }
    };

    redis_connection
        .set(redis_key.clone(), // Подтверждаем, что ключ существует, чтобы указать, что сеанс "живой".
             String::new(),
        )
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "RedisError (set): {}", e);
            e
        })?;

    let settings = get_settings().expect("Cannot load settings.");
    let current_date_time = Local::now();
    let dt = {
        if is_for_password_change.is_some() {
            current_date_time + Duration::try_hours(1).map_or(Duration::zero(), |duration| duration)
        } else {
            current_date_time + Duration::try_minutes(settings.secret.token_expiration).map_or(Duration::zero(), |duration| duration)
        }
    };

    let time_to_live = {
        if is_for_password_change.is_some() {
            Duration::try_hours(1).map_or(Duration::zero(), |duration| duration)
        } else {
            Duration::try_minutes(settings.secret.token_expiration).map_or(Duration::zero(), |duration| duration)//
        }
    };

    redis_connection
        .expire(
            redis_key.clone(),
            time_to_live.num_seconds().try_into().unwrap()
        )
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "RedisError (expiry): {}", e);
            e
        })?;

    let mut claims = Claims::new().unwrap();
    claims.expiration(&dt.to_rfc3339()).unwrap();
    claims
        .add_additional("user_id", json!(user_id))
        .unwrap();
    claims
        .add_additional("session_key", json!(session_key))
        .unwrap();

    let sk = SymmetricKey::<V4>::from(settings.secret.secret_key.as_bytes()).unwrap();
    Ok(local::encrypt(
        &sk,
        &claims,
        None,
        Some(settings.secret.hmac_secret.as_bytes()),
    )
        .unwrap())
}

/// Проверяет и уничтожает токен. Токен уничтожается немедленно
/// он успешно проверен, и все закодированные данные извлечены.
/// Redis используется для такого уничтожения.

#[tracing::instrument(name = "Verify pasetors token", skip(token, redis_connection))]
pub async fn verify_confirmation_token_pasetor(
    token: String,
    redis_connection: &mut deadpool_redis::redis::aio::Connection,
    is_password: Option<bool>,
) -> Result<crate::types::ConfirmationToken, String> {
    let settings = get_settings().expect("Cannot load settings.");
    let sk = SymmetricKey::<V4>::from(settings.secret.secret_key.as_bytes()).unwrap();

    let validation_rules = ClaimsValidationRules::new();
    let untrusted_token = UntrustedToken::<pasetors::token::Local, V4>::try_from(&token) //проверить написание pasetors::token::Local, в исходнике только Local
        .map_err(|e| format!("TokenValidation: {}", e))?;

    let trusted_token = local::decrypt(
        &sk,
        &untrusted_token,
        &validation_rules,
        None,
        Some(settings.secret.hmac_secret.as_bytes()),
    )
        .map_err(|e| format!("Pasetor: {}", e))?;
    let claims = trusted_token.payload_claims().unwrap();

    let uid = serde_json::to_value(claims.get_claim("user_id").unwrap()).unwrap();

    match serde_json::from_value::<String>(uid) {
        Ok(uuid_string) => match uuid::Uuid::parse_str(&uuid_string) {
            Ok(user_uuid) => {
                let sss_key =
                    serde_json::to_value(claims.get_claim("session_key").unwrap()).unwrap();
                let session_key = match serde_json::from_value::<String>(sss_key) {
                    Ok(session_key) => session_key,
                    Err(e) => return Err(format!("{}", e)),
                };

                let redis_key = {
                    if is_password.is_some() {
                        format!(
                            "{}{} is_for_password_change",
                            SESSION_KEY_PREFIX, session_key
                        )
                    } else {
                        format!("{}{}", SESSION_KEY_PREFIX, session_key)
                    }
                };

                if redis_connection
                    .get::<_, Option<String>>(redis_key.clone())
                    .await
                    .map_err(|e| format!("{}", e))?
                    .is_none()
                {
                    return Err("Token has been used or expired.".to_string());
                }
                redis_connection
                    .del(redis_key.clone())
                    .await
                    .map_err(|e| format!("{}", e))?;
                Ok(crate::types::ConfirmationToken { user_id: user_uuid })
            }
            Err(e) => Err(format!("{}", e)),
        },
        Err(e) => Err(format!("{}", e)),
    }
}