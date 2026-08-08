import { spawn } from "node:child_process";
import { StringDecoder } from "node:string_decoder";

const DEFAULT_MAX_OUTPUT_BYTES = 8 * 1024 * 1024;
const DEFAULT_MAX_LINE_BYTES = 64 * 1024;
const DEFAULT_TERMINATION_GRACE_MS = 1_000;
const STDERR_PREVIEW_BYTES = 4 * 1024;

function positiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${label} must be a positive safe integer`);
  }
  return value;
}

function boundedStderrPreview(current, chunk) {
  const remaining = STDERR_PREVIEW_BYTES - Buffer.byteLength(current, "utf8");
  if (remaining <= 0) return current;
  return `${current}${chunk.subarray(0, remaining).toString("utf8")}`;
}

export function androidLogcatContains({
  adb,
  serial,
  pattern,
  timeoutMs,
  maxOutputBytes = DEFAULT_MAX_OUTPUT_BYTES,
  maxLineBytes = DEFAULT_MAX_LINE_BYTES,
  terminationGraceMs = DEFAULT_TERMINATION_GRACE_MS,
  spawnProcess = spawn,
}) {
  if (typeof adb !== "string" || adb.length === 0) {
    throw new Error("Android logcat observation requires an adb executable");
  }
  if (typeof serial !== "string" || serial.length === 0) {
    throw new Error("Android logcat observation requires a device serial");
  }
  if (!(pattern instanceof RegExp)) {
    throw new Error("Android logcat observation requires a RegExp pattern");
  }
  const boundedTimeoutMs = positiveInteger(timeoutMs, "Android logcat timeout");
  const boundedOutputBytes = positiveInteger(
    maxOutputBytes,
    "Android logcat output limit",
  );
  const boundedLineBytes = positiveInteger(maxLineBytes, "Android logcat line limit");
  const boundedTerminationGraceMs = positiveInteger(
    terminationGraceMs,
    "Android logcat termination grace",
  );
  if (boundedLineBytes > boundedOutputBytes) {
    throw new Error("Android logcat line limit cannot exceed the output limit");
  }
  const matcher = new RegExp(pattern.source, pattern.flags.replace(/[gy]/g, ""));

  return new Promise((resolve, reject) => {
    let child;
    try {
      child = spawnProcess(
        adb,
        ["-s", serial, "logcat", "-d", "-v", "raw"],
        { stdio: ["ignore", "pipe", "pipe"], windowsHide: true },
      );
    } catch (error) {
      reject(new Error(`Android logcat observation failed to start: ${error.message}`));
      return;
    }

    const decoder = new StringDecoder("utf8");
    let observedBytes = 0;
    let lineBuffer = "";
    let stderrPreview = "";
    let matched = false;
    let failure = null;
    let settled = false;
    let timer;
    let terminationTimer;

    const settle = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      clearTimeout(terminationTimer);
      if (error) reject(error);
      else resolve(value);
    };
    const stopWith = (error) => {
      if (failure) return;
      failure = error;
      try {
        if (!child.kill("SIGKILL")) {
          failure = new Error(`${error.message}; child termination was not accepted`);
        }
      } catch (killError) {
        failure = new Error(`${error.message}; child termination failed: ${killError.message}`);
      }
      terminationTimer = setTimeout(() => {
        settle(new Error(
          `${failure.message}; child did not close within ${boundedTerminationGraceMs} ms`,
        ));
      }, boundedTerminationGraceMs);
    };
    const inspectCompleteLines = () => {
      let newline;
      while ((newline = lineBuffer.indexOf("\n")) >= 0) {
        const line = lineBuffer.slice(0, newline);
        lineBuffer = lineBuffer.slice(newline + 1);
        if (Buffer.byteLength(line, "utf8") > boundedLineBytes) {
          stopWith(new Error(
            `Android logcat observation line exceeded the ${boundedLineBytes} byte limit`,
          ));
          return;
        }
        if (matcher.test(line)) matched = true;
      }
      if (Buffer.byteLength(lineBuffer, "utf8") > boundedLineBytes) {
        stopWith(new Error(
          `Android logcat observation line exceeded the ${boundedLineBytes} byte limit`,
        ));
      }
    };
    const observeBytes = (chunk) => {
      observedBytes += chunk.length;
      if (observedBytes > boundedOutputBytes) {
        stopWith(new Error(
          `Android logcat observation exceeded the ${boundedOutputBytes} byte output limit`,
        ));
        return false;
      }
      return true;
    };

    timer = setTimeout(() => {
      stopWith(new Error(
        `Android logcat observation timed out after ${boundedTimeoutMs} ms`,
      ));
    }, boundedTimeoutMs);

    child.stdout.on("data", (chunk) => {
      if (!observeBytes(chunk) || failure) return;
      lineBuffer += decoder.write(chunk).replaceAll("\r", "");
      inspectCompleteLines();
    });
    child.stderr.on("data", (chunk) => {
      if (!observeBytes(chunk)) return;
      stderrPreview = boundedStderrPreview(stderrPreview, chunk).replaceAll("\r", "");
    });
    child.once("error", (error) => {
      settle(failure ?? new Error(
        `Android logcat observation failed to start: ${error.message}`,
      ));
    });
    child.once("close", (code, signal) => {
      if (!failure) {
        lineBuffer += decoder.end().replaceAll("\r", "");
        if (lineBuffer.length > 0 && matcher.test(lineBuffer)) matched = true;
        if (Buffer.byteLength(lineBuffer, "utf8") > boundedLineBytes) {
          failure = new Error(
            `Android logcat observation line exceeded the ${boundedLineBytes} byte limit`,
          );
        }
      }
      if (failure) {
        settle(failure);
        return;
      }
      if (code !== 0) {
        const status = code == null ? `signal ${signal ?? "unknown"}` : `status ${code}`;
        const detail = stderrPreview.trim();
        settle(new Error(
          `Android logcat observation exited with ${status}${detail ? `: ${detail}` : ""}`,
        ));
        return;
      }
      settle(null, matched);
    });
  });
}
