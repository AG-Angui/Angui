use std::{
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    Error, FromRequest, HttpMessage, HttpRequest, ResponseError,
    body::{EitherBody, MessageBody},
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    http::header,
    web,
};
use futures_util::future::{LocalBoxFuture, Ready, ready};

use crate::{
    app_state::AppState, error::ApiError, models::AuthenticatedUser, services::auth_service,
};

/// 在 JSON 等 payload extractor 之前验证 API 会话，确保所有受保护端点对缺失或失效 token
/// 均返回统一的 401，而不会因请求 body 不完整先泄露 400。
#[derive(Clone, Copy)]
pub struct ApiSessionAuthentication;

pub struct ApiSessionAuthenticationMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for ApiSessionAuthentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiSessionAuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiSessionAuthenticationMiddleware {
            service: Rc::new(service),
        }))
    }
}

impl<S, B> Service<ServiceRequest> for ApiSessionAuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(context)
    }

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        Box::pin(async move {
            if public_api_path(request.path()) {
                return service
                    .call(request)
                    .await
                    .map(ServiceResponse::map_into_left_body);
            }

            let Some(state) = request.app_data::<web::Data<AppState>>().cloned() else {
                return Ok(request
                    .into_response(ApiError::Internal.error_response().map_into_right_body()));
            };
            let token = match bearer_token_from_service_request(&request) {
                Ok(token) => token,
                Err(error) => {
                    return Ok(request.into_response(error.error_response().map_into_right_body()));
                }
            };
            let authenticated = match auth_service::authenticate(&state.db, &token).await {
                Ok(authenticated) => authenticated,
                Err(error) => {
                    return Ok(request.into_response(error.error_response().map_into_right_body()));
                }
            };
            request.extensions_mut().insert(authenticated);

            service
                .call(request)
                .await
                .map(ServiceResponse::map_into_left_body)
        })
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = ApiError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        if let Some(authenticated) = request.extensions().get::<AuthenticatedUser>().cloned() {
            return Box::pin(async move { Ok(authenticated) });
        }
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

fn public_api_path(path: &str) -> bool {
    matches!(
        path,
        "/api/health" | "/api/auth/login" | "/api/learning/public/prevention-card"
    )
}

fn bearer_token_from_service_request(request: &ServiceRequest) -> Result<String, ApiError> {
    let value = request
        .headers()
        .get(header::AUTHORIZATION)
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
