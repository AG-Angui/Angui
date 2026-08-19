use actix_web::{
    http::{StatusCode, header},
    test,
};
use serde_json::{Value, json};

use crate::support::{ADMIN, FAMILY, LEARNER, TestContext, assert_error};

macro_rules! transition {
    ($app:expr, $context:expr, $item_id:expr, $action:expr) => {{
        let response = test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!(
                    "/api/admin/knowledge-items/{}/{}",
                    $item_id, $action
                ))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", $context.token(ADMIN).await),
                ))
                .to_request(),
        )
        .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{} should succeed",
            $action
        );
    }};
}

macro_rules! search {
    ($app:expr, $context:expr, $base_id:expr, $query:expr, $email:expr) => {{
        let response = test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!("/api/knowledge-bases/{}/search", $base_id))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", $context.token($email).await),
                ))
                .set_json(json!({ "query": $query }))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        test::read_body_json(response).await
    }};
}

#[actix_web::test]
async fn knowledge_rag_requires_governed_publication_before_search_and_chat() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let admin_token = context.token(ADMIN).await;

    let family_management = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .set_json(json!({
                "name": "Family cannot manage RAG",
                "description": "",
                "visibility": "learner"
            }))
            .to_request(),
    )
    .await;
    assert_error(family_management, StatusCode::FORBIDDEN, "forbidden").await;

    let base = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({
                "name": "RAG Safety Manual",
                "description": "Published safety material",
                "visibility": "learner"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(base.status(), StatusCode::CREATED);
    let base: Value = test::read_body_json(base).await;
    let base_id = base["id"].as_str().expect("knowledge base id").to_owned();

    let item = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-bases/{base_id}/items"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({
                "title": "First hour emergency checklist",
                "summary": "Immediate steps for the first hour",
                "content": "Call emergency services and share a recent photo with responders.",
                "category": "safety",
                "keywords": ["emergency", "first hour"],
                "source_name": "Approved safety handbook",
                "source_url": "https://example.invalid/safety",
                "visibility": "learner"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(item.status(), StatusCode::CREATED);
    let item: Value = test::read_body_json(item).await;
    let item_id = item["knowledge_item_id"]
        .as_str()
        .expect("knowledge item id")
        .to_owned();

    let search_before_publish: Value = search!(&app, &context, &base_id, "emergency", LEARNER);
    assert_eq!(search_before_publish["results"], json!([]));

    transition!(&app, &context, &item_id, "review");
    transition!(&app, &context, &item_id, "publish");

    let search_after_publish: Value = search!(&app, &context, &base_id, "emergency", LEARNER);
    let results = search_after_publish["results"]
        .as_array()
        .expect("search results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["knowledge_item_id"], item_id);
    assert!(
        results[0]["score"]
            .as_f64()
            .is_some_and(|score| score > 0.0)
    );

    let chat = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/knowledge-bases/{base_id}/chat"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "query": "emergency", "limit": 1 }))
            .to_request(),
    )
    .await;
    assert_eq!(chat.status(), StatusCode::OK);
    let chat: Value = test::read_body_json(chat).await;
    assert_eq!(chat["certainty"], "rule_based");
    assert_eq!(chat["sources"][0]["knowledge_item_id"], item_id);
    assert!(
        chat["answer"]
            .as_str()
            .is_some_and(|answer| answer.contains(&item_id))
    );

    transition!(&app, &context, &item_id, "withdraw");
    let search_after_withdrawal: Value = search!(&app, &context, &base_id, "emergency", LEARNER);
    assert_eq!(search_after_withdrawal["results"], json!([]));

    let invalid_limit = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/knowledge-bases/{base_id}/search"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "query": "emergency", "limit": 0 }))
            .to_request(),
    )
    .await;
    assert_error(invalid_limit, StatusCode::BAD_REQUEST, "validation_error").await;
}

#[actix_web::test]
async fn knowledge_csv_import_previews_invalid_rows_and_only_imports_valid_rows() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let admin_token = context.token(ADMIN).await;

    let base = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({
                "name": "CSV Import Base",
                "description": "",
                "visibility": "learner"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(base.status(), StatusCode::CREATED);
    let base: Value = test::read_body_json(base).await;
    let base_id = base["id"].as_str().expect("knowledge base id").to_owned();

    let boundary = "knowledge-csv-boundary";
    let long_title = "x".repeat(241);
    let too_many_keywords = (0..21)
        .map(|index| format!("tag-{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let csv = format!(
        "knowledge_base_id,title,content,summary,category,keywords,source_name,source_url,visibility\n{base_id},Imported emergency guide,Call emergency services,Immediate response,safety,\"emergency,response\",CSV handbook,https://example.invalid/csv,learner\nwrong-base,Rejected row,Should not import,Invalid,safety,invalid,CSV handbook,https://example.invalid/csv,learner\n{base_id},{long_title},Long title content,Invalid,safety,invalid,CSV handbook,https://example.invalid/csv,learner\n{base_id},Too many keywords,Keyword validation,Invalid,safety,\"{too_many_keywords}\",CSV handbook,https://example.invalid/csv,learner\n"
    );
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"knowledge.csv\"\r\nContent-Type: text/csv\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(csv.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let preview = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!(
                "/api/admin/knowledge-bases/{base_id}/imports/preview"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(body)
            .to_request(),
    )
    .await;
    assert_eq!(preview.status(), StatusCode::CREATED);
    let preview: Value = test::read_body_json(preview).await;
    assert_eq!(preview["status"], "previewed");
    assert_eq!(preview["total_rows"], 4);
    assert_eq!(preview["valid_rows"], 1);
    assert_eq!(preview["invalid_rows"], 3);
    assert_eq!(preview["rows"][0]["status"], "valid");
    assert_eq!(preview["rows"][1]["status"], "invalid");
    assert_eq!(preview["rows"][2]["status"], "invalid");
    assert_eq!(preview["rows"][3]["status"], "invalid");
    let batch_id = preview["id"].as_str().expect("import batch id");

    let confirm = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-imports/{batch_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(confirm.status(), StatusCode::OK);
    let confirm: Value = test::read_body_json(confirm).await;
    assert_eq!(confirm["status"], "confirmed");
    assert_eq!(confirm["rows"][0]["status"], "imported");
    assert_eq!(confirm["rows"][1]["status"], "invalid");
    assert_eq!(confirm["rows"][2]["status"], "invalid");
    assert_eq!(confirm["rows"][3]["status"], "invalid");
    assert!(confirm["rows"][0]["knowledge_item_id"].is_string());
    assert!(confirm["rows"][1]["knowledge_item_id"].is_null());

    let items = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/admin/knowledge-bases/{base_id}/items"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(items.status(), StatusCode::OK);
    let items: Value = test::read_body_json(items).await;
    assert_eq!(items.as_array().map(Vec::len), Some(1));
    assert_eq!(items[0]["title"], "Imported emergency guide");

    let learner_search: Value = search!(&app, &context, &base_id, "emergency", LEARNER);
    assert_eq!(learner_search["results"], json!([]));
}
