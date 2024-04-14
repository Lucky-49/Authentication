use std::env::{current_dir, var};
use config::{Config, File};
use serde::Deserialize;
use sqlx::ConnectOptions;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgSslMode::{Prefer, Require};

/// Глобальные настройки для отображения всех предварительно сконфигурированных переменных
#[derive(Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub debug: bool,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub secret: Secret,
    pub email: EmailSettings,
    pub frontend_url: String,
}

/// Конкретные настройки приложения для предоставления доступа к `порту`,
/// `хосту`, `протоколу` и возможный URL-адрес приложения
/// во время и после разработки
#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub protocol: String,
}

/// Настройки базы данных для всего приложения
#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    username: String,
    password: String,
    port: u16,
    host: String,
    database_name: String,
    require_ssl: bool,
}

/// Настройки Redis для всего приложения
#[derive(Deserialize, Clone, Debug)]
pub struct RedisSettings {
    pub uri: String,
    pub pool_max_open: u64,
    pub pool_max_idle: u64,
    pub pool_timeout_seconds: u64,
    pub pool_expire_seconds: u64,
}

#[derive(Deserialize, Clone)]
pub struct Secret {
    pub secret_key: String,
    pub token_expiration: i64,
    pub hmac_secret: String,
}

#[derive(Deserialize, Clone)]
pub struct EmailSettings {
    pub host: String,
    pub host_user: String,
    pub host_user_password: String,
}

impl DatabaseSettings {
    pub fn connect_to_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl { Require } else { Prefer };
        let mut options = PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password)
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name);
        options.log_statements(tracing::log::LevelFilter::Trace);
        options
    }
}

/// Среда выполнения для приложения.
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `development` or `production`.", //(не является поддерживаемой средой. Используйте либо "разработка", либо `производство`)
                other
            )),
        }
    }
}

/// Многоцелевая функция, которая помогает определить текущую среду приложения
/// выполняется с использованием переменной окружения `APP_ENVIRONMENT`.
///
/// \`\`\`
/// APP_ENVIRONMENT = development | production.
/// \`\`\`
///
/// После обнаружения он загружает соответствующий файл .yaml
/// затем он загружает переменную окружения, которая переопределяет все, что задано в файле .yaml.
/// Чтобы это сработало, переменная окружения ДОЛЖНА быть в верхнем регистре и начинаться с `APP`,
/// разделитель `_`, затем категория настроек,
/// за которым следует разделитель `__` а затем переменная
/// например `APP_APPLICATION__PORT=5001` для порта, который должен быть установлен как `5001`
pub fn get_settings() -> Result<Settings, config::ConfigError> {
    let base_path = current_dir().expect(
        "Failed to determine the current directory (Не удалось определить текущий каталог)",
    );
    let setting_directory = base_path.join("settings");

    // Определяем запущенную среду.
    // По умолчанию используется значение "разработка", если не указано другое.
    let environment: Environment = var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "development".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT (Не удалось проанализировать APP_ENVIRONMENT).");
    let environment_filename = format!("{}.yaml", environment.as_str());

    println!("получаем настройки из файла {}", environment_filename);

    let settings = Config::builder()
        .add_source(File::from(setting_directory.join("base.yaml")))
        .add_source(File::from(setting_directory.join(environment_filename),))
        // Добавить настройки из переменных окружения (с префиксом APP и '__' в качестве разделителя)
        // Например. `APP_APPLICATION__PORT=5001 установит `Settings.application.port`
        .add_source(Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}