use actix_web::HttpResponse;

#[tracing::instrument]//
#[actix_web::get("/health_check/")]//
pub async fn health_check() -> HttpResponse {//
    tracing::event!(target: "backend", tracing::Level::INFO,
        "Accessing health-check endpoint (Доступ к конечной точке проверки работоспособности).");//
    HttpResponse::Ok().json(crate::types::SuccessResponse {
        message: "Application is safe and healthy.".to_string(),
    })
}