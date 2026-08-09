# Browser E2E coverage matrix

This suite complements, rather than duplicates, the focused Vitest tests in
`frontend/src`. Every test file that exercises real API state, authentication,
or server-side authorization must have a corresponding browser workflow.

| Frontend test file(s) | Browser workflow | Status |
| --- | --- | --- |
| `LoginPage`, `AuthContext`, `App`, `api/client` | Login failure, session creation and revocation, role-route guards, API proxy | Implemented |
| `api/cases`, `DashboardPage`, `CaseWorkspacePage` | Case creation and membership visibility, clue review, case state transitions, role-filtered workspaces | Planned |
| `FamilyIntakeForm` | Intake session, answers and correction, explicit double confirmation, case creation | Planned |
| `LearningCenterPage`, `LearningGovernancePage` | Published learning visibility and administrator governance lifecycle | Planned |
| `ProfilePage` | Profile read and update persistence after refresh | Planned |
| `VolunteerWorkspacePage` | Task claim, authorized status progression, location/feedback submission | Planned |
| `LocationConfirmationPicker` | Not applicable: browser geolocation and AMap loader/conversion are external-SDK failure modes and remain isolated unit tests | Unit only |
| `AiReviewProgress` | Not applicable: pure presentation of already-authorized API state | Unit only |

The browser suite always uses an isolated SQLite database and only `.invalid`
demo identities. It must not use real case records, real locations, or external
AI and map credentials.
