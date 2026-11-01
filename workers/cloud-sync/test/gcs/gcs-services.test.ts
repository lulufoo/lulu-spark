import { describe, expect, it } from "vitest";
import { InvalidLimitsError, ObjectTooLargeError, QuotaExceededError } from "../../src/gcs/errors";
import { createGcsServices, type GcsServicesConfig } from "../../src/gcs/gcs-services";
import { jsonReply, makeFakeFetch, type RecordedRequest } from "./fake-fetch";
import { TOKEN_URI, makeServiceAccount } from "./service-account-fixture";

const BUCKET_NAME_KEY = "test-bucket-name-key-0123456789abcdef";
const USER_ID = "123e4567-e89b-12d3-a456-426614174000";
const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86"; // what the key and user above give
const DOCUMENT_ID = "0123456789abcdef0123456789abcdef";
const START_MS = Date.parse("2026-11-01T00:00:00Z");
const MAX_OBJECT = 10_000;
const MAX_TOTAL = 50_000;

/** Google's token endpoint and a storage API with one existing bucket, whose contents the test sets. */
function makeFakeGoogle(storage: { versionSizes: number[]; generation?: string }) {
  let sourceTokens = 0;
  return makeFakeFetch((request: RecordedRequest) => {
    if (request.url === TOKEN_URI) {
      sourceTokens += 1;
      return jsonReply(200, { access_token: `source-tok-${sourceTokens}`, expires_in: 3600 });
    }
    const object = /\/b\/[^/?]+\/o(?:\/([^?]*))?(?:\?|$)/.exec(request.url);
    if (object) {
      if (request.method === "DELETE") return new Response(null, { status: 204 });
      if (object[1] === undefined) return jsonReply(200, { items: storage.versionSizes.map((size) => ({ size: String(size) })) });
      return storage.generation ? jsonReply(200, { generation: storage.generation }) : jsonReply(404, { error: { status: "NOT_FOUND" } });
    }
    if (request.method === "GET" && /\/b\/[^/?]+/.test(request.url)) return jsonReply(200, { projectNumber: "55973988960" });
    throw new Error(`unexpected request: ${request.method} ${request.url}`);
  });
}

async function makeServices(fetchFn: typeof fetch, overrides: Partial<GcsServicesConfig> = {}) {
  const account = await makeServiceAccount();
  return createGcsServices({
    serviceAccountKeyJson: account.keyJson,
    bucketNameKey: BUCKET_NAME_KEY,
    projectId: "spark-cloud-sync",
    projectNumber: "55973988960",
    maxObjectBytes: MAX_OBJECT,
    maxUserTotalBytes: MAX_TOTAL,
    uploadLinkLifetimeSeconds: 600,
    fetchFn,
    now: () => START_MS,
    ...overrides,
  });
}

describe("createGcsServices: documents", () => {
  it("signs an upload link for the user's bucket that works for the configured lifetime", async () => {
    const { fetchFn } = makeFakeGoogle({ versionSizes: [100, 200] });
    const { documents } = await makeServices(fetchFn);

    const upload = await documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 5000, version: 4 });

    const url = new URL(upload.url);
    expect(upload.method).toBe("PUT");
    expect(url.pathname).toBe(`/${BUCKET}/${DOCUMENT_ID}`);
    expect(url.searchParams.get("X-Goog-Expires")).toBe("600");
    expect(upload.expiresAt).toBe(START_MS / 1000 + 600);
    expect(upload.headers).toEqual({
      "x-goog-content-length-range": "0,5000",
      "x-goog-if-generation-match": "0",
      "x-goog-meta-version": "4",
    });
  });

  it("takes the lifetime from the setting", async () => {
    const { fetchFn } = makeFakeGoogle({ versionSizes: [] });
    const { documents } = await makeServices(fetchFn, { uploadLinkLifetimeSeconds: "900" });

    const upload = await documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 1, version: 0 });

    expect(new URL(upload.url).searchParams.get("X-Goog-Expires")).toBe("900");
    expect(upload.expiresAt).toBe(START_MS / 1000 + 900);
  });

  it("ties the link to the existing object's generation", async () => {
    const { fetchFn } = makeFakeGoogle({ versionSizes: [100], generation: "1730419200123456" });
    const { documents } = await makeServices(fetchFn);

    const upload = await documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 10, version: 1 });

    expect(upload.headers["x-goog-if-generation-match"]).toBe("1730419200123456");
  });

  it("asks Google about every version in the user's bucket, with one shared access token", async () => {
    const { fetchFn, requests } = makeFakeGoogle({ versionSizes: [] });
    const { documents } = await makeServices(fetchFn);

    await documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 10, version: 1 });
    await documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 10, version: 2 });

    const storageCalls = requests.filter((request) => request.url.startsWith("https://storage.googleapis.com/"));
    const listings = storageCalls.filter((request) => new URL(request.url).pathname === `/storage/v1/b/${BUCKET}/o`);
    expect(listings).toHaveLength(2);
    expect(listings.every((request) => new URL(request.url).searchParams.get("versions") === "true")).toBe(true);
    expect(requests.filter((request) => request.url === TOKEN_URI)).toHaveLength(1);
    expect(new Set(storageCalls.map((request) => request.headers.get("Authorization")))).toEqual(new Set(["Bearer source-tok-1"]));
  });

  it("refuses an object over the limit and a user over the total, using the configured numbers", async () => {
    const { fetchFn } = makeFakeGoogle({ versionSizes: [MAX_TOTAL - 100] });
    const { documents } = await makeServices(fetchFn);

    await expect(documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: MAX_OBJECT + 1, version: 1 })).rejects.toBeInstanceOf(ObjectTooLargeError);
    await expect(documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 101, version: 1 })).rejects.toBeInstanceOf(QuotaExceededError);
    await expect(documents.issueUpload(USER_ID, { documentId: DOCUMENT_ID, size: 100, version: 1 })).resolves.toMatchObject({ method: "PUT" });
  });

  it("deletes from the bucket that belongs to the user", async () => {
    const { fetchFn, requests } = makeFakeGoogle({ versionSizes: [] });
    const { documents } = await makeServices(fetchFn);

    await documents.deleteDocument(USER_ID, DOCUMENT_ID);

    const deletes = requests.filter((request) => request.method === "DELETE");
    expect(deletes.map((request) => request.url)).toEqual([`https://storage.googleapis.com/storage/v1/b/${BUCKET}/o/${DOCUMENT_ID}`]);
  });
});

describe("createGcsServices: settings", () => {
  it.each([
    ["no object limit", { maxObjectBytes: undefined }, /MAX_OBJECT_BYTES/],
    ["no total limit", { maxUserTotalBytes: undefined }, /MAX_USER_TOTAL_BYTES/],
    ["an object limit above the total", { maxObjectBytes: MAX_TOTAL + 1 }, /larger than/],
    ["no link lifetime", { uploadLinkLifetimeSeconds: undefined }, /UPLOAD_LINK_LIFETIME_SECONDS/],
    ["a link lifetime of 30 seconds", { uploadLinkLifetimeSeconds: 30 }, /UPLOAD_LINK_LIFETIME_SECONDS/],
    ["a link lifetime of 2 hours", { uploadLinkLifetimeSeconds: 7200 }, /UPLOAD_LINK_LIFETIME_SECONDS/],
  ])("refuses to start with %s", async (_label, change, message) => {
    const { fetchFn } = makeFakeGoogle({ versionSizes: [] });
    await expect(makeServices(fetchFn, change)).rejects.toBeInstanceOf(InvalidLimitsError);
    await expect(makeServices(fetchFn, change)).rejects.toThrow(message);
  });
});
