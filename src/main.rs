// оригинал статьи https://dev.to/sirneij/full-stack-authentication-system-using-rust-actix-web-and-sveltekit-1cc6

use Authentication::settings::get_settings;
use Authentication::startup;
use Authentication::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let settings = get_settings().expect("Failed to read settings (Не удалось прочитать настройки).");

    let subscriber = get_subscriber(settings.clone().debug);
    init_subscriber(subscriber);

    let application = startup::Application::build(settings).await?;

    tracing::event!(target: "Application", tracing::Level::INFO, "Listening on http://127.0.0.1:{}/", application.port());

    application.run_until_stopped().await?;
    Ok(())
}
