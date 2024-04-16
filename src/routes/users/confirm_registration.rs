use actix_web::{get, HttpResponse};
use actix_web::http::header::LOCATION;
use actix_web::web::{Data, Query};
use deadpool_redis::Pool;
use serde::Deserialize;
use sqlx::{Error, PgPool};
use tracing::instrument;
use uuid::Uuid;
use crate::settings::get_settings;
use crate::types::{ErrorResponse, SuccessResponse};
use crate::utils::verify_confirmation_token_pasetor;

#[derive(Deserialize)]
pub struct Parameters {
    token: String,
}

#[instrument(name = "Activating a new user",
skip(pool, parameters, redis_pool))]
#[get("/register/confirm/")]
pub async fn confirm (
    parameters: Query<Parameters>,
    pool: Data<PgPool>,
    redis_pool: Data<Pool>,
) -> HttpResponse {
    let settings = get_settings()
        .expect("Failed to read settings.");

    let mut redis_con = redis_pool.get()
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{}", e);

            HttpResponse::SeeOther()
                .insert_header((LOCATION,
                format!("{}/auth/error", settings.frontend_url),
                ))
                .json(ErrorResponse {
                    error: "We cannot activate your account at the moment".to_string(),
                })
        })
        .expect("Redis connection cannot be gotten.");

    let confirmation_token = match verify_confirmation_token_pasetor(
        parameters.token.clone(),
        &mut redis_con,
        None,
    )
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{:#?}", e);

            return HttpResponse::SeeOther().insert_header((LOCATION,
            format!("{}/auth/regenerate-token", settings.frontend_url),
            ))
                .json(ErrorResponse {
                    error: "It appears that your confirmation token has expired or previously used. \
                    Kindiy generate a new token".to_string(),
                });
        }
    };

    match activate_new_user(&pool, confirmation_token.user_id)
        .await {
        Ok(_) => {
            tracing::event!(target: "backend", tracing::Level::INFO, "New user was activated successfully");

            HttpResponse::SeeOther()
                .insert_header((LOCATION,
                format!("{}/auth/confirmed", settings.frontend_url),
                ))
                .json(SuccessResponse {
                    message: "Your account has been activated successfully! You can log in".to_string(),
                })
        }

        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "Cannot activate account : {}", e);

            HttpResponse::SeeOther()
                .insert_header((LOCATION,
                format!("{}/auth/error?reason={e}", settings.frontend_url),
                ))
                .json(ErrorResponse {
                    error: "We cannot activate your account at the moment".to_string(),
                })
        }
    }
}


#[instrument(name = "Mark a user active", skip(pool),
fields(new_user_user_id = %user_id))]
pub async fn activate_new_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), Error> {
    match sqlx::query("UPDATE users SET is_active=true WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Failed to execute query: {:#?}", e);
            Err(e)
        }
    }
}