use crate::models::{
    app_state_model::AppState, auth_user::AuthUser, token_claims::TokenClaims, user_model::User,
    user_no_password::UserNoPassword,
};
use actix_web::{
    delete, get, patch, post,
    web::{Data, Json, Path, ReqData},
    HttpResponse, Responder,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use argonautica::{Hasher, Verifier};
use chrono::NaiveDateTime;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use sqlx::{self, FromRow};

#[get("/check")]
async fn check(req_user: Option<ReqData<TokenClaims>>) -> impl Responder {
    match req_user {
        Some(user) => HttpResponse::Ok().json(user.id),
        None => HttpResponse::Unauthorized().json("Invalid token"),
    }
}

#[get("/auth")]
async fn basic_auth(state: Data<AppState>, credentials: BasicAuth) -> impl Responder {
    let jwt_secret: Hmac<Sha256> = Hmac::new_from_slice(
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set")
            .as_bytes(),
    )
    .unwrap();

    let email = credentials.user_id().to_string();
    let password = credentials.password();

    match password {
        None => HttpResponse::Unauthorized().body("Must provide a password"),
        Some(pass) => {
            match sqlx::query_as::<_, AuthUser>(
                r#"
                SELECT id, email, password
                FROM users
                WHERE email = ?
                "#,
            )
            .bind(&email.to_string())
            .fetch_one(&state.db)
            .await
            {
                Ok(user) => {
                    let hash_secret =
                        std::env::var("HASH_SECRET").expect("HASH_SECRET must be set");
                    let mut verifier = Verifier::default();
                    let is_valid = verifier
                        .with_hash(user.password)
                        .with_password(pass)
                        .with_secret_key(hash_secret)
                        .verify()
                        .unwrap();

                    if is_valid {
                        let claims = TokenClaims { id: user.id };

                        let token_str = claims.sign_with_key(&jwt_secret).unwrap();

                        HttpResponse::Ok().json(token_str)
                    } else {
                        HttpResponse::Unauthorized().json("Invalid password")
                    }
                }
                Err(error) => HttpResponse::InternalServerError().json(format!("Error: {}", error)),
            }
        }
    }
}

#[delete("/user/{id}")]
async fn delete_user(path: Path<i32>, data: Data<AppState>) -> impl Responder {
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
async fn update_user(body: Json<User>, data: Data<AppState>) -> impl Responder {
    let update_result = sqlx::query(
        r#"
        UPDATE users 
        SET first_name = ?, last_name = ?, email = ?, age = ?, password = ?
        WHERE id = ?"#,
    )
    .bind(&body.first_name)
    .bind(&body.last_name)
    .bind(&body.email)
    .bind(&body.age)
    .bind(&body.password)
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
        Ok(user) => HttpResponse::Ok().json(UserNoPassword {
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
async fn create_user(body: Json<User>, data: Data<AppState>) -> impl Responder {
    let hash_secret = std::env::var("HASH_SECRET").expect("HASH_SECRET must be set");

    let mut hasher = Hasher::default();

    let hash = hasher
        .with_password(&body.password)
        .with_secret_key(hash_secret)
        .hash();

    if let Err(e) = hash {
        println!("🔥 Failed to hash password: {:?}", e);
        return HttpResponse::InternalServerError().body("There was a problem creating the user.");
    }

    let hash = hash.unwrap();

    let result = sqlx::query!(
        r#"
        INSERT INTO users (first_name,last_name,email,age,password) 
        VALUES (?, ?, ?, ?, ?)
        "#,
        &body.first_name,
        &body.last_name,
        &body.email,
        &body.age,
        &hash,
    )
    .execute(&data.db)
    .await;

    if result.is_err() {
        return HttpResponse::InternalServerError().body(result.err().unwrap().to_string());
    }

    HttpResponse::Ok().json(UserNoPassword {
        id: Some(result.unwrap().last_insert_id() as i32),
        first_name: body.first_name.clone(),
        last_name: body.last_name.clone(),
        email: body.email.clone(),
        age: body.age,
    })
}

#[get("/user/{email}")]
async fn get_user_by_email(path: Path<String>, data: Data<AppState>) -> impl Responder {
    let email = path.into_inner();
    let user_result = sqlx::query!(
        r#"
        SELECT *
        FROM users
        WHERE email = ?
        "#,
        email
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
            password: user.password,
        }),
        Err(_) => HttpResponse::NotFound().body("User not found"),
    }
}
#[get("/user/{id}")]
async fn get_user_by_id(path: Path<i32>, data: Data<AppState>) -> impl Responder {
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
        Ok(user) => HttpResponse::Ok().json(UserNoPassword {
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
async fn get_all_users(data: Data<AppState>) -> impl Responder {
    let users_result = sqlx::query_as!(
        UserNoPassword,
        r#"
        SELECT id, first_name, last_name, email, age
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
