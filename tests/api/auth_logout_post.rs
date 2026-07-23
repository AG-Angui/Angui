use actix_web::{
    http::{StatusCode, header},
    test,
};

use crate::support::{FAMILY, TestContext, assert_error};

#[actix_web::test]
async fn logout_revokes_the_current_bearer_session() {
    let context = TestContext::new().await;
    let token = context.token(FAMILY).await;
    let app = crate::init_api_app!(&context);
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/logout")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let revoked = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/auth/me")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_error(revoked, StatusCode::UNAUTHORIZED, "unauthorized").await;
}
