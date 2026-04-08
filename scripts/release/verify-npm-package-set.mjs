import path from "node:path";
import { writeJson } from "./release-helpers.mjs";
import {
  formatVerificationSummary,
  parseArgs,
  verifyPackageSet,
} from "./npm-package-set-helpers.mjs";

const args = parseArgs(process.argv.slice(2));
const report = verifyPackageSet({
  gitTag: args["git-tag"],
  requireReleaseEvidence: Boolean(args["require-release-evidence"]),
  releaseEvidencePath: args["release-evidence"],
  stagedRoot: args["staged-root"] ?? args["staged-dist"],
  version: args.version,
});

const reportPath = path.resolve(
  process.cwd(),
  args.report ?? ".work/release/npm/package-set-verification.json",
);

writeJson(reportPath, report);
process.stdout.write(`${formatVerificationSummary(report)}\n`);
process.stdout.write(`Verification report: ${reportPath}\n`);

if (!report.validation.ok) {
  process.exit(1);
}
