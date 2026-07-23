use actix_web::{http::StatusCode, test};

use crate::support::{TestContext, assert_error};

macro_rules! assert_unauthorized {
    ($app:expr, $method:ident, $uri:expr) => {{
        let missing =
            test::call_service(&$app, test::TestRequest::$method().uri($uri).to_request()).await;
        assert_eq!(
            missing.status(),
            StatusCode::UNAUTHORIZED,
            "missing token should be rejected before request parsing for {}",
            $uri
        );
        assert_error(missing, StatusCode::UNAUTHORIZED, "unauthorized").await;
        let invalid = test::call_service(
            &$app,
            test::TestRequest::$method()
                .uri($uri)
                .insert_header(("authorization", "Bearer invalid-token"))
                .to_request(),
        )
        .await;
        assert_eq!(
            invalid.status(),
            StatusCode::UNAUTHORIZED,
            "invalid token should be rejected before request parsing for {}",
            $uri
        );
        assert_error(invalid, StatusCode::UNAUTHORIZED, "unauthorized").await;
    }};
}

#[actix_web::test]
async fn every_protected_endpoint_rejects_missing_and_invalid_bearer_tokens() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);

    assert_unauthorized!(app, get, "/api/auth/me");
    assert_unauthorized!(app, post, "/api/auth/logout");
    assert_unauthorized!(app, get, "/api/cases");
    assert_unauthorized!(app, post, "/api/cases");
    assert_unauthorized!(app, get, "/api/cases/not-used");
    assert_unauthorized!(app, patch, "/api/cases/not-used/status");
    assert_unauthorized!(app, post, "/api/cases/not-used/members");
    assert_unauthorized!(app, post, "/api/cases/not-used/clues");
    assert_unauthorized!(app, patch, "/api/clues/not-used/review");
}
