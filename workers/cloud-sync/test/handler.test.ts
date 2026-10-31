import type { JWTVerifyGetKey } from "jose";
import { describe, expect, it, vi } from "vitest";
import type { BucketService } from "../src/gcs/bucket-service";
import { BucketNameTakenError, GcsRequestError } from "../src/gcs/errors";
import { handleRequest, type Services } from "../src/handler";
import { ISSUER, USER_ID, makeSigner } from "./signing-fixture";

function makeRequest(path: string, token?: string, method = "GET"): Request {
  return new Request(`https://worker.test${path}`, {
    method,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
}

const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86";

function makeBuckets(ensureUserBucket: BucketService["ensureUserBucket"]) {
  const spy = vi.fn(ensureUserBucket);
  return { buckets: { ensureUserBucket: spy } satisfies BucketService, spy };
}

function makeServices(keys: JWTVerifyGetKey, buckets?: BucketService): Services {
  const unused = makeBuckets(async () => {
    throw new Error("storage must not be touched by this request");
  }).buckets;
  return { verifier: { issuer: ISSUER, keys }, buckets: buckets ?? unused };
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

describe("POST /bucket", () => {
  it("makes sure the signed-in user's bucket exists and returns its name", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { buckets, spy } = makeBuckets(async () => ({ bucket: BUCKET, created: true }));

    const response = await handleRequest(makeRequest("/bucket", token, "POST"), makeServices(signer.keys, buckets));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ bucket: BUCKET, created: true });
    expect(spy).toHaveBeenCalledExactlyOnceWith(USER_ID);
  });

  it("answers 401 without touching storage when there is no token", async () => {
    const signer = await makeSigner();
    const { buckets, spy } = makeBuckets(async () => ({ bucket: BUCKET, created: true }));

    const response = await handleRequest(makeRequest("/bucket", undefined, "POST"), makeServices(signer.keys, buckets));

    expect(response.status).toBe(401);
    expect(spy).not.toHaveBeenCalled();
  });

  it("serves POST only", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { buckets, spy } = makeBuckets(async () => ({ bucket: BUCKET, created: true }));

    const response = await handleRequest(makeRequest("/bucket", token, "GET"), makeServices(signer.keys, buckets));

    expect(response.status).toBe(404);
    expect(spy).not.toHaveBeenCalled();
  });

  it("answers 409 and logs when the bucket name belongs to someone else", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { buckets } = makeBuckets(async () => {
      throw new BucketNameTakenError("bucket name belongs to another project");
    });
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});

    const response = await handleRequest(makeRequest("/bucket", token, "POST"), makeServices(signer.keys, buckets));

    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ error: "bucket_name_taken" });
    expect(logged).toHaveBeenCalledOnce();
    expect(JSON.parse(String(logged.mock.calls[0][0]))).toMatchObject({ errorName: "BucketNameTakenError" });
    logged.mockRestore();
  });

  it("answers 503 and logs for any other storage failure, without leaking the token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { buckets } = makeBuckets(async () => {
      throw new GcsRequestError("create bucket", 500, "backend error");
    });
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});

    const response = await handleRequest(makeRequest("/bucket", token, "POST"), makeServices(signer.keys, buckets));

    expect(response.status).toBe(503);
    expect(await response.json()).toEqual({ error: "storage_unavailable" });
    expect(logged).toHaveBeenCalledOnce();
    const line = String(logged.mock.calls[0][0]);
    expect(JSON.parse(line)).toMatchObject({ errorName: "GcsRequestError" });
    expect(line).toContain("HTTP 500");
    expect(line).not.toContain(token);
    logged.mockRestore();
  });
});
