use actix_web::{
    http::{StatusCode, header},
    test,
};
use angui::entities::audit_events;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::support::{ADMIN, ADMIN2, FAMILY, LEARNER, TestContext, assert_error};

macro_rules! transition {
    ($app:expr, $context:expr, $item_id:expr, $action:expr, $email:expr) => {{
        let response = test::call_service(
            $app,
            test::TestRequest::post()
                .uri(&format!(
                    "/api/admin/knowledge-items/{}/{}",
                    $item_id, $action
                ))
                .insert_header((
                    header::AUTHORIZATION,
                    format!("Bearer {}", $context.token($email).await),
                ))
                .set_json(json!({ "reason": "验收测试中的内容治理操作" }))
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

    transition!(&app, &context, &item_id, "deidentify", ADMIN);
    let self_review = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-items/{item_id}/review"))
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(ADMIN).await),
            ))
            .set_json(json!({ "reason": "提交人不可审核自己的内容" }))
            .to_request(),
    )
    .await;
    assert_error(self_review, StatusCode::FORBIDDEN, "forbidden").await;
    transition!(&app, &context, &item_id, "review", ADMIN2);
    transition!(&app, &context, &item_id, "publish", ADMIN);

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

    let preview = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/knowledge-items/{item_id}/learner-preview"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(preview.status(), StatusCode::OK);
    let preview: Value = test::read_body_json(preview).await;
    assert_eq!(preview["content"], results[0]["content"]);

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

    transition!(&app, &context, &item_id, "withdraw", ADMIN);
    let withdrawn_preview = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/admin/knowledge-items/{item_id}/learner-preview"
            ))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_error(withdrawn_preview, StatusCode::NOT_FOUND, "not_found").await;
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
async fn learner_answer_uses_every_keyword_match_even_when_a_legacy_limit_is_sent() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let admin_token = context.token(ADMIN).await;
    for (title, category, tag) in [
        ("Beacon field guide", "search", "route"),
        ("Beacon safety guide", "safety", "checklist"),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/admin/knowledge-bases/learning-materials/items")
                .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
                .set_json(json!({
                    "title": title, "summary": "Beacon reference", "content": format!("{title}: verify the report before acting."),
                    "category": category, "keywords": [tag], "source_name": "Approved guide", "visibility": "learner"
                }))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let item: Value = test::read_body_json(response).await;
        let id = item["knowledge_item_id"].as_str().expect("item id");
        transition!(&app, &context, id, "deidentify", ADMIN);
        transition!(&app, &context, id, "review", ADMIN2);
        transition!(&app, &context, id, "publish", ADMIN);
    }

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/knowledge-bases/learning-materials/chat")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "query": "Beacon", "limit": 1 }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let answer: Value = test::read_body_json(response).await;
    assert_eq!(answer["sources"].as_array().expect("sources").len(), 2);

    let filtered = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/knowledge-bases/learning-materials/chat")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "query": "Beacon", "category": "search", "tag": "route" }))
            .to_request(),
    )
    .await;
    assert_eq!(filtered.status(), StatusCode::OK);
    let answer: Value = test::read_body_json(filtered).await;
    assert_eq!(answer["sources"].as_array().expect("sources").len(), 1);
    assert_eq!(answer["sources"][0]["title"], "Beacon field guide");
}

