use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::headers::ContentType;
// pub struct AppError(anyhow::Error);

pub enum AppError {
    Internal(anyhow::Error),
    BadRequest(String),
    UnsupportedMediaType(String),
    PayloadTooLarge(String),
    Forbidden(String),
    NotFound(String),
    Exists(String),
}

// // Tell axum how to convert `AppError` into a response.
// impl IntoResponse for AppError {
//     fn into_response(self) -> Response {
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             format!("Something went wrong: {}", self.0),
//         )
//             .into_response()
//     }
// }
//
// // This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// // `Result<_, AppError>`. That way you don't need to do that manually.
// impl<E> From<E> for AppError
// where
//     E: Into<anyhow::Error>,
// {
//     fn from(err: E) -> Self {
//         Self(err.into())
//     }
// }

impl AppError {
    fn into_json_response<S>(status_code: StatusCode, s: S) -> Response
    where
        S: Into<String>,
    {
        (
            status_code,
            [(header::CONTENT_TYPE, ContentType::json().to_string())],
            serde_json::json!({"error": s.into()}).to_string(),
        )
            .into_response()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            Self::Internal(e) => {
                tracing::error!("{e}");
                Self::into_json_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }

            Self::BadRequest(s) => Self::into_json_response(StatusCode::BAD_REQUEST, s),

            Self::UnsupportedMediaType(s) => {
                Self::into_json_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, s)
            }
            Self::PayloadTooLarge(s) => Self::into_json_response(StatusCode::PAYLOAD_TOO_LARGE, s),

            Self::Forbidden(s) => Self::into_json_response(StatusCode::FORBIDDEN, s),
            Self::NotFound(s) => Self::into_json_response(StatusCode::NOT_FOUND, s),
            Self::Exists(s) => {Self::into_json_response(StatusCode::CONFLICT, s)}
        }
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(e: E) -> Self {
        Self::Internal(e.into())
    }
}
