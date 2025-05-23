use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RP<T: Serialize> {
    pub code: i32,
    pub msg: String,
    pub count: u64,
    pub data: T,
}

impl<T: Serialize> RP<T> {
    pub fn new(code: i32, msg: String, count: u64, data: T) -> Self {
        Self {
            code,
            msg,
            count,
            data,
        }
    }
    pub fn ok(count: u64, data: T) -> Self {
        Self::new(0, "ok".to_string(), count, data)
    }
}

impl<T> IntoResponse for RP<Vec<T>>
where
    Vec<T>: Serialize,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct R<T: Serialize> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

impl<T: Serialize> R<T> {
    pub fn new(code: i32, msg: String, data: Option<T>) -> Self {
        Self { code, msg, data }
    }
    pub fn ok() -> Self {
        Self::new(0, "ok".to_string(), None)
    }
    pub fn ok_data(data: T) -> Self {
        Self::new(0, "ok".to_string(), Some(data))
    }
    pub fn err() -> Self {
        Self::new(500, "server error".to_string(), None)
    }
    pub fn err_msg(msg: String) -> Self {
        Self::new(500, msg, None)
    }
    pub fn code(code: i32, msg: String) -> Self {
        Self::new(code, msg, None)
    }
}

impl<T: Serialize> IntoResponse for R<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
