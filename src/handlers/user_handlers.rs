use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use serde_json::json;

use crate::models::{app_state_model::AppState, user_model::User};

#[delete("/user/{id}")]
pub async fn delete_user(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
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
pub async fn update_user(body: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
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
pub async fn create_user(body: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
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
pub async fn get_user_by_id(path: web::Path<i32>, data: web::Data<AppState>) -> impl Responder {
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
pub async fn get_all_users(data: web::Data<AppState>) -> impl Responder {
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
