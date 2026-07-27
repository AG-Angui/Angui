use serde::{Deserialize, Serialize};

use crate::{
    entities::{
        cases, clue_attributions, clues, elder_profiles, intake_session_answers, intake_sessions,
        task_assignments, tasks,
    },
    roles::{AccountType, CaseRole, GlobalCapability},
};

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
pub struct LoginRequest {
    pub email: String,
    pub password: String,
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

#[derive(Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IntakeInitialAnswers {
    pub basic_information: Option<String>,
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
    pub privacy_notice: String,
    pub created_at: String,
    pub updated_at: String,
}

impl IntakeSessionResponse {
    pub fn new(
        model: intake_sessions::Model,
        initial_answers: IntakeInitialAnswers,
        missing_fields: Vec<String>,
        next_question: Option<IntakeQuestion>,
    ) -> Self {
        let phase = IntakePhaseProgress::for_answers(&initial_answers);
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
    pub status: String,
    pub source_scope: String,
    pub generated_at: String,
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
#[derive(Clone, Debug, Serialize)]
pub struct IntakeProfileDraftFieldMetadata {
    pub field: String,
    pub source_field: String,
    pub source: String,
    pub status: String,
    pub generated_at: String,
}

#[derive(Clone, Debug, Serialize)]
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
    pub fn for_answers(answers: &IntakeInitialAnswers) -> Self {
        let completed_phase_one_fields = [
            ("basic_information", answers.basic_information.as_ref()),
            ("health_status", answers.health_status.as_ref()),
            ("behavior_habits", answers.behavior_habits.as_ref()),
            ("last_seen", answers.last_seen.as_ref()),
        ]
        .into_iter()
        .filter_map(|(field, value)| value.as_ref().map(|_| field.to_owned()))
        .collect();
        let missing_phase_one_fields = [
            ("basic_information", answers.basic_information.is_none()),
            ("last_seen", answers.last_seen.is_none()),
        ]
        .into_iter()
        .filter(|(_, missing)| *missing)
        .map(|(field, _)| field.to_owned())
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
    pub volunteer_user_id: String,
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub items: Vec<TaskResponse>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct TaskLocationReportReceipt {
    pub id: String,
    pub source: String,
    pub captured_at: String,
    pub retention_expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TaskFeedbackReceipt {
    pub task_id: String,
    pub clue_id: String,
    pub status: String,
    pub submitted_at: String,
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
        assignment: Option<task_assignments::Model>,
        include_assignee: bool,
    ) -> Self {
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
                .then(|| {
                    assignment
                        .as_ref()
                        .map(|value| value.volunteer_user_id.clone())
                })
                .flatten(),
            assigned_at: assignment.map(|value| value.assigned_at),
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
    ) -> Self {
        Self {
            id: case_model.id,
            case_code: case_model.case_code,
            status: case_model.status,
            access_role,
            elder_profile,
            clues,
            places,
            attachments,
            created_at: case_model.created_at,
            updated_at: case_model.updated_at,
        }
    }
}
