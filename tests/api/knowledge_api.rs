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

    let unfiltered = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/knowledge-bases/learning-materials/chat")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "query": "Beacon", "category": "", "tag": "   " }))
            .to_request(),
    )
    .await;
    assert_eq!(unfiltered.status(), StatusCode::OK);
    let answer: Value = test::read_body_json(unfiltered).await;
    assert_eq!(answer["sources"].as_array().expect("sources").len(), 2);

    let category_only = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/knowledge-bases/learning-materials/chat")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .set_json(json!({ "query": "Beacon", "category": "search", "tag": "" }))
            .to_request(),
    )
    .await;
    assert_eq!(category_only.status(), StatusCode::OK);
    let answer: Value = test::read_body_json(category_only).await;
    assert_eq!(answer["sources"].as_array().expect("sources").len(), 1);
    assert_eq!(answer["sources"][0]["title"], "Beacon field guide");

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
    body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"source_metadata\"\r\n\r\n{{\"version\":\"1.0\",\"field_descriptions\":{{\"judgment\":\"Original answer\"}}}}\r\n--{boundary}--\r\n").as_bytes());

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
    assert_eq!(preview["source_metadata"]["version"], "1.0");
    assert_eq!(preview["status"], "previewed");
    assert_eq!(preview["total_rows"], 4);
    assert_eq!(preview["valid_rows"], 1);
    assert_eq!(preview["invalid_rows"], 3);
    assert_eq!(preview["rows"][0]["status"], "valid");
    assert_eq!(
        preview["rows"][0]["raw_data"][1],
        "Imported emergency guide"
    );
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
    assert_eq!(confirm["source_metadata"], preview["source_metadata"]);
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
    assert_eq!(items[0]["status"], "published");

    let learner_search: Value = search!(&app, &context, &base_id, "emergency", LEARNER);
    assert_eq!(learner_search["results"].as_array().unwrap().len(), 1);
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
async fn knowledge_csv_import_preserves_separate_source_fields_and_validates_metadata() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let token = context.token(ADMIN).await;
    let boundary = "structured-experience-import";
    let mut csv = "knowledge_base_id,title,content,summary,category,keywords,source_name,source_url,visibility,external_id,metadata\n".to_owned();
    for (external_id, metadata) in [
        ("EXP-1", r#"{"source_fields":{"原始回答":"First source"}}"#),
        ("EXP-2", r#"{"source_fields":{"原始回答":"Second source"}}"#),
        ("EXP-1", r#"{"source_fields":{"原始回答":"First source"}}"#),
        ("EXP-3", "[]"),
        ("EXP-4", "invalid json"),
    ] {
        let metadata_cell = format!("\"{}\"", metadata.replace('"', "\"\""));
        csv.push_str(&format!("learning-materials,Support volunteers,Assign a mentor,Training,safety,mentor,Handbook,,learner,{external_id},{metadata_cell}\n"));
    }
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"experiences.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
    );
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases/learning-materials/imports/preview")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(body)
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let preview: Value = test::read_body_json(response).await;
    assert_eq!(preview["valid_rows"], 2);
    for (row, expected) in preview["rows"].as_array().unwrap().iter().zip([
        "valid",
        "valid",
        "duplicate",
        "invalid",
        "invalid",
    ]) {
        assert_eq!(row["status"], expected);
    }
    let batch_id = preview["id"].as_str().unwrap();
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-imports/{batch_id}/confirm"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/knowledge-bases/learning-materials/items")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let items: Value = test::read_body_json(response).await;
    let items = items.as_array().unwrap();
    assert_eq!(items.len(), 2);
    for (id, answer) in [("EXP-1", "First source"), ("EXP-2", "Second source")] {
        let item = items.iter().find(|item| item["external_id"] == id).unwrap();
        assert_eq!(item["content"], "Assign a mentor");
        assert_eq!(item["category"], "safety");
        assert_eq!(item["metadata"]["source_fields"]["原始回答"], answer);
    }
}

