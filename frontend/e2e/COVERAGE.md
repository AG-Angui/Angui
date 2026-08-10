# Browser E2E coverage matrix

This suite complements, rather than duplicates, the focused Vitest tests in
`frontend/src`. Every test file that exercises real API state, authentication,
or server-side authorization must have a corresponding browser workflow.

| Frontend test file(s) | Browser workflow | Status |
| --- | --- | --- |
| `LoginPage`, `AuthContext`, `App`, `api/client` | Login failure, session creation and revocation, role-route guards, API proxy | Implemented |
| `api/cases`, `DashboardPage`, `CaseWorkspacePage` | Case creation and membership visibility, a family browser clue submission, commander review, and role-filtered workspaces | Implemented |
| `FamilyIntakeForm` | Intake session, required answers, AI-review acknowledgement, explicit double confirmation, and browser-visible case creation | Implemented |
| `LearningCenterPage`, `LearningGovernancePage` | Independently governed publication, learner visibility, and withdrawal-driven access revocation | Implemented |
| `ProfilePage` | Profile read and browser update persistence after refresh | Implemented |
| `VolunteerWorkspacePage` | Volunteer browser task claim and persisted commander-visible application | Implemented |
| `LocationConfirmationPicker` | Not applicable: browser geolocation and AMap loader/conversion are external-SDK failure modes and remain isolated unit tests | Unit only |
| `AiReviewProgress` | Not applicable: pure presentation of already-authorized API state | Unit only |

The browser suite always uses an isolated SQLite database and only `.invalid`
demo identities. It must not use real case records, real locations, or external
AI and map credentials.
