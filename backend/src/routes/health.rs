#[tracing::instrument]
#[actix_web::get("/health_check/")]
pub async fn health_check() -> actix_web::HttpResponse {
    tracing::event!(target: "Authentication", tracing::Level::DEBUG,
        "Accessing health-check endpoint (Доступ к конечной точке проверки работоспособности).");
    actix_web::HttpResponse::Ok().json("Application is safe and healthy.")
}
