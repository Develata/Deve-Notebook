const ANDROID_APP_ID_PATTERN = /^[A-Za-z][A-Za-z0-9_]*(\.[A-Za-z][A-Za-z0-9_]*)+$/;
const ANDROID_COMPONENT_PATTERN =
  /(?:^|[\s{])([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+)\/([A-Za-z0-9_.$]+)(?=[\s}]|$)/g;

const SOFT_UNAVAILABLE = "soft-unavailable";
const HARD_INVALID = "hard-invalid";

function projectClassifierState(state) {
  return state === SOFT_UNAVAILABLE || state === HARD_INVALID ? "unavailable" : state;
}

function classifyComponentNamesDetailed(names, appId, matchedState, otherState) {
  const components = names.map((name) => {
    const matches = [...name.matchAll(ANDROID_COMPONENT_PATTERN)];
    return matches.length === 1 ? `${matches[0][1]}/${matches[0][2]}` : null;
  });
  if (components.length === 0 || components.some((component) => component === null)) {
    return HARD_INVALID;
  }
  const uniqueComponents = new Set(components);
  if (uniqueComponents.size !== 1) return HARD_INVALID;
  const [component] = uniqueComponents;
  const packageName = component.slice(0, component.indexOf("/"));
  if (packageName === appId) return matchedState;
  if (packageName.startsWith(`${appId}.`) || appId.startsWith(`${packageName}.`)) {
    return HARD_INVALID;
  }
  return otherState;
}

function classifyExactPackageComponentDetailed(
  records,
  appId,
  containerPattern,
  matchedState,
  otherState,
) {
  const names = records.map((line) => {
    const containers = [...line.matchAll(containerPattern)];
    return containers.length === 1 ? containers[0][1] : null;
  });
  if (names.some((name) => name === null)) return HARD_INVALID;
  return classifyComponentNamesDetailed(names, appId, matchedState, otherState);
}

function classifyExactPackageComponent(
  records,
  appId,
  containerPattern,
  matchedState,
  otherState,
) {
  return projectClassifierState(classifyExactPackageComponentDetailed(
    records,
    appId,
    containerPattern,
    matchedState,
    otherState,
  ));
}

export function classifyAndroidActivityResumed(output, appId) {
  if (typeof output !== "string"
    || typeof appId !== "string"
    || !ANDROID_APP_ID_PATTERN.test(appId)) {
    return "unavailable";
  }
  const records = output.replaceAll("\r", "").split("\n").filter((line) => {
    const normalized = line.trim();
    return /^(?:mResumedActivity|topResumedActivity)\s*[:=]/.test(normalized)
      || /^ResumedActivity\s*:/.test(normalized);
  });
  if (records.length === 0) return "unavailable";
  return classifyExactPackageComponent(
    records,
    appId,
    /ActivityRecord\{([^{}]+)\}/g,
    "resumed",
    "not-resumed",
  );
}

function classifyAndroidWindowFocusDetailed(output, appId) {
  if (typeof appId !== "string" || !ANDROID_APP_ID_PATTERN.test(appId)) return HARD_INVALID;
  if (typeof output !== "string") return SOFT_UNAVAILABLE;
  const authorityRecords = output.replaceAll("\r", "").split("\n").filter((line) =>
    /^mCurrentFocus\b/.test(line.trim()));
  if (authorityRecords.length === 0) return SOFT_UNAVAILABLE;
  if (authorityRecords.some((line) => !/^mCurrentFocus\s*[:=]/.test(line.trim()))) {
    return HARD_INVALID;
  }
  const records = authorityRecords;
  const nullCount = records.filter((line) =>
    /^mCurrentFocus\s*[:=]\s*null\s*$/.test(line.trim())).length;
  if (nullCount === records.length) return SOFT_UNAVAILABLE;
  if (nullCount !== 0) return HARD_INVALID;
  return classifyExactPackageComponentDetailed(
    records,
    appId,
    /Window\{([^{}]+)\}/g,
    "focused",
    "not-focused",
  );
}

export function classifyAndroidWindowFocused(output, appId) {
  return projectClassifierState(classifyAndroidWindowFocusDetailed(output, appId));
}

function readModernInputDispatcherFocus(lines) {
  const focusedDisplayRecords = lines.flatMap((line) => {
    const normalized = line.trim();
    if (!normalized.startsWith("FocusedDisplayId")) return [];
    const match = normalized.match(/^FocusedDisplayId\s*:\s*([0-9]+)$/);
    return [{ valid: Boolean(match), value: match ? Number(match[1]) : null }];
  });
  if (focusedDisplayRecords.some(({ valid, value }) => !valid || !Number.isSafeInteger(value))
    || focusedDisplayRecords.length > 1) {
    return { kind: "invalid" };
  }
  const focusedWindowsAuthorityRecords = lines.filter((line) =>
    /^FocusedWindows\b/.test(line.trim()));
  if (focusedWindowsAuthorityRecords.some((line) =>
    !/^FocusedWindows\s*:\s*(?:<none>)?$/.test(line.trim()))) {
    return { kind: "invalid" };
  }
  const noneIndexes = lines.flatMap((line, index) =>
    /^FocusedWindows\s*:\s*<none>$/.test(line.trim()) ? [index] : []);
  const sectionIndexes = lines.flatMap((line, index) =>
    /^FocusedWindows\s*:\s*$/.test(line.trim()) ? [index] : []);
  if (noneIndexes.length > 1 || sectionIndexes.length > 1
    || (noneIndexes.length === 1 && sectionIndexes.length === 1)) {
    return { kind: "invalid" };
  }
  if (noneIndexes.length === 1) {
    for (let index = noneIndexes[0] + 1; index < lines.length; index += 1) {
      const normalized = lines[index].trim();
      if (normalized.length === 0) continue;
      if (/^displayId\b/.test(normalized)) return { kind: "invalid" };
      break;
    }
    return { kind: "unavailable" };
  }
  if (sectionIndexes.length === 0) {
    return focusedDisplayRecords.length === 0 ? { kind: "absent" } : { kind: "invalid" };
  }

  const records = [];
  for (let index = sectionIndexes[0] + 1; index < lines.length; index += 1) {
    const normalized = lines[index].trim();
    if (normalized.length === 0) continue;
    if (!normalized.startsWith("displayId=")) {
      if (/^displayId\b/.test(normalized)) return { kind: "invalid" };
      break;
    }
    const match = normalized.match(/^displayId=([0-9]+), name='([^'\r\n]+)'$/);
    if (!match) return { kind: "invalid" };
    const displayId = Number(match[1]);
    if (!Number.isSafeInteger(displayId)) return { kind: "invalid" };
    records.push({ displayId, name: match[2] });
  }
  if (records.length === 0) return { kind: "invalid" };

  if (focusedDisplayRecords.length === 0) {
    return records.length === 1
      ? { kind: "present", names: [records[0].name] }
      : { kind: "invalid" };
  }
  const selected = records.filter(({ displayId }) =>
    displayId === focusedDisplayRecords[0].value);
  return selected.length === 1
    ? { kind: "present", names: [selected[0].name] }
    : { kind: "invalid" };
}

