import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

export function writeAndroidWritableEvidence({
  evidencePath,
  targetFactsPath,
  producer,
  mode,
  webcrypto,
  journey,
  repoLifecycle,
  recovery,
}) {
  if (!evidencePath) return;
  if (!targetFactsPath) throw new Error(`${producer} evidence requires target facts`);
  const evidence = {
    schema: 1,
    producer,
    mode,
    target: JSON.parse(readFileSync(targetFactsPath, "utf8")),
    webcrypto,
    journey,
  };
  if (repoLifecycle) evidence.repoLifecycle = repoLifecycle;
  if (recovery) evidence.recovery = recovery;
  mkdirSync(dirname(evidencePath), { recursive: true });
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
}

export function writeAndroidLocalBackendEvidence({
  evidencePath,
  targetFactsPath,
  identityCapability,
  firstRepo,
  drawerGestureProof,
  keyboardPresentation,
  testImeService,
  repoLifecycle,
  rootBackProof,
}) {
  writeAndroidWritableEvidence({
    evidencePath,
    targetFactsPath,
    producer: "smoke-mobile-android-lifecycle",
    mode: "local-backend",
    webcrypto: identityCapability,
    repoLifecycle,
    journey: {
      loginOrNativeSession: true,
      bootstrapUnbound: {
        syncStatus: firstRepo.initial.status,
        repoIdEmpty: firstRepo.initial.repoIdRaw === "",
        scopeNonce: firstRepo.initial.scopeNonce,
        defaultRepoAbsent: firstRepo.defaultRepoAbsent,
      },
      firstCreate: {
        writerReady: firstRepo.created.status === "ready",
        repoIdBound: Boolean(firstRepo.created.repoId),
        scopeNonce: firstRepo.created.scopeNonce,
        aliasCount: firstRepo.aliasCount,
      },
      edit: true,
      commitHistory: true,
      backgroundResume: true,
      staleScopeRejected: true,
      pendingPreserved: true,
      nativeSystemGestureInsetsAcceptedAfterReload: true,
      nativeDrawerGesturesAfterReload: drawerGestureProof.leftDrawerOpened
        && drawerGestureProof.rightDrawerOpened
        && drawerGestureProof.pidStable,
      imeBackPreservedEditorSession: true,
      imeRetapReopenedKeyboard: true,
      keyboardPresentationMode: keyboardPresentation.presented.keyboardMode,
      keyboardTestImeService: testImeService,
      repoRemovalNoScope: repoLifecycle.noScope,
      rootBackBackgroundsTaskWithStablePid: rootBackProof.rootBackBackgrounded
        && rootBackProof.pidStable
        && rootBackProof.reentryReady,
      writableLifecycleComplete: true,
    },
  });
}
