use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use futures_util::future::LocalBoxFuture;

use crate::{
    app_state::AppState, error::ApiError, models::AuthenticatedUser, services::auth_service,
};

impl FromRequest for AuthenticatedUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let Some(state) = request.app_data::<web::Data<AppState>>().cloned() else {
            return Box::pin(async { Err(ApiError::Internal) });
        };
        let token = match bearer_token(request) {
            Ok(token) => token,
            Err(error) => return Box::pin(async move { Err(error) }),
        };

        Box::pin(async move { auth_service::authenticate(&state.db, &token).await })
    }
}

fn bearer_token(request: &HttpRequest) -> Result<String, ApiError> {
    let value = request
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("authentication required".to_owned()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid authorization header".to_owned()))?;
    let mut parts = value.split_whitespace();
    let scheme = parts.next().unwrap_or_default();
    let token = parts.next().unwrap_or_default();
    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() || parts.next().is_some() {
        return Err(ApiError::Unauthorized(
            "invalid authorization header".to_owned(),
        ));
    }
    Ok(token.to_owned())
}