function readLegacyInputDispatcherFocus(lines) {
  const records = lines.flatMap((line) => {
    const normalized = line.trim();
    if (!/^(?:FocusedWindow|focusedWindow)\b/.test(normalized)) return [];
    const current = normalized.match(/^FocusedWindow\s*:\s*name='([^'\r\n]+)'$/);
    const legacy = normalized.match(/^focusedWindow\s*:\s*'([^'\r\n]+)'$/);
    return [{ valid: Boolean(current || legacy), name: current?.[1] ?? legacy?.[1] ?? null }];
  });
  if (records.length === 0) return { kind: "absent" };
  if (records.length !== 1 || !records[0].valid) return { kind: "invalid" };
  if (records[0].name === "<null>") return { kind: "unavailable" };
  return { kind: "present", names: [records[0].name] };
}

function readCurrentInputDispatcherLines(lines) {
  const authorityHeaders = lines.filter((line) =>
    /^Input Dispatcher State\b/.test(line.trim()));
  if (authorityHeaders.some((line) => {
    const normalized = line.trim();
    return !/^Input Dispatcher State\s*:\s*$/.test(normalized)
      && !/^Input Dispatcher State at time of last ANR\s*:\s*$/.test(normalized);
  })) {
    return null;
  }
  const currentIndexes = lines.flatMap((line, index) =>
    /^Input Dispatcher State\s*:\s*$/.test(line.trim()) ? [index] : []);
  const historicalIndexes = lines.flatMap((line, index) =>
    /^Input Dispatcher State at time of last ANR\s*:\s*$/.test(line.trim()) ? [index] : []);
  if (currentIndexes.length > 1 || historicalIndexes.length > 1) return null;
  const historicalIndex = historicalIndexes[0] ?? -1;
  if (currentIndexes.length === 0) {
    return {
      lines: lines.slice(0, historicalIndex >= 0 ? historicalIndex : lines.length),
      historicalWithoutCurrent: historicalIndex >= 0,
    };
  }
  if (historicalIndex >= 0 && historicalIndex < currentIndexes[0]) return null;
  return {
    lines: lines.slice(
      currentIndexes[0] + 1,
      historicalIndex >= 0 ? historicalIndex : lines.length,
    ),
    historicalWithoutCurrent: false,
  };
}

