use actix_cors::Cors;
use actix_web::body::BoxBody;
use actix_web::http::header::ContentType;
use actix_web::middleware::Logger;
use actix_web::{delete, patch, HttpRequest};
use actix_web::{get, http::header, post, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};

#[delete("/user/{id}")]
async fn delete_user(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let result = sqlx::query!(
        r#"
        DELETE FROM users
        WHERE id = ?
        "#,
        id
    )
    .execute(&data.db)
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() == 0 {
                let message = format!("User with ID: {} not found", id);
                HttpResponse::NotFound().json(json!({"status": "fail","message": message}))
            } else {
                let message = format!("User with ID: {} has been deleted", id);
                HttpResponse::Ok().body(message)
            }
        }
        Err(e) => {
            let message = format!("Internal server error: {}", e);
            HttpResponse::InternalServerError().json(json!({"status": "error","message": message}))
        }
    }
}

#[patch("/user")]
async fn update_user(body: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
    let update_result = sqlx::query(
        r#"
        UPDATE users 
        SET first_name = ?, last_name = ?, email = ?, age = ?
        WHERE id = ?"#,
    )
    .bind(&body.first_name)
    .bind(&body.last_name)
    .bind(&body.email)
    .bind(&body.age)
    .bind(&body.id)
    .execute(&data.db)
    .await;

    match update_result {
        Ok(result) => {
            if result.rows_affected() == 0 {
                let message = format!("User with ID: {} not found", &body.id.unwrap());
                return HttpResponse::NotFound().json(json!({"status": "fail","message": message}));
            }
        }
        Err(e) => {
            let message = format!("Internal server error: {}", e);
            return HttpResponse::InternalServerError()
                .json(json!({"status": "error","message": message}));
        }
    }

    let updated_user_result = sqlx::query!(
        r#"
        SELECT *
        FROM users
        WHERE id = ?
        "#,
        &body.id
    )
    .fetch_one(&data.db)
    .await;

    match updated_user_result {
        Ok(user) => HttpResponse::Ok().json(User {
            id: Some(user.id),
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            age: user.age,
        }),
        Err(_) => HttpResponse::NotFound().body("User not found"),
    }
}

#[post("/user")]
async fn create_user(body: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
    let result = sqlx::query!(
        r#"
        INSERT INTO users (first_name,last_name,email,age) 
        VALUES (?, ?, ?, ?)
        "#,
        &body.first_name,
        &body.last_name,
        &body.email,
        &body.age,
    )
    .execute(&data.db)
    .await;

    if result.is_err() {
        println!("🔥 Failed to execute query: {:?}", result.err());
        return HttpResponse::InternalServerError().body("There was a problem creating the user.");
    }

    HttpResponse::Ok().json(User {
        id: Some(result.unwrap().last_insert_id() as i32),
        first_name: body.first_name.clone(),
        last_name: body.last_name.clone(),
        email: body.email.clone(),
        age: body.age,
    })
}

#[get("/user/{id}")]
async fn get_user_by_id(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let user_result = sqlx::query!(
        r#"
        SELECT *
        FROM users
        WHERE id = ?
        "#,
        id
    )
    .fetch_one(&data.db)
    .await;

    match user_result {
        Ok(user) => HttpResponse::Ok().json(User {
            id: Some(user.id),
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            age: user.age,
        }),
        Err(_) => HttpResponse::NotFound().body("User not found"),
    }
}

#[get("/users")]
async fn get_all_users(data: web::Data<AppState>) -> impl Responder {
    let users_result = sqlx::query_as!(
        User,
        r#"
        SELECT *
        FROM users
        "#
    )
    .fetch_all(&data.db)
    .await;

    match users_result {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(_) => {
            HttpResponse::InternalServerError().body("There was a problem fetching all users")
        }
    }
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
        .service(create_user)
        .service(get_user_by_id)
        .service(get_all_users)
        .service(update_user)
        .service(delete_user);
    conf.service(scope);
}

struct AppState {
    db: Pool<MySql>,
}

#[derive(Deserialize, FromRow, Serialize, Debug)]
struct User {
    id: Option<i32>,
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
