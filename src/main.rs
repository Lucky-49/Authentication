use std::io::Result;
use dotenv::dotenv;
use backend::settings::get_settings;
use backend::startup::Application;
use backend::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]//
async fn main() -> Result<()> {//
    dotenv().ok();//
    // dotenv::from_filename(".env.development").ok();

    let settings =//
        get_settings().expect("Failed to read settings (Не удалось прочитать настройки).");//

    let subscriber = get_subscriber(settings.clone().debug);//
    init_subscriber(subscriber);//

    let application = Application::build(settings, None).await?;//

    tracing::event!(target: "backend", tracing::Level::INFO, "Listening on http://127.0.0.1:{}/",//
        application.port());//

    application.run_until_stopped().await?;//
    Ok(())//
}