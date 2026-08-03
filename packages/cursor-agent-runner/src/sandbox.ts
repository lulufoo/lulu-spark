import { accessSync, constants, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { RunnerError } from "./protocol.ts";

/** Cursor-hosted inference / API hosts required for Local agent runs. */
export const CURSOR_NETWORK_ALLOWLIST = [
  "cursor.com",
  "*.cursor.com",
] as const;

export type EnsureNodeAndSandboxInput = {
  nodeVersion?: string;
  cwd: string;
  workbenchMcpHost: string;
  probeSandboxHelper?: () => boolean;
};

export type EnsureNodeAndSandboxResult =
  | { ok: true; cwd: string }
  | { ok: false; error: RunnerError; cwd?: string };

const MIN_NODE = { major: 22, minor: 13, patch: 0 };

export function parseNodeVersion(version: string): {
  major: number;
  minor: number;
  patch: number;
} | null {
  const cleaned = version.trim().replace(/^v/i, "");
  const m = /^(\d+)\.(\d+)\.(\d+)/.exec(cleaned);
  if (!m) return null;
  return {
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
  };
}

export function isNodeVersionAtLeast(
  version: string,
  min = MIN_NODE,
): boolean {
  const parsed = parseNodeVersion(version);
  if (!parsed) return false;
  if (parsed.major !== min.major) return parsed.major > min.major;
  if (parsed.minor !== min.minor) return parsed.minor > min.minor;
  return parsed.patch >= min.patch;
}

/** Platform probe mirroring SDK Local sandbox prerequisites (macOS seatbelt). */
export function defaultProbeSandboxHelper(): boolean {
  if (process.platform === "darwin") {
    try {
      accessSync("/usr/bin/sandbox-exec", constants.X_OK);
      return true;
    } catch {
      return false;
    }
  }
  if (process.platform === "linux") {
    try {
      accessSync("/usr/bin/bwrap", constants.X_OK);
      return true;
    } catch {
      // Landlock path may still work inside Cursor's helper; without helper, fail closed.
      return false;
    }
  }
  return false;
}

export function writeSessionSandboxAllowlist(
  cwd: string,
  workbenchMcpHost: string,
): void {
  const dir = join(cwd, ".cursor");
  mkdirSync(dir, { recursive: true });
  const allow = [
    ...CURSOR_NETWORK_ALLOWLIST,
    workbenchMcpHost,
    "localhost",
  ];
  const unique = [...new Set(allow.filter(Boolean))];
  const body = {
    type: "workspace_readwrite",
    networkPolicy: {
      default: "deny",
      allow: unique,
    },
  };
  writeFileSync(join(dir, "sandbox.json"), `${JSON.stringify(body, null, 2)}\n`);
}

export function ensureNodeAndSandbox(
  input: EnsureNodeAndSandboxInput,
): EnsureNodeAndSandboxResult {
  const nodeVersion = input.nodeVersion ?? process.versions.node;
  if (!isNodeVersionAtLeast(nodeVersion)) {
    return {
      ok: false,
      cwd: input.cwd,
      error: {
        type: "sdk_config",
        message: `Node.js >= 22.13 required for @cursor/sdk Local; found ${nodeVersion}`,
      },
    };
  }

  const probe = input.probeSandboxHelper ?? defaultProbeSandboxHelper;
  if (!probe()) {
    return {
      ok: false,
      cwd: input.cwd,
      error: {
        type: "sdk_config",
        message:
          "Local SDK sandbox/allowlist cannot be established on this platform; fail closed",
      },
    };
  }

  try {
    writeSessionSandboxAllowlist(input.cwd, input.workbenchMcpHost);
  } catch (err) {
    return {
      ok: false,
      cwd: input.cwd,
      error: {
        type: "sdk_config",
        message: `failed to write sandbox allowlist: ${err instanceof Error ? err.message : "unknown"}`,
      },
    };
  }

  return { ok: true, cwd: input.cwd };
}