function classifyAndroidInputDispatcherFocusDetailed(output, appId) {
  if (typeof appId !== "string" || !ANDROID_APP_ID_PATTERN.test(appId)) return HARD_INVALID;
  if (typeof output !== "string") return SOFT_UNAVAILABLE;
  const current = readCurrentInputDispatcherLines(output.replaceAll("\r", "").split("\n"));
  if (!current) return HARD_INVALID;
  const modern = readModernInputDispatcherFocus(current.lines);
  const legacy = readLegacyInputDispatcherFocus(current.lines);
  if (modern.kind === "invalid" || legacy.kind === "invalid") return HARD_INVALID;
  if (modern.kind !== "absent" && legacy.kind !== "absent") return HARD_INVALID;
  const selected = modern.kind !== "absent" ? modern : legacy;
  if (selected.kind === "unavailable") return SOFT_UNAVAILABLE;
  if (selected.kind !== "present") {
    return current.historicalWithoutCurrent ? HARD_INVALID : SOFT_UNAVAILABLE;
  }
  return classifyComponentNamesDetailed(selected.names, appId, "focused", "not-focused");
}

export function classifyAndroidInputDispatcherFocused(output, appId) {
  return projectClassifierState(classifyAndroidInputDispatcherFocusDetailed(output, appId));
}

function combineFocusStates(windowDetailedState, dispatcherDetailedState) {
  if (windowDetailedState === HARD_INVALID || dispatcherDetailedState === HARD_INVALID) {
    return "unavailable";
  }
  const windowState = projectClassifierState(windowDetailedState);
  const dispatcherState = projectClassifierState(dispatcherDetailedState);
  if (windowDetailedState === SOFT_UNAVAILABLE) return dispatcherState;
  if (dispatcherDetailedState === SOFT_UNAVAILABLE) return windowState;
  return windowState === dispatcherState ? windowState : "unavailable";
}

export function classifyAndroidNativeInputTargetObservation(
  activityOutput,
  windowOutput,
  dispatcherOutput,
  appId,
) {
  const activityState = classifyAndroidActivityResumed(activityOutput, appId);
  const windowDetailedState = classifyAndroidWindowFocusDetailed(windowOutput, appId);
  const dispatcherDetailedState = classifyAndroidInputDispatcherFocusDetailed(dispatcherOutput, appId);
  const windowState = projectClassifierState(windowDetailedState);
  const dispatcherState = projectClassifierState(dispatcherDetailedState);
  const focusState = combineFocusStates(windowDetailedState, dispatcherDetailedState);
  const nativeTargetState = activityState === "unavailable" || focusState === "unavailable"
    ? "unavailable"
    : activityState === "resumed" && focusState === "focused" ? "ready" : "not-ready";
  return { activityState, windowState, dispatcherState, focusState, nativeTargetState };
}

export function classifyAndroidNativeInputTarget(
  activityOutput,
  windowOutput,
  dispatcherOutput,
  appId,
) {
  return classifyAndroidNativeInputTargetObservation(
    activityOutput,
    windowOutput,
    dispatcherOutput,
    appId,
  ).nativeTargetState;
}
