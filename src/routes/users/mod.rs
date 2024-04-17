use crate::routes::users::confirm_registration::confirm;
use crate::routes::users::register::register_user;
use actix_web::web::{scope, ServiceConfig};
use crate::routes::users::login::login_user;
use crate::routes::users::logout::log_out;

mod confirm_registration;
mod login;
mod register;
mod logout;

pub fn auth_routes_config(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/users")
            .service(register_user)
            .service(confirm)
            .service(login_user)
            .service(log_out),
    );
}
