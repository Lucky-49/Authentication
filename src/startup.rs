use std::io::Error;
use std::net::TcpListener;
use std::time::Duration;
use actix_session::SessionMiddleware;
use actix_session::storage::CookieSessionStore;
use actix_web::{App, HttpServer};
use actix_web::cookie::{Key, SameSite};
use actix_web::dev::Server;
use actix_web::web::Data;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use crate::routes::{auth_routes_config, health_check};
use crate::settings::{DatabaseSettings, Settings};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(settings: Settings, test_pool: Option<PgPool>) -> Result<Self, Error> {
        let connection_pool = if let Some(pool) = test_pool {
            pool
        } else {
            get_connection_pool(&settings.database).await
        };

        sqlx::migrate!("../backend/migrations")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database (Не удалось перенести базу данных).");

        let address = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, settings).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), Error> {
        self.server.await
    }
}

pub async fn get_connection_pool(settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy_with(settings.connect_to_db())
}

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    settings: Settings,
) -> Result<Server, Error> {
    // Состояние приложения пула подключений к базе данных
    let pool = Data::new(db_pool);

    // Пул подключений Redis
    let cfg = deadpool_redis::Config::from_url(settings.clone().redis.uri);
    let redis_pool = cfg
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Cannot create deadpool redis (Не удается создать deadpool_redis.).");
    let redis_pool_data = Data::new(redis_pool);

    //Создание сессии
    let secret_key = Key::from(settings.secret.hmac_secret.as_bytes());

    let server = HttpServer::new(move || {
        App::new()
            .wrap(if settings.debug {
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
                    .cookie_http_only(true)
                    .cookie_same_site(SameSite::None)
                    .cookie_secure(true)
                    .build()
            } else {
                SessionMiddleware::new(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
            })
            .service(health_check)
            .configure(auth_routes_config)  //Маршруты аутентификации
            //Добавляем, в состояние приложения, пул баз данных и пул Redis
            .app_data(pool.clone())
            .app_data(redis_pool_data.clone())
    })
        .listen(listener)?
        .run();

    Ok(server)
}