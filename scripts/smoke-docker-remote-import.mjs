import {
  assertDiagnostics,
  chromium,
  login,
} from "./lib/docker-remote-import-runtime.mjs";
import {
  exerciseS3,
  exerciseWebDav,
  exerciseWebDavFailure,
} from "./lib/docker-remote-import-journeys.mjs";
import { closeBrowserResources } from "./lib/docker-remote-import-browser-cleanup.mjs";

async function run() {
  const scenario = process.argv[2];
  const scenarios = {
    "webdav-failure": {
      baseUrl: process.env.DEVE_REMOTE_IMPORT_WEBDAV_FAILURE_BASE_URL,
      diagnosticLabel: "remote-import-webdav-failure",
      exercise: exerciseWebDavFailure,
    },
    webdav: {
      baseUrl: process.env.DEVE_REMOTE_IMPORT_WEBDAV_BASE_URL,
      diagnosticLabel: "remote-import-webdav",
      exercise: exerciseWebDav,
    },
    s3: {
      baseUrl: process.env.DEVE_REMOTE_IMPORT_S3_BASE_URL,
      diagnosticLabel: "remote-import-s3",
      exercise: exerciseS3,
    },
  };
  if (!Object.hasOwn(scenarios, scenario)) {
    throw new Error(
      "expected exactly one Remote Import scenario: webdav-failure, webdav, or s3",
    );
  }
  const selected = scenarios[scenario];
  const browser = await chromium.launch({
    headless: !["0", "false", "no"].includes(
      (process.env.DEVE_REMOTE_IMPORT_HEADLESS ?? "1").toLowerCase(),
    ),
  });
  let context;
  let journeyError;
  try {
    context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
    });
    const page = await context.newPage();
    const diagnostics = await login(
      page,
      selected.baseUrl,
      selected.diagnosticLabel,
    );
    await selected.exercise(page, diagnostics);
    assertDiagnostics(diagnostics);
  } catch (error) {
    journeyError = error;
  } finally {
    try {
      await closeBrowserResources(context, browser);
    } catch (cleanupError) {
      if (journeyError) {
        throw new AggregateError(
          [journeyError, cleanupError],
          "Remote Import journey and browser cleanup both failed",
        );
      }
      throw cleanupError;
    }
  }
  if (journeyError) throw journeyError;
}

run().catch((error) => {
  console.error(error.stack || error.message);
  process.exitCode = 1;
});
