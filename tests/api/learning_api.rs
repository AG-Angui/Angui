use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::{Value, json};

use crate::support::{FAMILY, LEARNER, TestContext, assert_error};

#[actix_web::test]
async fn learning_endpoints_keep_empty_catalog_safe_and_require_authentication() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);

    let unauthorized = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/learning/resources")
            .to_request(),
    )
    .await;
    assert_error(unauthorized, StatusCode::UNAUTHORIZED, "unauthorized").await;

    let resources = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/learning/resources")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(resources.status(), StatusCode::OK);
    let resources: Value = test::read_body_json(resources).await;
    assert_eq!(resources, json!([]));

    let questions = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/learning/questions")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(questions.status(), StatusCode::OK);
    let questions: Value = test::read_body_json(questions).await;
    assert_eq!(questions, json!([]));

    let no_source_answer = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/knowledge/ask")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "question": "未发布题库怎么使用" }))
            .to_request(),
    )
    .await;
    assert_eq!(no_source_answer.status(), StatusCode::OK);
    let no_source_answer: Value = test::read_body_json(no_source_answer).await;
    assert_eq!(no_source_answer["certainty"], "insufficient_sources");
    assert_eq!(no_source_answer["sources"], json!([]));
}
