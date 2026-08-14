use actix_web::{body::MessageBody, dev::ServiceResponse, http::StatusCode, test};
use angui::{
    ai_gateway::AiGateway,
    amap_service::AmapService,
    app_state::AppState,
    models::{
        AddCaseMemberRequest, AuthenticatedUser, CreateCaseRequest, CreateClueRequest,
        LoginRequest, UpdateCaseStatusRequest,
    },
    rate_limit::LoginRateLimiter,
    roles::CaseRole,
    services::{auth_service, case_service},
};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::{Value, json};
use uuid::Uuid;

pub const PASSWORD: &str = "demo-password-123";
pub const FAMILY: &str = "family@demo.invalid";
pub const COMMANDER: &str = "commander@demo.invalid";
pub const VOLUNTEER: &str = "volunteer@demo.invalid";
pub const LEARNER: &str = "learner@demo.invalid";
pub const ADMIN: &str = "admin@demo.invalid";

pub struct TestContext {
    pub(crate) database: DatabaseConnection,
}

impl TestContext {
    pub async fn new() -> Self {
        let database_url = format!(
            "sqlite:file:angui-api-test-{}?mode=memory&cache=shared",
            Uuid::new_v4()
        );
        let database = Database::connect(&database_url)
            .await
            .expect("test database should connect");
        Migrator::up(&database, None)
            .await
            .expect("test migrations should succeed");
        database
            .execute_unprepared(
                "UPDATE intake_question_definitions SET status = CASE WHEN version = 2 THEN 'active' WHEN version = 3 THEN 'disabled' ELSE status END",
            )
            .await
            .expect("legacy intake contract should be available to existing tests");
        auth_service::bootstrap_demo_users(&database, PASSWORD)
            .await
            .expect("test demo users should bootstrap");
        Self { database }
    }

    pub async fn enable_intake_report_details(&self) {
        self.database
            .execute_unprepared(
                "UPDATE intake_question_definitions SET status = CASE WHEN version = 2 THEN 'disabled' WHEN version = 3 THEN 'active' ELSE status END",
            )
            .await
            .expect("intake report detail question set should be enabled");
    }

    pub fn app_state(&self) -> AppState {
        AppState {
            db: self.database.clone(),
            frontend_origin: "http://localhost:5173".to_owned(),
            session_ttl_hours: 8,
            intake_answer_hard_max: 2_000,
            attachment_storage_directory: std::env::temp_dir().join("angui-api-test-attachments"),
            attachment_max_image_bytes: 5 * 1024 * 1024,
            attachment_max_per_case: 12,
            case_place_types: vec![
                "frequent".to_owned(),
                "key_location".to_owned(),
                "last_seen_context".to_owned(),
                "medical".to_owned(),
                "shelter".to_owned(),
                "other".to_owned(),
            ],
            amap_service: AmapService::disabled(),
            ai_gateway: AiGateway::from_configurations(Vec::new())
                .expect("empty AI provider configuration should be valid"),
            login_limiter: LoginRateLimiter::default(),
            message_delivery: angui::message_delivery::MessageDelivery::disabled(),
        }
    }

    pub async fn token(&self, email: &str) -> String {
        auth_service::login(
            &self.database,
            LoginRequest {
                email: email.to_owned(),
                password: PASSWORD.to_owned(),
            },
            8,
        )
        .await
        .expect("fixture login should succeed")
        .token
    }

    pub async fn authenticated(&self, email: &str) -> AuthenticatedUser {
        let token = self.token(email).await;
        auth_service::authenticate(&self.database, &token)
            .await
            .expect("fixture session should authenticate")
    }

    pub async fn create_case(&self) -> String {
        let family = self.authenticated(FAMILY).await;
        case_service::create_case(&self.database, &family, create_case_request())
            .await
            .expect("fixture case should be created")
            .id
    }

    pub async fn add_member(&self, case_id: &str, actor_email: &str, email: &str, role: &str) {
        let actor = self.authenticated(actor_email).await;
        case_service::add_case_member(
            &self.database,
            &actor,
            case_id,
            AddCaseMemberRequest {
                email: email.to_owned(),
                case_role: role
                    .parse::<CaseRole>()
                    .expect("fixture role should be valid"),
            },
        )
        .await
        .expect("fixture case member should be added");
    }

    pub async fn create_clue(&self, case_id: &str, actor_email: &str) -> String {
        let actor = self.authenticated(actor_email).await;
        case_service::create_clue(&self.database, &actor, case_id, create_clue_request())
            .await
            .expect("fixture clue should be created")
            .id
    }

    pub async fn close_case(&self, case_id: &str) {
        let commander = self.authenticated(COMMANDER).await;
        case_service::update_case_status(
            &self.database,
            &commander,
            case_id,
            UpdateCaseStatusRequest {
                status: "closed".to_owned(),
            },
        )
        .await
        .expect("fixture case should close");
    }
}

pub fn create_case_request() -> CreateCaseRequest {
    CreateCaseRequest {
        display_name: "测试老人".to_owned(),
        age: Some(76),
        gender: Some("female".to_owned()),
        physical_description: Some("测试体貌".to_owned()),
        clothing_description: Some("测试衣着".to_owned()),
        health_notes: Some("仅用于测试的健康备注".to_owned()),
        last_seen_at: Some("2026-07-13T09:00:00Z".to_owned()),
        last_seen_location: Some("测试公园北门".to_owned()),
    }
}

pub fn create_case_json() -> Value {
    json!({
        "display_name": "测试老人",
        "age": 76,
        "gender": "female",
        "physical_description": "测试体貌",
        "clothing_description": "测试衣着",
        "health_notes": "仅用于测试的健康备注",
        "last_seen_at": "2026-07-13T09:00:00Z",
        "last_seen_location": "测试公园北门"
    })
}

pub fn create_clue_request() -> CreateClueRequest {
    CreateClueRequest {
        source: "family".to_owned(),
        content: "测试线索：曾向测试市场方向步行".to_owned(),
        source_type: None,
        raw_record_reference: None,
        occurred_at: Some("2026-07-13T09:10:00Z".to_owned()),
        location_text: Some("测试公园北门".to_owned()),
        location_precision: None,
        next_action: None,
        linked_task_reference: None,
        attachment_ids: Vec::new(),
    }
}

pub fn create_clue_json() -> Value {
    json!({
        "source": "family",
        "content": "测试线索：曾向测试市场方向步行",
        "occurred_at": "2026-07-13T09:10:00Z",
        "location_text": "测试公园北门"
    })
}

#[macro_export]
macro_rules! init_api_app {
    ($context:expr) => {{
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new($context.app_state()))
                .configure(angui::routes::configure),
        )
        .await
    }};
}

pub async fn assert_error<B>(response: ServiceResponse<B>, status: StatusCode, code: &str)
where
    B: MessageBody + 'static,
{
    assert_eq!(response.status(), status);
    let body: Value = test::read_body_json(response).await;
    assert_eq!(body["error"]["code"], code);
}

pub async fn read_sse_completed_json<B>(response: ServiceResponse<B>) -> Value
where
    B: MessageBody + 'static,
{
    let body = test::read_body(response).await;
    let stream = std::str::from_utf8(&body).expect("SSE response should be valid UTF-8");
    for frame in stream.split("\n\n") {
        let event = frame
            .lines()
            .find_map(|line| line.strip_prefix("event:").map(str::trim));
        if event != Some("completed") {
            continue;
        }
        let payload = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        return serde_json::from_str(&payload)
            .expect("completed SSE event should contain a JSON value");
    }
    panic!("SSE response should contain a completed event: {stream}");
}
