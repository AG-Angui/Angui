use serde::{Deserialize, Serialize};

use crate::entities::{cases, clues, elder_profiles};

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

#[derive(Debug, Serialize)]
pub struct CaseListItem {
    pub id: String,
    pub case_code: String,
    pub status: String,
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

impl From<clues::Model> for ClueResponse {
    fn from(model: clues::Model) -> Self {
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
        }
    }
}

impl CaseDetail {
    pub fn new(
        case_model: cases::Model,
        elder_profile: elder_profiles::Model,
        clue_models: Vec<clues::Model>,
    ) -> Self {
        Self {
            id: case_model.id,
            case_code: case_model.case_code,
            status: case_model.status,
            elder_profile: elder_profile.into(),
            clues: clue_models.into_iter().map(Into::into).collect(),
            created_at: case_model.created_at,
            updated_at: case_model.updated_at,
        }
    }
}
