import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, it } from "node:test";
import {
  CURSOR_NETWORK_ALLOWLIST,
  ensureNodeAndSandbox,
  writeSessionSandboxAllowlist,
} from "../src/sandbox.ts";

const temps: string[] = [];

afterEach(() => {
  while (temps.length) {
    const p = temps.pop();
    if (p) rmSync(p, { recursive: true, force: true });
  }
});

describe("Node + optional SDK sandbox gates", () => {
  it("ensureNodeAndSandbox fails closed when Node < 22.13", () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-sb-"));
    temps.push(cwd);
    const result = ensureNodeAndSandbox({
      nodeVersion: "22.12.0",
      cwd,
      workbenchMcpHost: "127.0.0.1",
      sdkSandboxEnabled: false,
    });
    assert.equal(result.ok, false);
    if (result.ok) throw new Error("expected failure");
    assert.equal(result.error.type, "sdk_config");
    assert.match(result.error.message, /22\.13/);
  });

  it("Workbench path (sdkSandboxEnabled false) accepts Node ≥ 22.13 without sandbox helper", () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-sb-"));
    temps.push(cwd);
    const result = ensureNodeAndSandbox({
      nodeVersion: "22.13.0",
      cwd,
      workbenchMcpHost: "127.0.0.1",
      sdkSandboxEnabled: false,
      probeSandboxHelper: () => false,
    });
    assert.equal(result.ok, true);
  });

  it("when sdkSandboxEnabled true, fails closed if sandbox helper unavailable", () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-sb-"));
    temps.push(cwd);
    const result = ensureNodeAndSandbox({
      nodeVersion: "22.13.0",
      cwd,
      workbenchMcpHost: "127.0.0.1",
      sdkSandboxEnabled: true,
      probeSandboxHelper: () => false,
    });
    assert.equal(result.ok, false);
    if (result.ok) throw new Error("expected failure");
    assert.equal(result.error.type, "sdk_config");
    assert.match(result.error.message, /sandbox/i);
  });

  it("when sdkSandboxEnabled true and helper ok, writes allowlist", () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-sb-"));
    temps.push(cwd);
    const result = ensureNodeAndSandbox({
      nodeVersion: "22.13.0",
      cwd,
      workbenchMcpHost: "127.0.0.1",
      sdkSandboxEnabled: true,
      probeSandboxHelper: () => true,
    });
    assert.equal(result.ok, true);
    const raw = readFileSync(join(cwd, ".cursor", "sandbox.json"), "utf8");
    const cfg = JSON.parse(raw) as {
      networkPolicy: { default: string; allow: string[] };
    };
    assert.equal(cfg.networkPolicy.default, "deny");
    for (const host of CURSOR_NETWORK_ALLOWLIST) {
      assert.ok(cfg.networkPolicy.allow.includes(host), `missing ${host}`);
    }
  });

  it("writes session sandbox.json allowlisting Cursor endpoints + Workbench MCP", () => {
    const cwd = mkdtempSync(join(tmpdir(), "car-sb-"));
    temps.push(cwd);
    writeSessionSandboxAllowlist(cwd, "127.0.0.1");
    const raw = readFileSync(join(cwd, ".cursor", "sandbox.json"), "utf8");
    const cfg = JSON.parse(raw) as {
      networkPolicy: { default: string; allow: string[] };
    };
    assert.equal(cfg.networkPolicy.default, "deny");
    for (const host of CURSOR_NETWORK_ALLOWLIST) {
      assert.ok(cfg.networkPolicy.allow.includes(host), `missing ${host}`);
    }
    assert.ok(cfg.networkPolicy.allow.includes("127.0.0.1"));
  });
});
