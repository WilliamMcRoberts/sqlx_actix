mod configuration;
mod handlers;
mod models;
use crate::configuration::config_functions::{config, load_pool, set_up_cors, set_up_logging};
use crate::models::app_state_model::AppState;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // logging and database configuration
    set_up_logging();
    let pool = load_pool().await;

    HttpServer::new(move || {
        // cors configuration
        let cors = set_up_cors();

        // set up the server
        App::new()
            .app_data(web::Data::new(AppState { db: pool.clone() }))
            .configure(config)
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
