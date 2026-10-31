import type { JWTVerifyGetKey } from "jose";
import { describe, expect, it, vi } from "vitest";
import { handleRequest } from "../src/handler";
import { ISSUER, USER_ID, makeSigner } from "./signing-fixture";

function makeRequest(path: string, token?: string, method = "GET"): Request {
  return new Request(`https://worker.test${path}`, {
    method,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
}

describe("handleRequest", () => {
  it("answers 200 with the user id for a valid token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/whoami", token), {
      issuer: ISSUER,
      keys: signer.keys,
    });
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ userId: USER_ID });
  });

  it("answers 401 when there is no token", async () => {
    const signer = await makeSigner();
    const response = await handleRequest(makeRequest("/whoami"), {
      issuer: ISSUER,
      keys: signer.keys,
    });
    expect(response.status).toBe(401);
    expect(response.headers.get("WWW-Authenticate")).toBe("Bearer");
  });

  it("answers 401 for an expired token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken({ expiresAt: Math.floor(Date.now() / 1000) - 60 });
    const response = await handleRequest(makeRequest("/whoami", token), {
      issuer: ISSUER,
      keys: signer.keys,
    });
    expect(response.status).toBe(401);
  });

  it("answers 401 for a token signed by another key", async () => {
    const trusted = await makeSigner();
    const stranger = await makeSigner();
    const token = await stranger.signToken();
    const response = await handleRequest(makeRequest("/whoami", token), {
      issuer: ISSUER,
      keys: trusted.keys,
    });
    expect(response.status).toBe(401);
  });

  it("checks the token before the route, so an unknown path without a token is 401", async () => {
    const signer = await makeSigner();
    const response = await handleRequest(makeRequest("/nope"), {
      issuer: ISSUER,
      keys: signer.keys,
    });
    expect(response.status).toBe(401);
  });

  it("answers 404 for an unknown path with a valid token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/nope", token), {
      issuer: ISSUER,
      keys: signer.keys,
    });
    expect(response.status).toBe(404);
  });

  it("answers 404 for a method the route does not serve", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/whoami", token, "POST"), {
      issuer: ISSUER,
      keys: signer.keys,
    });
    expect(response.status).toBe(404);
  });

  it("answers 503, not 401, when the signing keys cannot be fetched", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const networkDown: JWTVerifyGetKey = async () => {
      throw new TypeError("fetch failed");
    };
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    const response = await handleRequest(makeRequest("/whoami", token), {
      issuer: ISSUER,
      keys: networkDown,
    });
    expect(response.status).toBe(503);
    expect(logged).toHaveBeenCalledOnce();
    expect(String(logged.mock.calls[0][0])).not.toContain(token);
    logged.mockRestore();
  });
});
