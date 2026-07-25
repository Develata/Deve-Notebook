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
