use actix_cors::Cors;
use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::middleware::Logger;
use actix_web::HttpRequest;
use actix_web::{get, http::header, post, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/user")]
async fn create_user(body: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
    let result =
        sqlx::query("INSERT INTO users (first_name,last_name,email,age) VALUES (?, ?, ?, ?)")
            .bind(&body.first_name)
            .bind(&body.last_name)
            .bind(&body.email)
            .bind(&body.age)
            .execute(&data.db)
            .await
            .unwrap()
            .last_insert_id();

    match result {
        0 => {
            return HttpResponse::BadRequest().body("There was a problem creating the user.");
        }
        id => {
            return HttpResponse::Ok().json(User {
                id: Some(id),
                first_name: body.first_name.clone(),
                last_name: body.last_name.clone(),
                email: body.email.clone(),
                age: body.age,
            });
        }
    }
}

#[get("/hello")]
async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }

    dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    println!("🔥 Connecting to {}", database_url);

    let pool = match MySqlPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    println!("🚀 Server started successfully");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:8080")
            .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            .supports_credentials();
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

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(hello)
        .service(create_user)
        .service(manual_hello);
    conf.service(scope);
}

struct AppState {
    db: Pool<MySql>,
}

#[derive(Deserialize, FromRow, Serialize, Debug)]
struct User {
    id: Option<u64>,
    first_name: String,
    last_name: String,
    email: String,
    age: i32,
}

impl Responder for User {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body)
    }
}
