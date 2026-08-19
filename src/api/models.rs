use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    entities::{
        cases, clue_attributions, clues, elder_profiles, intake_session_answers, intake_sessions,
        task_applications, task_assignments, tasks,
    },
    roles::{AccountType, CaseRole, GlobalCapability},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCollaborationSpaceRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinCollaborationSpaceRequest {
    pub location_consent: bool,
    pub consent_version: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CollaborationSpaceResponse {
    pub id: String,
    pub case_id: String,
    pub name: String,
    pub status: String,
    pub created_by_user_id: String,
    pub created_at: String,
    pub archived_at: Option<String>,
    pub current_version: i32,
    pub member_status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SpaceMemberResponse {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub joined_at: String,
    pub left_at: Option<String>,
    pub location_consent_granted: bool,
}

#[derive(Debug, Serialize)]
pub struct CollaborationSpaceSnapshotResponse {
    pub space: CollaborationSpaceResponse,
    pub members: Vec<SpaceMemberResponse>,
    pub version: i32,
}

#[derive(Debug, Deserialize)]
pub struct SpaceEventsQuery {
    pub after_version: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SpaceEventResponse {
    pub event_id: String,
    pub space_id: String,
    pub case_id: String,
    pub event_type: String,
    pub version: i32,
    pub occurred_at: String,
    pub visibility_scope: String,
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordSpaceLocationRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: f64,
    pub captured_at: String,
    pub operation_id: String,
}

#[derive(Debug, Serialize)]
pub struct SpaceLocationResponse {
    pub id: String,
    pub user_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: f64,
    pub captured_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSpaceMessageRequest {
    pub content: String,
    pub message_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpaceMessageResponse {
    pub id: String,
    pub sender_id: String,
    pub sender_display_name: String,
    pub message_type: String,
    pub content: String,
    pub sent_at: String,
    pub recalled_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VoiceReportResponse {
    pub id: String,
    pub reporter_id: String,
    pub content_type: String,
    pub byte_size: i64,
    pub status: String,
    pub created_at: String,
    pub failed_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<VoiceTranscriptResponse>,
}

#[derive(Debug, Serialize)]
pub struct VoiceTranscriptResponse {
    pub content: String,
    pub provider: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub account_type: AccountType,
    pub global_capabilities: Vec<GlobalCapability>,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningResourceQuery {
    pub resource_type: Option<String>,
    pub tag: Option<String>,
    pub category_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningCategoryResponse {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ManagedLearningCategoryResponse {
    #[serde(flatten)]
    pub category: LearningCategoryResponse,
    pub submitted_by_user_id: String,
    pub reviewed_by_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct LearningResourceResponse {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub resource_type: String,
    pub tags: Vec<String>,
    pub category: Option<LearningCategoryResponse>,
    pub source_name: String,
    pub source_url: Option<String>,
    pub previous_version_id: Option<String>,
    pub version: i32,
    pub effective_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningQuestionQuery {
    pub tag: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LearningQuestionResponse {
    pub id: String,
    pub prompt: String,
    pub question_type: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub options: Value,
    pub source_resource_id: String,
    pub previous_version_id: Option<String>,
    pub version: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitLearningAnswerRequest {
    pub selected_option_id: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitLearningAnswerResponse {
    pub question_id: String,
    pub is_correct: bool,
    pub explanation: String,
    pub source: LearningAnswerSource,
}

#[derive(Debug, Serialize)]
pub struct LearningAnswerSource {
    pub resource_id: String,
    pub title: String,
    pub version: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeAskRequest {
    pub question: String,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeAnswerResponse {
    pub answer: String,
    pub certainty: String,
    pub sources: Vec<LearningAnswerSource>,
    pub human_review_notice: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateKnowledgeBaseRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub visibility: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateKnowledgeBaseRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeBaseResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub visibility: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeImageInput {
    pub storage_path: String,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[serde(default = "empty_json_object")]
    pub metadata: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct KnowledgeImageResponse {
    pub id: String,
    pub storage_path: String,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub metadata: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateKnowledgeItemRequest {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub content: String,
    #[serde(default)]
    pub category: String,
    pub category_id: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub visibility: String,
    #[serde(default)]
    pub images: Vec<KnowledgeImageInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeSearchRequest {
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct KnowledgeSearchResultResponse {
    pub knowledge_item_id: String,
    pub title: String,
    pub content: String,
    pub score: f64,
    pub knowledge_base_id: String,
    pub version: i32,
    pub source_name: String,
    pub source_url: Option<String>,
    pub images: Vec<KnowledgeImageResponse>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeSearchResponse {
    pub results: Vec<KnowledgeSearchResultResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeChatRequest {
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeChatSourceResponse {
    pub knowledge_item_id: String,
    pub title: String,
    pub version: i32,
    pub score: f64,
    pub images: Vec<KnowledgeImageResponse>,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeChatResponse {
    pub answer: String,
    pub certainty: String,
    pub sources: Vec<KnowledgeChatSourceResponse>,
    pub human_review_notice: String,
}

fn empty_json_object() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLearningResourceRequest {
    pub title: String,
    pub summary: String,
    pub content: String,
    pub resource_type: String,
    pub tags: Vec<String>,
    pub category_id: Option<String>,
    pub source_name: String,
    pub source_url: Option<String>,
    pub previous_version_id: Option<String>,
    pub visibility: String,
    pub effective_at: String,
    pub permitted_use: String,
    pub submission_reason: String,
}

/// A learner may submit a draft but can never self-publish it. The resource
/// shape is intentionally shared with administrators so validation and the
/// independent review lifecycle cannot drift.
pub type SubmitLearningResourceDraftRequest = CreateLearningResourceRequest;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLearningCategoryRequest {
    pub name: String,
    pub submission_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLearningQuestionRequest {
    pub source_resource_id: String,
    pub prompt: String,
    pub question_type: String,
    pub difficulty: String,
    pub tags: Vec<String>,
    pub options: Value,
    pub correct_option_id: String,
    pub explanation: String,
    pub previous_version_id: Option<String>,
    pub visibility: String,
    pub effective_at: String,
    pub permitted_use: String,
    pub submission_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LearningContentActionRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct LearningContentLifecycleResponse {
    pub submitted_by_user_id: String,
    pub deidentified_by_user_id: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub published_by_user_id: Option<String>,
    pub withdrawn_by_user_id: Option<String>,
    pub state: String,
    pub permitted_use: String,
    pub events: Vec<LearningContentReviewEventResponse>,
}

#[derive(Debug, Serialize)]
pub struct LearningContentReviewEventResponse {
    pub event_type: String,
    pub actor_user_id: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ManagedLearningResourceResponse {
    #[serde(flatten)]
    pub resource: LearningResourceResponse,
    pub lifecycle: LearningContentLifecycleResponse,
}

#[derive(Debug, Serialize)]
pub struct ManagedLearningQuestionResponse {
    #[serde(flatten)]
    pub question: LearningQuestionResponse,
    pub lifecycle: LearningContentLifecycleResponse,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAccessRequest {
    pub email: String,
    pub display_name: String,
    pub requested_role: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyAccessRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewAccessRequest {
    pub action: String,
    pub role: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordSetupRequest {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AccessRequestResponse {
    pub id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AdminAccessRequestResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub requested_role: String,
    pub status: String,
    pub email_verified_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub account_type: AccountType,
    pub global_capabilities: Vec<GlobalCapability>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserPreferences {
    pub locale: String,
    pub reduced_motion: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            locale: "zh-CN".to_owned(),
            reduced_motion: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UserProfileResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub account_type: AccountType,
    pub global_capabilities: Vec<GlobalCapability>,
    pub team_name: Option<String>,
    pub avatar_reference: Option<String>,
    pub preferences: UserPreferences,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub avatar_reference: Option<String>,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminAuditEventQuery {
    pub case_id: Option<String>,
    pub entity_type: Option<String>,
    pub action: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminUserQuery {
    pub account_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAdminUserStatusRequest {
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeidentifyArchiveDraftRequest {
    pub outcome: String,
    pub reason: String,
    pub deidentified_material: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewArchiveDraftRequest {
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ArchiveReviewMaterialResponse {
    pub id: String,
    pub case_id: String,
    pub version: i32,
    pub parent_material_id: Option<String>,
    pub content: String,
    pub source_scope: Vec<String>,
    pub status: String,
    pub created_by_user_id: String,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub created_at: String,
    pub selected_for_ai: bool,
}

#[derive(Debug, Serialize)]
pub struct ArchiveReviewMaterialDiffResponse {
    pub from_version: i32,
    pub to_version: i32,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreArchiveReviewMaterialRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AdminAuditEventResponse {
    pub id: String,
    pub case_id: Option<String>,
    pub actor_user_id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AdminAuditEventPage {
    pub items: Vec<AdminAuditEventResponse>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct AdminUserResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub account_type: AccountType,
    pub global_capabilities: Vec<GlobalCapability>,
    pub status: String,
    pub created_at: String,
    pub last_session_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminUserPage {
    pub items: Vec<AdminUserResponse>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCaseRequest {
    pub display_name: String,
    pub age: Option<i16>,
    pub gender: Option<String>,
    pub physical_description: Option<String>,
    pub clothing_description: Option<String>,
    pub health_notes: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_location: Option<String>,
}

/// Starts a family-owned intake session. These values remain unconfirmed
/// collection input; clients cannot set a fact-confirmation state here.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CreateIntakeSessionRequest {
    #[serde(default)]
    pub initial_answers: IntakeInitialAnswers,
}

/// A single answer is kept separate from the candidate field generated from
/// it. Both remain unconfirmed until the later explicit confirmation flow.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitIntakeAnswerRequest {
    pub field: String,
    pub answer: String,
    #[serde(default)]
    pub replace: bool,
    pub structured: Option<IntakeStructuredFacts>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IntakeStructuredFacts {
    pub last_seen_at: Option<String>,
    pub last_seen_location: Option<IntakeLocation>,
    pub follow_up_at: Option<String>,
    pub follow_up_location: Option<IntakeLocation>,
    pub mobility: Option<String>,
    #[serde(default)]
    pub transport_modes: Vec<String>,
    pub companion_status: Option<String>,
    #[serde(default)]
    pub belongings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeLocation {
    pub name: String,
    pub longitude: f64,
    pub latitude: f64,
    pub coordinate_system: String,
}

/// A family-reviewed profile is intentionally distinct from the draft built
/// from intake answers. Only this request can create formal case records.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmIntakeSessionRequest {
    pub profile: ConfirmedIntakeProfile,
    pub human_confirmed: bool,
}

/// Starts the family-visible AI initial review. It does not create a case or
/// change any submitted answer; the supplied profile is kept as the exact
/// snapshot that must be submitted at the later second confirmation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartIntakeAiInitialReviewRequest {
    pub profile: ConfirmedIntakeProfile,
}

/// The family must explicitly acknowledge every AI-raised item before the
/// second confirmation. A blocking deterministic consistency check cannot be
/// acknowledged away and still requires an actual correction.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcknowledgeIntakeAiInitialReviewRequest {
    pub confirmed_issue_ids: Vec<String>,
    pub human_confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmedIntakeProfile {
    pub display_name: String,
    pub age: Option<i16>,
    pub gender: Option<String>,
    pub physical_description: Option<String>,
    pub clothing_description: Option<String>,
    pub health_notes: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_location: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeAiInitialReviewIssue {
    pub id: String,
    pub field: String,
    pub severity: String,
    pub evidence_summary: String,
    pub clarification_question: String,
    pub source_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakeAiInitialReviewResponse {
    pub session_id: String,
    pub status: String,
    pub degradation_status: String,
    pub issues: Vec<IntakeAiInitialReviewIssue>,
    pub blocking_assessments: Vec<IntakeAssessment>,
    pub generated_at: String,
    pub requires_family_acknowledgement: bool,
    pub ready_for_second_confirmation: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IntakeInitialAnswers {
    pub basic_information: Option<String>,
    pub police_report_status: Option<String>,
    pub family_phone: Option<String>,
    pub health_status: Option<String>,
    pub behavior_habits: Option<String>,
    pub last_seen: Option<String>,
    pub frequent_locations: Option<String>,
    pub belongings: Option<String>,
    pub transport_ability: Option<String>,
    pub follow_up_clues: Option<String>,
    pub suspicious_motive: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakeQuestion {
    pub field: String,
    pub prompt: String,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakeSessionResponse {
    pub id: String,
    pub status: String,
    pub question_set_version: i32,
    pub initial_answers: IntakeInitialAnswers,
    pub missing_fields: Vec<String>,
    pub phase: String,
    pub completed_phase_one_fields: Vec<String>,
    pub missing_phase_one_fields: Vec<String>,
    pub phase_transition_ready: bool,
    pub next_question: Option<IntakeQuestion>,
    pub guidance_mode: String,
    pub ai_initial_review_status: String,
    pub privacy_notice: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakePhotoResponse {
    pub id: String,
    pub original_filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub created_at: String,
}

impl IntakeSessionResponse {
    pub fn new(
        model: intake_sessions::Model,
        initial_answers: IntakeInitialAnswers,
        missing_fields: Vec<String>,
        next_question: Option<IntakeQuestion>,
    ) -> Self {
        let phase = IntakePhaseProgress::for_answers(&initial_answers, model.question_set_version);
        Self {
            id: model.id,
            status: model.status,
            question_set_version: model.question_set_version,
            initial_answers,
            missing_fields,
            phase: phase.current_phase,
            completed_phase_one_fields: phase.completed_phase_one_fields,
            missing_phase_one_fields: phase.missing_phase_one_fields,
            phase_transition_ready: phase.phase_transition_ready,
            next_question,
            guidance_mode: "rule_based".to_owned(),
            ai_initial_review_status: model.ai_initial_review_status,
            privacy_notice: "Answers are visible only to the session creator and, after case authorization, the case's authorized commanders. They are unconfirmed drafts and are not copied into audit metadata.".to_owned(),
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakeCandidateField {
    pub field: String,
    pub value: String,
    pub source: String,
    pub status: String,
    pub generated_at: String,
    pub model: Option<String>,
    pub template_version: Option<String>,
    pub source_text: String,
    pub confidence: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IntakeAssessment {
    pub field_path: String,
    pub conflict_type: String,
    pub severity: String,
    pub evidence_summary: String,
    pub suggested_action: String,
    pub route_estimate: Option<IntakeRouteEstimate>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IntakeRouteEstimate {
    pub distance_meters: u64,
    pub available_seconds: i64,
    pub minimum_seconds: Option<u64>,
    pub basis: String,
    pub degraded: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakeProfileDraft {
    pub id: String,
    pub status: String,
    pub source_scope: String,
    pub generated_at: String,
    pub provider_model: Option<String>,
    pub template_version: String,
    pub degradation_status: String,
    pub version: i32,
    pub requires_human_confirmation: bool,
    pub profile: IntakeProfileDraftFields,
    pub field_metadata: Vec<IntakeProfileDraftFieldMetadata>,
    pub missing_fields: Vec<String>,
    pub assessments: Vec<IntakeAssessment>,
    pub confirmation_blocked_reasons: Vec<String>,
    pub direction_hypotheses: Vec<IntakeDirectionHypothesis>,
}

/// Provenance for a non-empty field in an unconfirmed intake profile draft.
/// The value itself remains in `profile`; this metadata lets clients display
/// the draft's origin without treating it as a confirmed case fact.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IntakeProfileDraftFieldMetadata {
    pub field: String,
    pub source_field: String,
    pub source: String,
    pub status: String,
    pub generated_at: String,
    pub source_excerpt: Option<String>,
    pub provider_model: Option<String>,
    pub template_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IntakeProfileDraftFields {
    pub physical_description: Option<String>,
    pub clothing_description: Option<String>,
    pub health_notes: Option<String>,
    pub mobility_notes: Option<String>,
    pub transportation_ability: Option<String>,
    pub frequent_locations: Option<String>,
    pub last_seen_information: Option<String>,
    pub behavior_habits: Option<String>,
    pub suspicious_motive: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IntakeDirectionHypothesis {
    pub status: String,
    pub source_fields: Vec<String>,
    pub generated_at: String,
    pub uncertainty_notice: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConfirmIntakeSessionResponse {
    pub case_id: String,
    pub case_code: String,
    pub status: String,
    pub confirmation_status: String,
    pub confirmed_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubmitIntakeAnswerResponse {
    pub session_id: String,
    pub question_set_version: i32,
    pub status: String,
    pub raw_answer: String,
    pub candidate_fields: Vec<IntakeCandidateField>,
    pub missing_fields: Vec<String>,
    pub phase: String,
    pub completed_phase_one_fields: Vec<String>,
    pub missing_phase_one_fields: Vec<String>,
    pub phase_transition_ready: bool,
    pub assessments: Vec<IntakeAssessment>,
    pub next_question: Option<IntakeQuestion>,
    pub guidance_mode: String,
    pub ai_initial_review_status: String,
    pub privacy_notice: String,
    pub updated_at: String,
}

impl SubmitIntakeAnswerResponse {
    pub fn new(
        session: intake_sessions::Model,
        answer: intake_session_answers::Model,
        missing_fields: Vec<String>,
        next_question: Option<IntakeQuestion>,
        phase: IntakePhaseProgress,
        assessments: Vec<IntakeAssessment>,
    ) -> Self {
        Self {
            session_id: session.id,
            question_set_version: session.question_set_version,
            status: session.status,
            raw_answer: answer.raw_answer.clone(),
            candidate_fields: vec![IntakeCandidateField {
                field: answer.field_code,
                value: answer.candidate_value,
                source: answer.source,
                status: answer.status,
                generated_at: answer.generated_at,
                model: answer.model,
                template_version: answer.template_version,
                source_text: answer.raw_answer,
                confidence: None,
            }],
            missing_fields,
            phase: phase.current_phase,
            completed_phase_one_fields: phase.completed_phase_one_fields,
            missing_phase_one_fields: phase.missing_phase_one_fields,
            phase_transition_ready: phase.phase_transition_ready,
            next_question,
            assessments,
            guidance_mode: "rule_based".to_owned(),
            ai_initial_review_status: session.ai_initial_review_status,
            privacy_notice: "Answers and candidate fields are unconfirmed drafts. They remain visible only to the session creator and are not copied into audit metadata.".to_owned(),
            updated_at: session.updated_at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IntakePhaseProgress {
    pub current_phase: String,
    pub completed_phase_one_fields: Vec<String>,
    pub missing_phase_one_fields: Vec<String>,
    pub phase_transition_ready: bool,
}

impl IntakePhaseProgress {
    pub fn for_answers(answers: &IntakeInitialAnswers, question_set_version: i32) -> Self {
        let phase_one_fields = if question_set_version >= 3 {
            vec![
                ("basic_information", answers.basic_information.as_ref()),
                ("last_seen", answers.last_seen.as_ref()),
                ("suspicious_motive", answers.suspicious_motive.as_ref()),
                (
                    "police_report_status",
                    answers.police_report_status.as_ref(),
                ),
                ("family_phone", answers.family_phone.as_ref()),
            ]
        } else {
            vec![
                ("basic_information", answers.basic_information.as_ref()),
                ("last_seen", answers.last_seen.as_ref()),
            ]
        };
        let completed_phase_one_fields = phase_one_fields
            .iter()
            .filter_map(|(field, value)| value.as_ref().map(|_| (*field).to_owned()))
            .collect();
        let missing_phase_one_fields = phase_one_fields
            .iter()
            .filter(|(_, value)| value.is_none())
            .map(|(field, _)| (*field).to_owned())
            .collect::<Vec<_>>();
        let phase_transition_ready = missing_phase_one_fields.is_empty();
        Self {
            current_phase: if phase_transition_ready {
                "phase_two".to_owned()
            } else {
                "phase_one".to_owned()
            },
            completed_phase_one_fields,
            missing_phase_one_fields,
            phase_transition_ready,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCaseStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateElderProfileRequest {
    pub display_name: Option<String>,
    pub age: Option<i16>,
    pub gender: Option<String>,
    pub physical_description: Option<String>,
    pub clothing_description: Option<String>,
    pub health_notes: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_location: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateClueRequest {
    pub source: String,
    pub content: String,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub raw_record_reference: Option<String>,
    pub occurred_at: Option<String>,
    pub location_text: Option<String>,
    #[serde(default)]
    pub location_precision: Option<String>,
    #[serde(default)]
    pub next_action: Option<String>,
    #[serde(default)]
    pub linked_task_reference: Option<String>,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClueTimelineQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub source_type: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCasePlaceRequest {
    pub name: String,
    pub place_type: String,
    pub address: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub visibility: PlaceVisibility,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewCasePlaceRequest {
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceVisibility {
    Public,
    Confirmed,
    Internal,
}

impl PlaceVisibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Confirmed => "confirmed",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewClueRequest {
    pub status: String,
    pub reason: String,
    #[serde(default)]
    pub related_clue_id: Option<String>,
    #[serde(default)]
    pub relationship_type: Option<String>,
    #[serde(default)]
    pub next_action: Option<String>,
    #[serde(default)]
    pub linked_task_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTaskRequest {
    pub source_clue_id: String,
    #[serde(default)]
    pub volunteer_user_id: Option<String>,
    #[serde(default)]
    pub volunteer_user_ids: Vec<String>,
    pub title: String,
    pub objective: String,
    pub area_text: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub due_at: String,
    pub background: String,
    pub risk_level: String,
    pub risk_notes: String,
    pub safety_briefing: String,
    pub expected_feedback: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateTaskStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitTaskLocationReportRequest {
    pub source: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: f64,
    pub captured_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitTaskFeedbackRequest {
    pub content: String,
    #[serde(default)]
    pub occurred_at: Option<String>,
    #[serde(default)]
    pub location_text: Option<String>,
    #[serde(default)]
    pub location_precision: Option<String>,
    #[serde(default)]
    pub attachment_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTaskApplicationRequest {
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewTaskApplicationRequest {
    pub action: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddCaseMemberRequest {
    pub email: String,
    pub case_role: CaseRole,
}

#[derive(Debug, Serialize)]
pub struct CaseMemberResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub account_type: AccountType,
    pub global_capabilities: Vec<GlobalCapability>,
    pub case_role: CaseRole,
}

#[derive(Debug, Serialize)]
pub struct CommandIntakeCaseResponse {
    pub id: String,
    pub case_code: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub area_hint: Option<String>,
    pub elder_age: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct CaseListItem {
    pub id: String,
    pub case_code: String,
    pub status: String,
    pub access_role: CaseRole,
    pub display_name: String,
    pub last_seen_at: Option<String>,
    pub last_seen_location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CaseDetail {
    pub id: String,
    pub case_code: String,
    pub status: String,
    pub access_role: CaseRole,
    pub family_contact_emails: Vec<String>,
    pub elder_profile: ElderProfileResponse,
    pub clues: Vec<ClueResponse>,
    pub places: Vec<CasePlaceResponse>,
    pub attachments: Vec<CaseAttachmentResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ElderProfileResponse {
    pub id: String,
    pub display_name: String,
    pub age: Option<i16>,
    pub gender: Option<String>,
    pub physical_description: Option<String>,
    pub clothing_description: Option<String>,
    pub health_notes: Option<String>,
    pub last_seen_at: Option<String>,
    pub last_seen_location: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClueResponse {
    pub id: String,
    pub case_id: String,
    pub status: String,
    pub source: String,
    pub source_type: String,
    pub content: String,
    pub raw_record_reference: Option<String>,
    pub occurred_at: Option<String>,
    pub reported_at: String,
    pub confirmed_at: Option<String>,
    pub location_text: Option<String>,
    pub location_precision: Option<String>,
    pub next_action: Option<String>,
    pub linked_task_reference: Option<String>,
    pub related_clue_id: Option<String>,
    pub relationship_type: Option<String>,
    pub review_reason: Option<String>,
    pub attachment_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub reviewed_at: Option<String>,
    pub is_own_submission: bool,
}

#[derive(Debug, Serialize)]
pub struct ClueTimelineResponse {
    pub items: Vec<ClueResponse>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub case_id: String,
    pub source_clue_id: Option<String>,
    pub title: String,
    pub objective: String,
    pub area_text: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub due_at: String,
    pub background: Option<String>,
    pub risk_level: String,
    pub risk_notes: String,
    pub safety_briefing: String,
    pub expected_feedback: String,
    pub status: String,
    pub result_summary: Option<String>,
    pub assigned_volunteer_user_id: Option<String>,
    pub assigned_at: Option<String>,
    pub collaborators: Vec<TaskCollaboratorResponse>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskCollaboratorResponse {
    pub volunteer_user_id: String,
    pub assigned_by_user_id: String,
    pub assigned_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskApplicationResponse {
    pub id: String,
    pub task_id: String,
    pub volunteer_user_id: String,
    pub status: String,
    pub note: Option<String>,
    pub reviewed_by_user_id: Option<String>,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskCollaborationLocationResponse {
    pub volunteer_user_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: f64,
    pub captured_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub items: Vec<TaskResponse>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct CaseMapViewResponse {
    pub items: Vec<CaseMapItem>,
}

#[derive(Debug, Serialize)]
pub struct CaseMapItem {
    pub id: String,
    pub object_type: String,
    pub display_name: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub location_text: Option<String>,
    pub location_precision: String,
    pub source: String,
    pub occurred_at: Option<String>,
    pub reported_at: Option<String>,
    pub review_status: String,
    pub related_task_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CaseSummaryResponse {
    pub case_id: String,
    pub access_role: CaseRole,
    pub generated_at: String,
    pub source_scope: Vec<String>,
    pub last_confirmed_information: Option<CaseSummaryClue>,
    pub confirmed_clues: Vec<CaseSummaryClue>,
    pub pending_verification: Vec<CaseSummaryClue>,
    pub excluded_directions: Vec<CaseSummaryClue>,
    pub current_focus: Vec<CaseSummaryFocus>,
    pub task_status: Vec<CaseSummaryTask>,
    pub safety_reminders: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CaseSummaryClue {
    pub clue_id: String,
    pub content: String,
    pub status: String,
    pub occurred_at: Option<String>,
    pub reported_at: String,
}

#[derive(Debug, Serialize)]
pub struct CaseSummaryFocus {
    pub task_id: String,
    pub title: String,
    pub objective: String,
    pub area_text: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CaseSummaryTask {
    pub task_id: String,
    pub title: String,
    pub objective: String,
    pub area_text: String,
    pub due_at: String,
    pub status: String,
    pub safety_briefing: String,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CreateClueDraftRequest {
    pub source_record_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewIntakeProfileDraftRequest {
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreIntakeProfileDraftRequest {
    pub draft_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct IntakeProfileDraftVersionsResponse {
    pub items: Vec<IntakeProfileDraft>,
}

#[derive(Debug, Serialize)]
pub struct IntakeProfileDraftDiffResponse {
    pub from_version: i32,
    pub to_version: i32,
    pub changed_fields: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCaseSourceRecordRequest {
    pub record_type: String,
    pub content: String,
    pub occurred_at: Option<String>,
    pub source_reference: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CaseSourceRecordResponse {
    pub id: String,
    pub case_id: String,
    pub record_type: String,
    pub content: String,
    pub occurred_at: Option<String>,
    pub source_reference: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ClueDraftResponse {
    pub id: String,
    pub case_id: String,
    pub status: String,
    pub content: String,
    pub source_type: String,
    pub raw_record_reference: Option<String>,
    pub source_record_id: Option<String>,
    pub occurred_at: Option<String>,
    pub location_text: Option<String>,
    pub uncertainty_notice: String,
    pub template_version: String,
    pub provider_model: Option<String>,
    pub degradation_status: String,
    pub candidate: ClueDraftCandidate,
    pub review_status: String,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub version: i32,
    pub promoted_clue_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeAiFollowUp {
    pub field: String,
    pub prompt: String,
    pub purpose: String,
    pub missing_fields: Vec<String>,
    pub skippable: bool,
}

#[derive(Debug, Serialize)]
pub struct IntakeAiFollowUpResponse {
    pub question: Option<IntakeAiFollowUp>,
    pub degradation_status: String,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct IntakeAnswerRevisionResponse {
    pub id: String,
    pub field: String,
    pub answer: String,
    pub revision_kind: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreIntakeAnswerRequest {
    pub revision_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClueDraftCandidate {
    pub content_summary: Option<String>,
    pub occurred_at: Option<String>,
    pub location_text: Option<String>,
    pub source_text: Option<String>,
    pub action_candidates: Vec<String>,
    pub missing_fields: Vec<String>,
    pub source_excerpt: String,
    #[serde(default)]
    pub field_sources: std::collections::BTreeMap<String, ClueDraftFieldSource>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClueDraftFieldSource {
    pub reference: Option<String>,
    pub excerpt: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewClueDraftRequest {
    pub action: String,
    pub reason: String,
    pub candidate: ClueDraftCandidate,
    #[serde(default)]
    pub field_decisions: std::collections::BTreeMap<String, ClueDraftFieldDecision>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClueDraftFieldDecision {
    pub action: String,
    pub value: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CasePublicProgressResponse {
    pub case_id: String,
    pub status: String,
    pub publication_status: String,
    pub generated_at: String,
    pub confirmed_progress: Vec<CasePublicProgressItem>,
    pub requested_family_information: Vec<CasePublicProgressItem>,
    pub safety_and_contact_reminders: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CasePublicProgressItem {
    pub clue_id: String,
    pub progress_type: String,
    pub review_status: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CreateSummaryDraftRequest {
    pub content: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSummaryDraftRequest {
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct SummaryDraftResponse {
    pub id: String,
    pub case_id: String,
    pub parent_draft_id: Option<String>,
    pub version: i32,
    pub status: String,
    pub content: String,
    pub source_scope: Vec<String>,
    pub template_version: String,
    pub provider_model: Option<String>,
    pub generated_at: String,
    pub reviewed_at: Option<String>,
    pub review_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub publication_eligible: bool,
}

#[derive(Debug, Serialize)]
pub struct SummaryDraftVersionResponse {
    pub items: Vec<SummaryDraftResponse>,
}

#[derive(Debug, Serialize)]
pub struct PublishedSummaryVersionResponse {
    pub items: Vec<PublishedSummaryVersion>,
}

#[derive(Debug, Serialize)]
pub struct PublishedSummaryVersion {
    pub version: i32,
    pub content: String,
    pub published_at: String,
}

#[derive(Debug, Serialize)]
pub struct SummaryDraftDiffResponse {
    pub from_version: i32,
    pub to_version: i32,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ArchiveDraftResponse {
    pub id: String,
    pub case_id: String,
    pub status: String,
    pub content: String,
    pub source_scope: Vec<String>,
    pub review_material_id: Option<String>,
    pub deidentification_status: String,
    pub template_version: String,
    pub provider_model: Option<String>,
    pub version: i32,
    pub usage_scope: String,
    pub retention_status: String,
    pub deidentified_at: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CasePoiQuery {
    pub category: Option<String>,
    /// Optional browser geolocation in WGS-84. It is used only for this
    /// request and is converted server-side before calling AMap.
    pub browser_longitude: Option<f64>,
    pub browser_latitude: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct CasePoiResponse {
    pub items: Vec<CasePoiItem>,
    pub center_source: String,
    pub source: String,
    pub degradation_status: String,
    pub fallback_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CasePoiItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub address: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub distance_meters: Option<u64>,
    /// Short-lived, user- and case-bound capability required to request a
    /// route to this POI. It is not persisted.
    pub selection_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CasePoiRouteQuery {
    /// Browser geolocation is WGS-84 and only exists for the duration of this request.
    pub browser_longitude: f64,
    pub browser_latitude: f64,
    /// A short-lived capability issued with the selected POI by list_case_pois.
    pub selection_token: String,
}

#[derive(Debug, Serialize)]
pub struct CasePoiRouteResponse {
    pub straight_line_meters: u64,
    pub walking_distance_meters: Option<u64>,
    pub walking_duration_seconds: Option<u64>,
    pub source: String,
    pub degradation_status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskLocationReportReceipt {
    pub id: String,
    pub source: String,
    pub captured_at: String,
    pub retention_expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskFeedbackReceipt {
    pub task_id: String,
    pub clue_id: String,
    pub status: String,
    pub submitted_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskSafetyBriefingResponse {
    pub task_id: String,
    pub risk_level: String,
    pub notices: Vec<String>,
    pub emergency_stop_message: String,
    pub source: String,
    pub degradation_status: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskNavigationResponse {
    pub task_id: String,
    pub area_text: String,
    pub navigation_url: Option<String>,
    pub route_summary: String,
    pub source: String,
    pub degradation_status: String,
    pub fallback_message: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CasePlaceResponse {
    pub id: String,
    pub case_id: String,
    pub name: String,
    pub place_type: String,
    pub address: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub source: String,
    pub visibility: String,
    pub review_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_own_submission: bool,
}

#[derive(Debug, Serialize)]
pub struct CaseAttachmentResponse {
    pub id: String,
    pub case_id: String,
    pub original_filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub source: String,
    pub review_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_own_submission: bool,
}

#[derive(Debug, Serialize)]
pub struct CaseResourceConfigurationResponse {
    pub attachment_max_image_bytes: usize,
    pub attachment_max_per_case: u64,
    pub case_place_types: Vec<String>,
}

impl From<elder_profiles::Model> for ElderProfileResponse {
    fn from(model: elder_profiles::Model) -> Self {
        Self {
            id: model.id,
            display_name: model.display_name,
            age: model.age,
            gender: model.gender,
            physical_description: model.physical_description,
            clothing_description: model.clothing_description,
            health_notes: model.health_notes,
            last_seen_at: model.last_seen_at,
            last_seen_location: model.last_seen_location,
        }
    }
}

impl ClueResponse {
    pub fn new(
        model: clues::Model,
        attribution: Option<clue_attributions::Model>,
        viewer_user_id: &str,
        attachment_ids: Vec<String>,
    ) -> Self {
        Self {
            id: model.id,
            case_id: model.case_id,
            status: model.status,
            source: model.source,
            source_type: model.source_type,
            content: model.content,
            raw_record_reference: model.raw_record_reference,
            occurred_at: model.occurred_at,
            reported_at: model.reported_at,
            confirmed_at: model.confirmed_at,
            location_text: model.location_text,
            location_precision: model.location_precision,
            next_action: model.next_action,
            linked_task_reference: model.linked_task_reference,
            related_clue_id: model.related_clue_id,
            relationship_type: model.relationship_type,
            review_reason: model.review_reason,
            attachment_ids,
            created_at: model.created_at,
            updated_at: model.updated_at,
            reviewed_at: attribution
                .as_ref()
                .and_then(|attribution| attribution.reviewed_at.clone()),
            is_own_submission: attribution
                .and_then(|attribution| attribution.submitted_by_user_id)
                .is_some_and(|user_id| user_id == viewer_user_id),
        }
    }
}

impl TaskResponse {
    pub fn new(
        model: tasks::Model,
        assignments: Vec<task_assignments::Model>,
        include_assignee: bool,
    ) -> Self {
        let first_assignment = assignments.first();
        Self {
            id: model.id,
            case_id: model.case_id,
            source_clue_id: include_assignee.then_some(model.source_clue_id).flatten(),
            title: model.title,
            objective: model.objective,
            area_text: model.area_text,
            latitude: model.latitude,
            longitude: model.longitude,
            due_at: model.due_at,
            background: include_assignee.then_some(model.background),
            risk_level: model.risk_level,
            risk_notes: model.risk_notes,
            safety_briefing: model.safety_briefing,
            expected_feedback: model.expected_feedback,
            status: model.status,
            result_summary: model.result_summary,
            assigned_volunteer_user_id: include_assignee
                .then(|| first_assignment.map(|value| value.volunteer_user_id.clone()))
                .flatten(),
            assigned_at: first_assignment.map(|value| value.assigned_at.clone()),
            collaborators: if include_assignee {
                assignments
                    .iter()
                    .map(|assignment| TaskCollaboratorResponse {
                        volunteer_user_id: assignment.volunteer_user_id.clone(),
                        assigned_by_user_id: assignment.assigned_by_user_id.clone(),
                        assigned_at: assignment.assigned_at.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl From<task_applications::Model> for TaskApplicationResponse {
    fn from(model: task_applications::Model) -> Self {
        Self {
            id: model.id,
            task_id: model.task_id,
            volunteer_user_id: model.volunteer_user_id,
            status: model.status,
            note: model.note,
            reviewed_by_user_id: model.reviewed_by_user_id,
            reviewed_at: model.reviewed_at,
            review_reason: model.review_reason,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

impl CaseDetail {
    pub fn new(
        case_model: cases::Model,
        elder_profile: ElderProfileResponse,
        clues: Vec<ClueResponse>,
        places: Vec<CasePlaceResponse>,
        attachments: Vec<CaseAttachmentResponse>,
        access_role: CaseRole,
        family_contact_emails: Vec<String>,
    ) -> Self {
        Self {
            id: case_model.id,
            case_code: case_model.case_code,
            status: case_model.status,
            access_role,
            family_contact_emails,
            elder_profile,
            clues,
            places,
            attachments,
            created_at: case_model.created_at,
            updated_at: case_model.updated_at,
        }
    }
}
