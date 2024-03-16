/// Глобальные настройки для отображения всех предварительно сконфигурированных переменных
#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSetting,
    pub debug: bool,
}

/// Конкретные настройки приложения для предоставления доступа к `порту`,
/// `хосту`, `протоколу` и возможный URL-адрес приложения
/// во время и после разработки
#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSetting {
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub protocol: String,
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
                "{} is not a supported environment. Use either `development` or `production` .", //(не является поддерживаемой средой. Используйте либо "разработка", либо `производство`)
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
/// за которым следует разделитель `__`, а затем переменная
/// например `APP_APPLICATION__PORT=5001` для порта, который должен быть установлен как `5001`
pub fn get_settings() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory (Не удалось определить текущий каталог)");
    let setting_directory = base_path.join("settings");

    // Определяем запущенную среду.
    // По умолчанию используется значение "разработка", если не указано другое.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "development".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT (Не удалось проанализировать APP_ENVIRONMENT).");
    let environment_filename = format!("{}.yaml", environment.as_str());
    let settings = config::Config::builder()
        .add_source(config::File::from(setting_directory.join("base.yaml")))
        .add_source(config::File::from(
            setting_directory.join(environment_filename),
        ))
        // Добавить настройки из переменных окружения (с префиксом APP и '__' в качестве разделителя)
        // Например. `APP_APPLICATION__PORT=5001 установит `Settings.application.port`
        .add_source(config::Environment::with_prefix("APP")
            .prefix_separator("_")
            .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}