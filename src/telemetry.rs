use tracing_subscriber::{EnvFilter, fmt, Registry};
use tracing_subscriber::layer::SubscriberExt;//

pub fn get_subscriber(debug: bool) -> impl tracing::Subscriber + Send + Sync {//
    let env_filter = if debug {//
        "trace".to_string()//
    } else {//
        "info".to_string()//
    };//
    let env_filter = EnvFilter::try_from_default_env()//
        .unwrap_or_else(|_| EnvFilter::new(env_filter));//

    let stdout_log = fmt::layer().pretty();//
    let subscriber = Registry::default()//
        .with(env_filter)//
        .with(stdout_log);//

    let json_log = if !debug {//
        let json_log = fmt::layer().json();//
        Some(json_log)//
    } else {//
        None//
    };//

    let subscriber = subscriber.with(json_log);//

    subscriber//
}//

pub fn init_subscriber(subscriber: impl tracing::Subscriber + Send + Sync) {//
    tracing::subscriber::set_global_default(subscriber)//
        .expect("Filed to set subscriber (Не удалось установить подписчика)")//
}//