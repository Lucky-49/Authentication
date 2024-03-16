use std::io::Error;
use std::net::TcpListener;
use sqlx::{PgPool};
use crate::settings::{DatabaseSettings, Settings};

pub struct Application {
    port: u16,
    server: actix_web::dev::Server,
}

impl Application {
    pub async fn build(settings: Settings,
    test_pool: Option<PgPool>,
    ) -> Result<Self, Error> {
        let connection_pool = if let Some(pool) = test_pool {
            pool
        } else {
            get_connection_pool(settings.clone().database).await
        };

        sqlx::migrate!("./backend")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database (Не удалось перенести базу данных).");

        let address = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let listener = std::net::TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, settings).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn get_connection_pool(settings: DatabaseSettings) -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(settings.connect_to_db())
}

async fn run(listener: TcpListener,
db_pool: PgPool,
settings: Settings,
) -> Result<actix_web::dev::Server, std::io::Error> {
    // Состояние приложения пула подключений к базе данных
    let pool = actix_web::web::Data::new(db_pool);

    // Пул подключений Redis
    let cfg = deadpool_redis::Config::from_url(settings.clone().redis.uri);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Cannot create deadpool redis (Не удается создать deadpool redis.).");
    let redis_pool_data = actix_web::web::Data::new(redis_pool);

    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new().service(crate::routes::health_check)
        // Добавляем пул баз данных и пул Redis в состояние приложения
            .app_data(pool.clone())
            .app_data(redis_pool_data.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
