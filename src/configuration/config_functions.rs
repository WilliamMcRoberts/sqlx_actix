use crate::handlers::user_handlers::*;
use crate::models::token_claims::TokenClaims;
use actix_cors::Cors;
use actix_web::error::Error;
use actix_web::HttpMessage;
use actix_web::{dev::ServiceRequest, http::header, web};
use actix_web_httpauth::extractors::{bearer, AuthenticationError};
use actix_web_httpauth::{extractors::bearer::BearerAuth, middleware::HttpAuthentication};
use dotenv::dotenv;
use hmac::{Hmac, Mac};
use jwt::VerifyWithKey;
use sha2::Sha256;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

// Configure handlers
pub fn config(conf: &mut web::ServiceConfig) {
    let bearer_middleware = HttpAuthentication::bearer(validator);
    let scope = web::scope("/api")
        .service(create_user)
        .service(get_user_by_id)
        .service(get_all_users)
        .service(update_user)
        .service(delete_user)
        .service(basic_auth)
        .service(
            web::scope("/protected")
                .wrap(bearer_middleware)
                .service(check),
        );

    conf.service(scope);
}

async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let jwt_secret: String = std::env::var("JWT_SECRET").expect("JWT_SECRET not set");
    let key: Hmac<Sha256> = Hmac::new_from_slice(jwt_secret.as_bytes()).unwrap();
    let token_string = credentials.token();

    let claims: Result<TokenClaims, &str> = token_string
        .verify_with_key(&key)
        .map_err(|_| "Invalid token");

    match claims {
        Ok(value) => {
            req.extensions_mut().insert(value);
            Ok(req)
        }
        Err(_) => {
            let config = req
                .app_data::<bearer::Config>()
                .cloned()
                .unwrap_or_default()
                .scope("");
            Err((AuthenticationError::from(config).into(), req))
        }
    }
}

pub async fn load_pool(connection_max: u32) -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    println!("🔥 Connecting to {}", database_url);

    let pool = MySqlPoolOptions::new()
        .max_connections(connection_max)
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