#[actix_web::test]
async fn knowledge_attachment_download_is_audited_and_withdrawal_revokes_access() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let admin_token = context.token(ADMIN).await;
    let created = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases/learning-materials/items")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({
                "title": "Attachment safety guide", "summary": "Approved attachment",
                "content": "Read the approved attachment.", "category": "safety", "keywords": ["attachment"],
                "source_name": "Approved guide", "visibility": "learner"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created: Value = test::read_body_json(created).await;
    let item_id = created["knowledge_item_id"].as_str().expect("item id");
    let boundary = "knowledge-attachment-boundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"guide.pdf\"\r\nContent-Type: application/pdf\r\n\r\n%PDF-1.4\n%%EOF\r\n--{boundary}--\r\n"
    );
    let uploaded = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-items/{item_id}/attachments"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(body)
            .to_request(),
    )
    .await;
    assert_eq!(uploaded.status(), StatusCode::CREATED);
    let attachment: Value = test::read_body_json(uploaded).await;
    let attachment_id = attachment["id"].as_str().expect("attachment id");
    transition!(&app, &context, item_id, "deidentify", ADMIN);
    transition!(&app, &context, item_id, "review", ADMIN2);
    transition!(&app, &context, item_id, "publish", ADMIN);

    let url = format!("/api/admin/knowledge-items/{item_id}/attachments/{attachment_id}");
    let learner_token = context.token(LEARNER).await;
    let download = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&url)
            .insert_header((header::AUTHORIZATION, format!("Bearer {learner_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(download.status(), StatusCode::OK);
    transition!(&app, &context, item_id, "withdraw", ADMIN);
    let denied = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&url)
            .insert_header((header::AUTHORIZATION, format!("Bearer {learner_token}")))
            .to_request(),
    )
    .await;
    assert_error(denied, StatusCode::NOT_FOUND, "not_found").await;

    for action in [
        "knowledge.attachment_downloaded",
        "knowledge.attachment_access_denied",
    ] {
        let event = audit_events::Entity::find()
            .filter(audit_events::Column::Action.eq(action))
            .filter(audit_events::Column::EntityId.eq(attachment_id))
            .one(&context.database)
            .await
            .expect("audit query should succeed");
        assert!(event.is_some(), "missing {action} audit");
    }
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
#[actix_web::test]
async fn knowledge_csv_preview_rejects_unauthorized_uploads_and_long_file_names() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let boundary = "knowledge-preview-boundary";
    let payload = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"knowledge.csv\"\r\nContent-Type: text/csv\r\n\r\ncontent\r\n--{boundary}--\r\n"
    );

    let unauthorized = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases/unknown/imports/preview")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(FAMILY).await),
            ))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(payload)
            .to_request(),
    )
    .await;
    assert_error(unauthorized, StatusCode::FORBIDDEN, "forbidden").await;

    let long_file_name = format!("{}.csv", "x".repeat(252));
    let long_name_payload = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{long_file_name}\"\r\nContent-Type: text/csv\r\n\r\ncontent\r\n--{boundary}--\r\n"
    );
    let too_long = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases/unknown/imports/preview")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(ADMIN).await),
            ))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(long_name_payload)
            .to_request(),
    )
    .await;
    assert_error(too_long, StatusCode::BAD_REQUEST, "validation_error").await;
}

#[actix_web::test]
async fn knowledge_terms_are_governed_per_base_and_archived_with_audit_reason() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let admin_token = context.token(ADMIN).await;
    let base = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases")
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({
                "name": "Vocabulary base",
                "description": "Controlled terms",
                "visibility": "learner"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(base.status(), StatusCode::CREATED);
    let base: Value = test::read_body_json(base).await;
    let base_id = base["id"].as_str().expect("base id");

    let create = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-bases/{base_id}/terms"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({"kind": "category", "name": "安全巡查"}))
            .to_request(),
    )
    .await;
    assert_eq!(create.status(), StatusCode::CREATED);
    let term: Value = test::read_body_json(create).await;
    let term_id = term["id"].as_str().expect("term id").to_owned();

    let duplicate = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-bases/{base_id}/terms"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({"kind": "category", "name": "安全巡查"}))
            .to_request(),
    )
    .await;
    assert_error(duplicate, StatusCode::CONFLICT, "conflict").await;

    let list = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/admin/knowledge-bases/{base_id}/terms"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .to_request(),
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list: Value = test::read_body_json(list).await;
    assert_eq!(list[0]["status"], "active");

    let disable = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-terms/{term_id}/disable"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
            .set_json(json!({"reason": "术语已统一到新版安全规范"}))
            .to_request(),
    )
    .await;
    assert_eq!(disable.status(), StatusCode::OK);
    let disabled: Value = test::read_body_json(disable).await;
    assert_eq!(disabled["status"], "archived");
}
