import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, it } from "node:test";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = join(__dirname, "..");
const require = createRequire(import.meta.url);

describe("cursor-agent-runner package contract", () => {
  it("is ESM with Node engines >=22.13 and depends on official @cursor/sdk", () => {
    const pkg = JSON.parse(readFileSync(join(PKG_ROOT, "package.json"), "utf8"));
    assert.equal(pkg.type, "module");
    assert.equal(pkg.engines?.node, ">=22.13");
    assert.ok(
      pkg.dependencies?.["@cursor/sdk"],
      "must declare official @cursor/sdk dependency",
    );
    assert.equal(
      pkg.dependencies?.["@cursor/sdk-mock"],
      undefined,
      "must not use a mock SDK package as delivery",
    );
  });

  it("resolves real @cursor/sdk Agent.create export (not a local mock)", () => {
    const sdk = require(join(PKG_ROOT, "node_modules/@cursor/sdk/package.json"));
    assert.equal(sdk.name, "@cursor/sdk");
    assert.match(String(sdk.version), /^\d+\./);
    const mod = require("@cursor/sdk");
    assert.equal(typeof mod.Agent?.create, "function");
  });
});
