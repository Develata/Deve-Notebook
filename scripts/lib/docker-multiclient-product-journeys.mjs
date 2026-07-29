export {
  assertNoScope,
  createFirstRepoFromNoScope,
  exerciseLastRepoNoScope,
  exerciseRepoLifecycle,
  restartCandidateContainer,
} from "./docker-multiclient-repo-lifecycle.mjs";
export {
  exerciseSourceControlAndExternalChanges,
} from "./docker-multiclient-source-control.mjs";
export {
  assertRemovalPreservation,
  selectWorkspaceRoot,
  validateWorkspaceIdentity,
} from "./docker-multiclient-workspace.mjs";
