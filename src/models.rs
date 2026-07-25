use serde::{Deserialize, Serialize};

use crate::{
    entities::{
        cases, clue_attributions, clues, elder_profiles, intake_session_answers, intake_sessions,
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
    pub missing_fields: Vec<String>,
    pub assessments: Vec<IntakeAssessment>,
    pub confirmation_blocked_reasons: Vec<String>,
    pub direction_hypotheses: Vec<IntakeDirectionHypothesis>,
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
pub struct CreateClueRequest {
    pub source: String,
    pub content: String,
    pub occurred_at: Option<String>,
    pub location_text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewClueRequest {
    pub status: String,
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
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
    pub content: String,
    pub occurred_at: Option<String>,
    pub location_text: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub reviewed_at: Option<String>,
    pub is_own_submission: bool,
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
    ) -> Self {
        Self {
            id: model.id,
            case_id: model.case_id,
            status: model.status,
            source: model.source,
            content: model.content,
            occurred_at: model.occurred_at,
            location_text: model.location_text,
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

impl CaseDetail {
    pub fn new(
        case_model: cases::Model,
        elder_profile: ElderProfileResponse,
        clues: Vec<ClueResponse>,
        access_role: CaseRole,
    ) -> Self {
        Self {
            id: case_model.id,
            case_code: case_model.case_code,
            status: case_model.status,
            access_role,
            elder_profile,
            clues,
            created_at: case_model.created_at,
            updated_at: case_model.updated_at,
        }
    }
}
