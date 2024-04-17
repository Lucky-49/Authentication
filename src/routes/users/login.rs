use crate::types::{ErrorResponse, User, UserVisible, USER_EMAIL_KEY, USER_ID_KEY};
use crate::utils::verify_password;
use actix_session::Session;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{query, Error, PgPool, Row};
use tokio::task::spawn_blocking;
use tracing::instrument;

#[derive(Serialize, Deserialize)]
pub struct LoginUser {
    email: String,
    password: String,
}

#[instrument(name = "Logging a user in", skip( pool, user, session), fields(user_email = %user.email))]
#[post("/login/")]
async fn login_user(pool: Data<PgPool>, user: Json<LoginUser>, session: Session) -> HttpResponse {
    match get_user_who_is_active(&pool, &user.email).await {
        Ok(loggedin_user) => match spawn_blocking(move || {
            verify_password(loggedin_user.password.as_ref(), user.password.as_bytes())
        })
        .await
        .expect("Unable to unwrap JoinError.")
        {
            Ok(_) => {
                tracing::event!(target: "backend", tracing::Level::INFO, "User logged in successfully");
                session.renew();
                session
                    .insert(USER_ID_KEY, loggedin_user.id)
                    .expect("'user_id' cannot be inserted into session");
                session
                    .insert(USER_EMAIL_KEY, &loggedin_user.email)
                    .expect("'user_email' cannot be inserted into session");

                HttpResponse::Ok().json(UserVisible {
                    id: loggedin_user.id,
                    email: loggedin_user.email,
                    first_name: loggedin_user.first_name,
                    last_name: loggedin_user.last_name,
                    is_active: loggedin_user.is_active,
                    is_staff: loggedin_user.is_staff,
                    is_superuser: loggedin_user.is_superuser,
                    thumbnail: loggedin_user.thumbnail,
                    date_joined: loggedin_user.date_joined,
                })
            }
            Err(e) => {
                tracing::event!(target: "argon2", tracing::Level::ERROR, "Failed to authenticate user: \
                {:#?}", e);
                HttpResponse::BadRequest().json(ErrorResponse {
                    error: "Email and password do not match.".to_string(),
                })
            }
        },
        Err(e) => {
            tracing::event!(target: "sqlx", tracing::Level::ERROR, "User not found: {:#?}", e);
            HttpResponse::NotFound().json(ErrorResponse {
                error: "A user with there details does not exist. If you registered with these details, \
                ensure you activate your account by clicking on the link sent to your e-mail address"
                    .to_string(),
            })
        }
    }
}

#[instrument(name = "Getting a user from DB.", skip(pool, email), fields(user_email = %email))]
pub async fn get_user_who_is_active(pool: &PgPool, email: &String) -> Result<User, Error> {
    match query(
        "SELECT id, email, password, first_name, last_name, is_staff, is_superuser, \
    thumbnail, date_joined FROM users WHERE email = $1 AND is_active = TRUE",
    )
    .bind(email)
    .map(|row: PgRow| User {
        id: row.get("id"),
        email: row.get("email"),
        password: row.get("password"),
        first_name: row.get("first_name"),
        last_name: row.get("last_name"),
        is_active: true,
        is_staff: row.get("is_staff"),
        is_superuser: row.get("is_superuser"),
        thumbnail: row.get("thumbnail"),
        date_joined: row.get("date_joined"),
    })
    .fetch_one(pool)
    .await
    {
        Ok(user) => Ok(user),
        Err(e) => {
            tracing::event!(target: "sqlx", tracing::Level::ERROR, "User not found in DB: {:#?}", e);
        }
    }
}