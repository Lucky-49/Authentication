use actix_web::web::{scope, ServiceConfig};
use crate::routes::users::confirm_registration::confirm;
use crate::routes::users::register::register_user;

mod register;
mod confirm_registration;
mod login;

pub fn auth_routes_config(cfg: &mut ServiceConfig) {
    cfg
        .service(scope("/users")
            .service(register_user)
            .service(confirm)
            .service(login_user)
            .service(log_out),
        );
}