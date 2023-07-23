use actix_web::{body::BoxBody, http::header::ContentType, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Deserialize, FromRow, Serialize, Debug)]
pub struct User {
    pub id: Option<i32>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub age: i32,
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
