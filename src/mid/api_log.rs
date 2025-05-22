use crate::common::res::R;
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use tokio::time::Instant;
use tracing::info;

pub async fn log(req: Request, next: Next) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let uri = parts.uri.to_string();
    let method = parts.method.to_string();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    let req_bytes = &bytes.clone();
    let req_val = std::str::from_utf8(req_bytes).unwrap();
    info!("{} {} req:{}", method, uri, req_val);
    let req = Request::from_parts(parts, Body::from(bytes));
    let start = Instant::now();
    let res = next.run(req).await;
    let status = res.status().clone();
    let duration = start.elapsed();
    let (parts, body) = res.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    let res_body = std::str::from_utf8(&bytes).unwrap().to_string();
    let mut res = Response::from_parts(parts, Body::from(bytes));
    // 统一错误返回
    if StatusCode::OK != status {
        res = R::<Value>::err_msg(res_body.clone()).into_response();
    }
    info!(
        "{} {} {:?} req:{} res:{}",
        method, uri, duration, req_val, res_body
    );
    Ok(res)
}