#[actix_web::test]
async fn knowledge_csv_bom_import_publishes_and_lists_only_one_server_page() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let token = context.token(ADMIN).await;
    let mut csv = "\u{feff}knowledge_base_id,title,content,summary,category,keywords,source_name,source_url,visibility\n".to_owned();
    for index in 0..23 {
        csv.push_str(&format!(
            "learning-materials,Page guide {index},Guidance {index},,Safety,,Handbook,,learner\n"
        ));
    }
    let boundary = "bom-page-import";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"bom.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
    );
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/knowledge-bases/learning-materials/imports/preview")
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .insert_header((
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            ))
            .set_payload(body)
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let preview: Value = test::read_body_json(response).await;
    assert_eq!(preview["valid_rows"], 23);
    let batch_id = preview["id"].as_str().unwrap();
    for _ in 0..2 {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/admin/knowledge-imports/{batch_id}/confirm"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let mut identifiers = std::collections::HashSet::new();
    for (page, count) in [(1, 10), (2, 10), (3, 3)] {
        let response = test::call_service(&app, test::TestRequest::get()
            .uri(&format!("/api/admin/knowledge-bases/learning-materials/items/page?page={page}&page_size=10"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}"))).to_request()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let data: Value = test::read_body_json(response).await;
        assert_eq!(data["total"], 23);
        assert_eq!(data["page"], page);
        let items = data["items"].as_array().unwrap();
        assert_eq!(items.len(), count);
        for item in items {
            assert_eq!(item["status"], "published");
            assert!(identifiers.insert(item["knowledge_item_id"].as_str().unwrap().to_owned()));
        }
    }
    assert_eq!(identifiers.len(), 23);
    for query in ["page=0&page_size=10", "page=1&page_size=500"] {
        let response = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!(
                    "/api/admin/knowledge-bases/learning-materials/items/page?{query}"
                ))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/knowledge-bases/learning-materials/items/page")
            .insert_header((
                header::AUTHORIZATION,
                format!("Bearer {}", context.token(LEARNER).await),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
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

#[actix_web::test]
async fn knowledge_experience_fields_round_trip_without_exposing_source_metadata() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let token = context.token(ADMIN).await;
    let base_id = "learning-materials";
    let mut terms = Vec::new();
    for body in [
        json!({"kind":"category", "name":"Movement", "description":"Behavior patterns"}),
        json!({"kind":"tag", "name":"Tag"}),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/admin/knowledge-bases/{base_id}/terms"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        terms.push(test::read_body_json::<Value, _>(response).await);
    }
    let parent = terms[0]["id"].as_str().unwrap();
    for (body, status) in [
        (
            json!({"kind":"category", "name":"Invalid tag parent", "parent_id":terms[1]["id"]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"kind":"tag", "name":"Invalid child tag", "parent_id":parent}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/api/admin/knowledge-bases/{base_id}/terms"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), status);
    }
    let response = test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/admin/knowledge-bases/{base_id}/terms"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({"kind":"category", "name":"Daily walk", "parent_id":parent, "description":"Routes"})).to_request()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let child: Value = test::read_body_json(response).await;
    assert_eq!(child["parent_id"], parent);
    assert_eq!(child["description"], "Routes");
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/admin/knowledge-bases/{base_id}/items"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({"title":"Parent category", "content":"Guidance", "category_id":parent, "visibility":"learner"}))
            .to_request(),
    ).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let metadata = json!({"original_answer":"Unedited answer", "priority":null, "verification_note":"Unverified"});
    let response = test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/admin/knowledge-bases/{base_id}/items"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({"title":"Walk experience", "content":"Beacon route guidance", "category_id":child["id"], "category":"Incorrect snapshot", "external_id":"EXP-000001", "metadata":metadata, "visibility":"learner"})).to_request()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let item: Value = test::read_body_json(response).await;
    let id = item["knowledge_item_id"].as_str().unwrap();
    assert_eq!(item["category"], "Daily walk");
    assert_eq!(item["category_id"], child["id"]);
    assert_eq!(item["external_id"], "EXP-000001");
    assert_eq!(item["metadata"], metadata);
    for (body, status) in [
        (json!({"metadata": []}), StatusCode::BAD_REQUEST),
        (
            json!({"summary":"Updated", "category":"Wrong snapshot"}),
            StatusCode::OK,
        ),
    ] {
        let response = test::call_service(
            &app,
            test::TestRequest::patch()
                .uri(&format!("/api/admin/knowledge-items/{id}"))
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .set_json(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), status);
        if status == StatusCode::OK {
            let updated: Value = test::read_body_json(response).await;
            assert_eq!(updated["external_id"], "EXP-000001");
            assert_eq!(updated["metadata"], metadata);
            assert_eq!(updated["category"], "Daily walk");
        }
    }
    transition!(&app, &context, id, "deidentify", ADMIN);
    transition!(&app, &context, id, "review", ADMIN2);
    transition!(&app, &context, id, "publish", ADMIN);
    let results: Value = search!(&app, &context, base_id, "Beacon", LEARNER);
    assert_eq!(results["results"].as_array().unwrap().len(), 1);
    assert!(results["results"][0].get("metadata").is_none());
    assert!(results["results"][0].get("external_id").is_none());
    // Source-only corrections must not be discarded as duplicate body text.
    let response = test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/admin/knowledge-bases/{base_id}/items"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({"title":"Walk experience", "summary":"Updated", "content":"Beacon route guidance", "external_id":"EXP-000001", "metadata":{"verification_note":"Independently verified"}, "visibility":"learner", "previous_version_id":id})).to_request()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let correction: Value = test::read_body_json(response).await;
    let correction_id = correction["knowledge_item_id"].as_str().unwrap();
    let response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/admin/knowledge-items/{correction_id}"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .set_json(json!({"external_id":"", "metadata":{}}))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cleared: Value = test::read_body_json(response).await;
    assert!(cleared["external_id"].is_null());
    assert_eq!(cleared["metadata"], json!({}));
}

#[actix_web::test]
async fn knowledge_import_rejects_invalid_source_metadata() {
    let context = TestContext::new().await;
    let app = crate::init_api_app!(&context);
    let token = context.token(ADMIN).await;
    let oversized = json!({"note":"x".repeat(128 * 1024)}).to_string();
    for metadata_fields in [vec!["[]"], vec![oversized.as_str()], vec!["{}", "{}"]] {
        let boundary = "metadata-validation-boundary";
        let mut body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.csv\"\r\nContent-Type: text/csv\r\n\r\nknowledge_base_id,title,content\nlearning-materials,Guide,Guidance\r\n"
        );
        for metadata in metadata_fields {
            body.push_str(&format!("--{boundary}\r\nContent-Disposition: form-data; name=\"source_metadata\"\r\n\r\n{metadata}\r\n"));
        }
        body.push_str(&format!("--{boundary}--\r\n"));
        let response = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/admin/knowledge-bases/learning-materials/imports/preview")
                .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
                .insert_header((
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                ))
                .set_payload(body)
                .to_request(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
