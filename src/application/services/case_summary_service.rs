use chrono::{SecondsFormat, Utc};
use sea_orm::DatabaseConnection;

use crate::{
    error::ApiError,
    models::{
        AuthenticatedUser, CaseSummaryClue, CaseSummaryFocus, CaseSummaryResponse, CaseSummaryTask,
        ClueResponse, TaskResponse,
    },
    roles::CaseRole,
    services::{case_service, task_service},
};

const EXCLUDED_CLUE_STATUSES: &[&str] = &[
    "rejected",
    "expired",
    "duplicate",
    "conflicting",
    "insufficient_information",
];

pub async fn get_case_summary(
    db: &DatabaseConnection,
    auth: &AuthenticatedUser,
    case_id: &str,
) -> Result<CaseSummaryResponse, ApiError> {
    let detail = case_service::get_case(db, auth, case_id).await?;
    let task_status = task_service::list_all_visible_tasks(db, auth, case_id).await?;
    let mut confirmed_clues = detail
        .clues
        .iter()
        .filter(|clue| clue.status == "confirmed")
        .map(summary_clue)
        .collect::<Vec<_>>();
    confirmed_clues.sort_by(|left, right| {
        right
            .reported_at
            .cmp(&left.reported_at)
            .then_with(|| right.clue_id.cmp(&left.clue_id))
    });
    let last_confirmed_information = confirmed_clues.first().cloned();

    let pending_verification = if detail.access_role == CaseRole::Volunteer {
        Vec::new()
    } else {
        detail
            .clues
            .iter()
            .filter(|clue| {
                matches!(
                    clue.status.as_str(),
                    "pending_review" | "needs_verification"
                )
            })
            .map(summary_clue)
            .collect()
    };
    let excluded_directions = if detail.access_role == CaseRole::Commander {
        detail
            .clues
            .iter()
            .filter(|clue| EXCLUDED_CLUE_STATUSES.contains(&clue.status.as_str()))
            .map(summary_clue)
            .collect()
    } else {
        Vec::new()
    };
    let current_focus = if detail.access_role == CaseRole::Commander {
        task_status
            .iter()
            .filter(|task| !matches!(task.status.as_str(), "completed" | "cancelled"))
            .map(summary_focus)
            .collect()
    } else {
        Vec::new()
    };

    Ok(CaseSummaryResponse {
        case_id: detail.id,
        access_role: detail.access_role,
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        source_scope: source_scope(detail.access_role),
        last_confirmed_information,
        confirmed_clues,
        pending_verification,
        excluded_directions,
        current_focus,
        task_status: task_status.into_iter().map(summary_task).collect(),
        safety_reminders: safety_reminders(detail.access_role),
    })
}

fn summary_clue(clue: &ClueResponse) -> CaseSummaryClue {
    CaseSummaryClue {
        clue_id: clue.id.clone(),
        content: clue.content.clone(),
        status: clue.status.clone(),
        occurred_at: clue.occurred_at.clone(),
        reported_at: clue.reported_at.clone(),
    }
}

fn summary_focus(task: &TaskResponse) -> CaseSummaryFocus {
    CaseSummaryFocus {
        task_id: task.id.clone(),
        title: task.title.clone(),
        objective: task.objective.clone(),
        area_text: task.area_text.clone(),
        status: task.status.clone(),
    }
}

fn summary_task(task: TaskResponse) -> CaseSummaryTask {
    CaseSummaryTask {
        task_id: task.id,
        title: task.title,
        objective: task.objective,
        area_text: task.area_text,
        due_at: task.due_at,
        status: task.status,
        safety_briefing: task.safety_briefing,
    }
}

fn source_scope(role: CaseRole) -> Vec<String> {
    match role {
        CaseRole::Commander => vec![
            "confirmed_clues".to_owned(),
            "unverified_clues".to_owned(),
            "excluded_clues".to_owned(),
            "all_case_tasks".to_owned(),
        ],
        CaseRole::Family => vec![
            "confirmed_clues".to_owned(),
            "own_unverified_clues".to_owned(),
        ],
        CaseRole::Volunteer => vec![
            "confirmed_clues".to_owned(),
            "authorized_case_tasks".to_owned(),
        ],
    }
}

fn safety_reminders(role: CaseRole) -> Vec<String> {
    match role {
        CaseRole::Commander => vec![
            "Only human-reviewed clues marked confirmed are confirmed facts.".to_owned(),
            "Keep task assignments and search directions within authorized case roles.".to_owned(),
        ],
        CaseRole::Family => vec![
            "Only human-reviewed clues marked confirmed are confirmed facts.".to_owned(),
            "Do not share sensitive case information publicly.".to_owned(),
        ],
        CaseRole::Volunteer => vec![
            "Follow the safety briefing for each assigned task.".to_owned(),
            "Stop work and notify the commander if conditions become unsafe.".to_owned(),
        ],
    }
}
