import type { JWTVerifyGetKey } from "jose";
import { describe, expect, it, vi } from "vitest";
import type { CredentialsService, UserCredentials } from "../src/gcs/credentials-service";
import { BucketNameTakenError, GcsRequestError, TokenExchangeError } from "../src/gcs/errors";
import { handleRequest, type Services } from "../src/handler";
import { ISSUER, USER_ID, makeSigner } from "./signing-fixture";

function makeRequest(path: string, token?: string, method = "GET"): Request {
  return new Request(`https://worker.test${path}`, {
    method,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
}

const CREDENTIALS: UserCredentials = {
  bucket: "spark-dee0434c91366b18561ffdcad4380e86",
  accessToken: "client-token-value",
  expiresAt: 1793490000,
};

function makeCredentials(issueUserCredentials: CredentialsService["issueUserCredentials"]) {
  const spy = vi.fn(issueUserCredentials);
  return { credentials: { issueUserCredentials: spy } satisfies CredentialsService, spy };
}

function makeServices(keys: JWTVerifyGetKey, credentials?: CredentialsService): Services {
  const unused = makeCredentials(async () => {
    throw new Error("storage must not be touched by this request");
  }).credentials;
  return { verifier: { issuer: ISSUER, keys }, credentials: credentials ?? unused };
}

describe("handleRequest", () => {
  it("answers 200 with the user id for a valid token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/whoami", token), makeServices(signer.keys));
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ userId: USER_ID });
  });

  it("answers 401 when there is no token", async () => {
    const signer = await makeSigner();
    const response = await handleRequest(makeRequest("/whoami"), makeServices(signer.keys));
    expect(response.status).toBe(401);
    expect(response.headers.get("WWW-Authenticate")).toBe("Bearer");
  });

  it("answers 401 for an expired token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken({ expiresAt: Math.floor(Date.now() / 1000) - 60 });
    const response = await handleRequest(makeRequest("/whoami", token), makeServices(signer.keys));
    expect(response.status).toBe(401);
  });

  it("answers 401 for a token signed by another key", async () => {
    const trusted = await makeSigner();
    const stranger = await makeSigner();
    const token = await stranger.signToken();
    const response = await handleRequest(makeRequest("/whoami", token), makeServices(trusted.keys));
    expect(response.status).toBe(401);
  });

  it("checks the token before the route, so an unknown path without a token is 401", async () => {
    const signer = await makeSigner();
    const response = await handleRequest(makeRequest("/nope"), makeServices(signer.keys));
    expect(response.status).toBe(401);
  });

  it("answers 404 for an unknown path with a valid token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/nope", token), makeServices(signer.keys));
    expect(response.status).toBe(404);
  });

  it("answers 404 for a method the route does not serve", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/whoami", token, "POST"), makeServices(signer.keys));
    expect(response.status).toBe(404);
  });

  it("answers 503, not 401, when the signing keys cannot be fetched", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const networkDown: JWTVerifyGetKey = async () => {
      throw new TypeError("fetch failed");
    };
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    const response = await handleRequest(makeRequest("/whoami", token), makeServices(networkDown));
    expect(response.status).toBe(503);
    expect(logged).toHaveBeenCalledOnce();
    expect(String(logged.mock.calls[0][0])).not.toContain(token);
    logged.mockRestore();
  });
});

describe("POST /credentials", () => {
  it("makes sure the signed-in user's bucket exists and hands out a token for it", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { credentials, spy } = makeCredentials(async () => CREDENTIALS);

    const response = await handleRequest(makeRequest("/credentials", token, "POST"), makeServices(signer.keys, credentials));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual(CREDENTIALS);
    expect(spy).toHaveBeenCalledExactlyOnceWith(USER_ID);
  });

  it("tells caches not to keep the token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { credentials } = makeCredentials(async () => CREDENTIALS);

    const response = await handleRequest(makeRequest("/credentials", token, "POST"), makeServices(signer.keys, credentials));

    expect(response.headers.get("Cache-Control")).toBe("no-store");
  });

  it("answers 401 without touching storage when there is no token", async () => {
    const signer = await makeSigner();
    const { credentials, spy } = makeCredentials(async () => CREDENTIALS);

    const response = await handleRequest(makeRequest("/credentials", undefined, "POST"), makeServices(signer.keys, credentials));

    expect(response.status).toBe(401);
    expect(spy).not.toHaveBeenCalled();
  });

  it("serves POST only", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { credentials, spy } = makeCredentials(async () => CREDENTIALS);

    const response = await handleRequest(makeRequest("/credentials", token, "GET"), makeServices(signer.keys, credentials));

    expect(response.status).toBe(404);
    expect(spy).not.toHaveBeenCalled();
  });

  it("no longer serves the bucket-only route that came before it", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const response = await handleRequest(makeRequest("/bucket", token, "POST"), makeServices(signer.keys));
    expect(response.status).toBe(404);
  });

  it("answers 409 and logs when the bucket name belongs to someone else", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { credentials } = makeCredentials(async () => {
      throw new BucketNameTakenError("bucket name belongs to another project");
    });
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});

    const response = await handleRequest(makeRequest("/credentials", token, "POST"), makeServices(signer.keys, credentials));

    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ error: "bucket_name_taken" });
    expect(logged).toHaveBeenCalledOnce();
    expect(JSON.parse(String(logged.mock.calls[0][0]))).toMatchObject({ errorName: "BucketNameTakenError" });
    logged.mockRestore();
  });

  it.each([
    ["a storage failure", () => new GcsRequestError("create bucket", 500, "backend error"), "GcsRequestError", "HTTP 500"],
    ["a refused token exchange", () => new TokenExchangeError("token exchange answered HTTP 400: invalid_request"), "TokenExchangeError", "HTTP 400"],
  ])("answers 503 and logs for %s, without leaking the user's token", async (_label, makeError, errorName, detail) => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { credentials } = makeCredentials(async () => {
      throw makeError();
    });
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});

    const response = await handleRequest(makeRequest("/credentials", token, "POST"), makeServices(signer.keys, credentials));

    expect(response.status).toBe(503);
    expect(await response.json()).toEqual({ error: "storage_unavailable" });
    expect(logged).toHaveBeenCalledOnce();
    const line = String(logged.mock.calls[0][0]);
    expect(JSON.parse(line)).toMatchObject({ errorName });
    expect(line).toContain(detail);
    expect(line).not.toContain(token);
    logged.mockRestore();
  });
});
