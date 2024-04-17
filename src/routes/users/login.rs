use actix_session::Session;
use actix_web::HttpResponse;
use actix_web::web::{Data, Json};
use serde::{Deserialize, Serialize};
use sqlx::{Error, PgPool, query};
use sqlx::postgres::PgRow;
use sqlx::query::Query;
use tokio::task::spawn_blocking;
use tracing::instrument;
use crate::types::{User, USER_EMAIL_KEY, USER_ID_KEY};
use crate::utils::verify_password;

#[derive(Serialize, Deserialize)]
pub struct LoginUser {
    email: String,
    password: String,
}

async fn login_user (pool: Data<PgPool>, user: Json<LoginUser>, session: Session,) -> HttpResponse {
    match get_user_who_is_active(&pool, &user.email).await {
        Ok(loggedin_user) => match spawn_blocking(move || {
            verify_password(loggedin_user.password.as_ref(), user.as_bytes())
        })
            .await
            .expect("Unable to unwrap JoinError.")
        {
            Ok(_) => {
                tracing::event!(target: "backend", tracing::Level::INFO, "User logged in successfully");
                session.renew();
                session.insert(USER_ID_KEY, loggedin_user.id)
                .expect("'user_id' cannot be inserted into session");
                session.insert(USER_EMAIL_KEY, &loggedin_user.email)
                    .expect("'user_email' cannot be inserted into session");

                HttpResponse::Ok().json()
            }
        }
    }
}

#[instrument(name = "Getting a user from DB.", skip(pool, email),fields(user_email = %email))]
pub async fn get_user_who_is_active(
    pool: &PgPool,
    email: &String,
) -> Result<User, Error> {
    match query("SELECT")
        .bind(email)
        .map(|row: PgRow| User {

        })
}