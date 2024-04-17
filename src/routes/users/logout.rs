use actix_session::Session;
use actix_web::{HttpResponse, post};
use tracing::instrument;
use uuid::Uuid;
use crate::types::{ErrorResponse, SuccessResponse, USER_ID_KEY};

#[instrument(name = "Log out user.", skip(session))]
#[post("/logout/")]
pub async fn log_out(session: Session) -> HttpResponse {
    match session_user_id(&session).await {
        Ok(_) => {
            tracing::event!(target: "backend", tracing::Level::INFO, "Users retrieved from the DB.");
            session.purge();
            HttpResponse::Ok().json(SuccessResponse {
                message: "You have successfully logged out".to_string(),
            })
        }
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "Failed to get user from session: {:#?}", e);
            HttpResponse::BadRequest().json(ErrorResponse {
                error: "We currently have some issues. Kindly try again and ensure you are logged in."
                    .to_string(),
            })
        }
    }
}

#[instrument(name = "Get user_id from session.", skip(session))]
async fn session_user_id(session: &Session) -> Result<Uuid, String> {
    match session.get(USER_ID_KEY) {
        Ok(user_id) => match user_id {
            None => Err("You are not authenticated". to_string()),
            Some(id) => Ok(id),
        },
        Err(e) => Err(format!("{}", e)),
    }
}