import { describe, expect, it } from "vitest";
import { createCredentialsService } from "../../src/gcs/credentials-service";
import { InvalidSecretError, InvalidServiceAccountKeyError, TokenExchangeError } from "../../src/gcs/errors";
import { jsonReply, makeFakeFetch, type RecordedRequest } from "./fake-fetch";
import { TOKEN_URI, makeServiceAccount } from "./service-account-fixture";

const STS_URL = "https://sts.googleapis.com/v1/token";
const BUCKET_NAME_KEY = "test-bucket-name-key-0123456789abcdef";
const USER_ID = "123e4567-e89b-12d3-a456-426614174000";
const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86";
const START_MS = Date.parse("2026-11-01T00:00:00Z");
const MINUTE_MS = 60 * 1000;

/** Google's token endpoint, its token exchange, and a storage API that remembers which buckets exist. */
function makeFakeGoogle(options: { exchangeStatus?: number } = {}) {
  const existing = new Set<string>();
  let sourceTokens = 0;
  return makeFakeFetch((request: RecordedRequest) => {
    if (request.url === TOKEN_URI) {
      sourceTokens += 1;
      return jsonReply(200, { access_token: `source-tok-${sourceTokens}`, expires_in: 3600 });
    }
    if (request.url === STS_URL) {
      return options.exchangeStatus
        ? jsonReply(options.exchangeStatus, { error: "invalid_request" })
        : jsonReply(200, { access_token: "client-tok", expires_in: 3000 });
    }
    const bucket = /\/b\/([^?/]+)/.exec(request.url)?.[1];
    if (request.method === "GET" && bucket) {
      return existing.has(bucket)
        ? jsonReply(200, { projectNumber: "55973988960" })
        : jsonReply(404, { error: { message: "not found" } });
    }
    if (request.method === "POST") {
      existing.add(JSON.parse(request.body).name);
      return jsonReply(200, {});
    }
    throw new Error(`unexpected request: ${request.method} ${request.url}`);
  });
}

async function makeService(fetchFn: typeof fetch, overrides: { bucketNameKey?: string; keyJson?: string } = {}, clock = { now: START_MS }) {
  const account = await makeServiceAccount();
  return createCredentialsService({
    serviceAccountKeyJson: overrides.keyJson ?? account.keyJson,
    bucketNameKey: "bucketNameKey" in overrides ? overrides.bucketNameKey : BUCKET_NAME_KEY,
    projectId: "spark-cloud-sync",
    projectNumber: "55973988960",
    fetchFn,
    now: () => clock.now,
  });
}

describe("createCredentialsService", () => {
  it("creates the user's bucket on first use, then hands out a read-only token for it", async () => {
    const { fetchFn, requests } = makeFakeGoogle();
    const service = await makeService(fetchFn);

    const credentials = await service.issueUserCredentials(USER_ID);

    expect(credentials).toEqual({ bucket: BUCKET, accessToken: "client-tok", expiresAt: START_MS / 1000 + 3000 });
    const createIndex = requests.findIndex((request) => request.method === "POST" && request.url.includes("/storage/v1/b?"));
    const exchangeIndex = requests.findIndex((request) => request.url === STS_URL);
    expect(createIndex).toBeGreaterThan(-1);
    expect(exchangeIndex).toBeGreaterThan(createIndex); // the bucket exists before a token is made for it
    expect(new URLSearchParams(requests[exchangeIndex].body).get("subject_token")).toBe("source-tok-1");
  });

  it("gives a repeat caller the same bucket without creating it again", async () => {
    const { fetchFn, requests } = makeFakeGoogle();
    const service = await makeService(fetchFn);

    await service.issueUserCredentials(USER_ID);
    const second = await service.issueUserCredentials(USER_ID);

    expect(second.bucket).toBe(BUCKET);
    expect(requests.filter((request) => request.method === "POST" && request.url.includes("/storage/v1/b?"))).toHaveLength(1);
  });

  it("shares one Google access token while it has more than half an hour left, then takes a new one", async () => {
    const { fetchFn, requests } = makeFakeGoogle();
    const clock = { now: START_MS };
    const service = await makeService(fetchFn, {}, clock);
    const sourceTokenRequests = () => requests.filter((request) => request.url === TOKEN_URI).length;

    await service.issueUserCredentials(USER_ID);
    clock.now = START_MS + 25 * MINUTE_MS; // 35 minutes left
    await service.issueUserCredentials(USER_ID);
    expect(sourceTokenRequests()).toBe(1);

    clock.now = START_MS + 35 * MINUTE_MS; // 25 minutes left: too little to hand on
    await service.issueUserCredentials(USER_ID);
    expect(sourceTokenRequests()).toBe(2);
    const exchanges = requests.filter((request) => request.url === STS_URL);
    expect(new URLSearchParams(exchanges.at(-1)?.body).get("subject_token")).toBe("source-tok-2");
  });

  it("fails, rather than hand out something wider, when the exchange is refused", async () => {
    const { fetchFn } = makeFakeGoogle({ exchangeStatus: 400 });
    const service = await makeService(fetchFn);
    await expect(service.issueUserCredentials(USER_ID)).rejects.toBeInstanceOf(TokenExchangeError);
  });

  it("refuses to start without BUCKET_NAME_KEY, or with a cut-off one", async () => {
    const { fetchFn } = makeFakeGoogle();
    await expect(makeService(fetchFn, { bucketNameKey: undefined })).rejects.toBeInstanceOf(InvalidSecretError);
    await expect(makeService(fetchFn, { bucketNameKey: "short" })).rejects.toBeInstanceOf(InvalidSecretError);
  });

  it("refuses to start without a usable service account key", async () => {
    const { fetchFn } = makeFakeGoogle();
    await expect(makeService(fetchFn, { keyJson: "{}" })).rejects.toBeInstanceOf(InvalidServiceAccountKeyError);
  });

  it("hands out the bucket, the client token and its expiry, and no key material", async () => {
    const { fetchFn } = makeFakeGoogle();
    const service = await makeService(fetchFn);

    const text = JSON.stringify(await service.issueUserCredentials(USER_ID));

    expect(Object.keys(JSON.parse(text)).sort()).toEqual(["accessToken", "bucket", "expiresAt"]);
    expect(text).not.toContain(BUCKET_NAME_KEY);
    expect(text).not.toContain("PRIVATE KEY");
    expect(text).not.toContain("source-tok");
  });
});
