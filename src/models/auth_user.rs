use serde::Serialize;
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct AuthUser {
    pub id: i32,
    pub email: String,
    pub password: String,
}
