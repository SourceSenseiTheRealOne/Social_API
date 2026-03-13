use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use uuid::Uuid;

pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();

    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        path = %request.uri().path(),
    );

    let _guard = span.enter();

    let mut response = next.run(request).await;

    response
        .headers_mut()
        .insert("X-Request-Id", HeaderValue::from_str(&request_id).unwrap());

    response
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestId(pub String);
