import path from "node:path";
import { spawnSync } from "node:child_process";
import { writeJson } from "./release-helpers.mjs";
import {
  parseArgs,
  verifyPackageSet,
} from "./npm-package-set-helpers.mjs";

const npmBin = process.platform === "win32" ? "npm.cmd" : "npm";
const npxBin = process.platform === "win32" ? "npx.cmd" : "npx";
const trustedPublishingNpmPackage =
  process.env.NPM_TRUSTED_PUBLISHING_CLI_PACKAGE ?? "npm@^11.5.1";

function resolveNpmInvocation(commandArgs, options) {
  if (options.mode === "publish" && process.env.GITHUB_ACTIONS === "true") {
    return {
      args: ["--yes", trustedPublishingNpmPackage, ...commandArgs],
      command: npxBin,
      displayCommand: `npx --yes ${trustedPublishingNpmPackage}`,
    };
  }

  return {
    args: commandArgs,
    command: npmBin,
    displayCommand: "npm",
  };
}

function runNpm(commandArgs, options) {
  const invocation = resolveNpmInvocation(commandArgs, options);
  const result = spawnSync(invocation.command, invocation.args, {
    cwd: options.cwd,
    encoding: "utf8",
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
  });

  if (result.stdout) {
    process.stdout.write(result.stdout);
  }

  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if ((result.status ?? 0) !== 0) {
    throw new Error(
      `${invocation.displayCommand} ${commandArgs.join(" ")} failed in ${options.cwd} with exit code ${result.status ?? 1}.`,
    );
  }

  return result.stdout?.trim() ?? "";
}

function packageSetPlan(report) {
  const platformPackages = report.stagedPackages.map((entry) => ({
    cwd: entry.packageDir,
    kind: "platform",
    manifestPath: entry.manifestPath,
    name: entry.name,
    version: entry.packageJson.version,
  }));

  return [
    ...platformPackages,
    {
      cwd: path.dirname(report.rootPackage.manifestPath),
      kind: "coordinating",
      manifestPath: report.rootPackage.manifestPath,
      name: report.rootPackage.packageJson.name,
      version: report.rootPackage.packageJson.version,
    },
  ];
}

const args = parseArgs(process.argv.slice(2));
const mode =
  args.publish || args.mode === "publish" ? "publish" : "dry-run";
const stagedRoot = args["staged-root"] ?? args["staged-dist"] ?? "./dist/npm";
const report = verifyPackageSet({
  gitTag: args["git-tag"],
  releaseEvidencePath: args["release-evidence"],
  stagedRoot,
  version: args.version,
});

if (!report.validation.ok) {
  process.stderr.write(
    `npm package-set verification failed before ${mode}.\n${report.validation.errors.map((entry) => `- ${entry}`).join("\n")}\n`,
  );
  process.exit(1);
}

const startedAt = new Date().toISOString();
const packageReports = [];

try {
  for (const pkg of packageSetPlan(report)) {
    process.stdout.write(
      `[npm-release] ${mode} ${pkg.name}@${pkg.version} from ${pkg.cwd}\n`,
    );

    if (mode === "publish") {
      runNpm(["publish", "--access", "public"], { cwd: pkg.cwd, mode });
      packageReports.push({
        cwd: pkg.cwd,
        manifestPath: pkg.manifestPath,
        mode,
        name: pkg.name,
        result: "published",
        version: pkg.version,
      });
      continue;
    }

    const rawPackOutput = runNpm(["pack", "--json", "--dry-run"], {
      cwd: pkg.cwd,
      mode,
    });
    let packOutput = null;

    if (rawPackOutput) {
      try {
        packOutput = JSON.parse(rawPackOutput);
      } catch {
        packOutput = rawPackOutput;
      }
    }

    packageReports.push({
      cwd: pkg.cwd,
      manifestPath: pkg.manifestPath,
      mode,
      name: pkg.name,
      packOutput,
      result: "dry_run_ok",
      version: pkg.version,
    });
  }

  const receiptPath = path.resolve(
    process.cwd(),
    args.report ?? ".work/release/npm/last-package-publication.json",
  );
  const receipt = {
    finishedAt: new Date().toISOString(),
    mode,
    packages: packageReports,
    startedAt,
    stagedRoot: report.stagedRoot,
    version: report.expectedVersion,
  };

  writeJson(receiptPath, receipt);
  process.stdout.write(`npm package-set ${mode} receipt: ${receiptPath}\n`);
} catch (error) {
  const receiptPath = path.resolve(
    process.cwd(),
    args.report ?? ".work/release/npm/last-package-publication.json",
  );
  writeJson(receiptPath, {
    error: error.message,
    failedAt: new Date().toISOString(),
    mode,
    packages: packageReports,
    startedAt,
    stagedRoot: report.stagedRoot,
    version: report.expectedVersion,
  });
  throw error;
}
