use crate::handlers::user_handlers::{
    create_user, delete_user, get_all_users, get_user_by_id, update_user,
};
use actix_cors::Cors;
use actix_web::{http::header, web};
use dotenv::dotenv;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(create_user)
        .service(get_user_by_id)
        .service(get_all_users)
        .service(update_user)
        .service(delete_user);
    conf.service(scope);
}

pub async fn load_pool() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    println!("🔥 Connecting to {}", database_url);

    let pool = MySqlPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await;

    match pool {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    }
}

pub fn set_up_logging() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }

    dotenv().ok();
    env_logger::init();
}

pub fn set_up_cors() -> Cors {
    Cors::default()
        .allowed_origin("http://localhost:8080")
        .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE"])
        .allowed_headers(vec![
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ])
        .supports_credentials()
}
