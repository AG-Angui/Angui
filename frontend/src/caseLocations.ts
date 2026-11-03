import type { CaseTask, Clue } from "./api/cases";

/** Read the shared entity first, with support for pre-migration API responses. */
export function getClueLocation(clue: Clue) {
  return clue.location ?? clue;
}

export function getTaskLocationText(task: CaseTask): string {
  return task.location ? task.location.location_text ?? "" : task.area_text;
}
