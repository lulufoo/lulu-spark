// The whole document-storage service (limits, signed links, delete) against real Google Cloud Storage.
// Needs the real name key, so the user's bucket is one that already exists and no new bucket is made:
//   GCS_SERVICE_ACCOUNT_KEY="$(cat ~/.config/lulu-spark/gcs-sync-sa-key.json)" \
//   BUCKET_NAME_KEY="$(cat ~/.config/lulu-spark/bucket-name-key)" \
//   GCS_LIVE_USER_ID=<a user id whose bucket exists> \
//   npx vitest run --config vitest.live.config.ts test-live/documents-live.test.ts
// It writes a few kilobytes under random document ids and deletes them at the end; the old versions
// stay until the bucket's 3-day rule removes them.
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createAccessTokenProvider } from "../src/gcs/access-token-provider";
import { ObjectTooLargeError, QuotaExceededError } from "../src/gcs/errors";
import { createGcsServices, type GcsServices } from "../src/gcs/gcs-services";
import { createObjectStore } from "../src/gcs/object-store";
import { parseServiceAccountKey } from "../src/gcs/service-account-key";
import type { SignedUpload } from "../src/gcs/signed-upload-url";
import { KEY_JSON, PROJECT_ID, PROJECT_NUMBER, STORAGE, makeRandomHex } from "./support";

const BUCKET_NAME_KEY = process.env.BUCKET_NAME_KEY?.trim();
const USER_ID = process.env.GCS_LIVE_USER_ID;
const OBJECT_LIMIT = 1500;
const LIFETIME_SECONDS = 600;

describe.skipIf(!KEY_JSON || !BUCKET_NAME_KEY || !USER_ID)("document storage against real Google Cloud Storage", () => {
  const key = parseServiceAccountKey(KEY_JSON);
  const longLived = createAccessTokenProvider({ key });
  const store = createObjectStore({ getAccessToken: () => longLived.getAccessToken() });
  const created: string[] = [];
  let bucket = "";
  let clientToken = "";
  let documents: GcsServices["documents"];
  let services: (totalLimit: number) => GcsServices;

  const newDocumentId = () => {
    const id = makeRandomHex(16);
    created.push(id);
    return id;
  };
  const put = (upload: SignedUpload, body: string) => fetch(upload.url, { method: "PUT", headers: upload.headers, body });
  const readWithClientToken = async (documentId: string, query: string) =>
    fetch(`${STORAGE}/b/${bucket}/o/${documentId}?${query}`, { headers: { Authorization: `Bearer ${clientToken}` } });

  beforeAll(async () => {
    services = (totalLimit) =>
      createGcsServices({
        serviceAccountKeyJson: KEY_JSON,
        bucketNameKey: BUCKET_NAME_KEY,
        projectId: PROJECT_ID,
        projectNumber: PROJECT_NUMBER,
        maxObjectBytes: OBJECT_LIMIT,
        maxUserTotalBytes: totalLimit,
        uploadLinkLifetimeSeconds: LIFETIME_SECONDS,
      });
    // The first service only finds the bucket and a read token; the limits that matter are set per test.
    const credentials = await services(1_000_000).credentials.issueUserCredentials(USER_ID!);
    bucket = credentials.bucket;
    clientToken = credentials.accessToken;
    documents = services((await store.totalBytes(bucket)) + 100_000).documents;
  });

  afterAll(async () => {
    for (const id of created) await store.deleteObject(bucket, id);
  });

  it("an upload within the limits arrives with the right bytes, size and version, and the client token can read it", async () => {
    const id = newDocumentId();
    const upload = await documents.issueUpload(USER_ID!, { documentId: id, size: 1000, version: 5 });

    const response = await put(upload, "a".repeat(1000));

    expect(response.status).toBe(200);
    const metadata = (await (await readWithClientToken(id, "fields=size,metadata")).json()) as { size?: string; metadata?: { version?: string } };
    expect(metadata).toMatchObject({ size: "1000", metadata: { version: "5" } });
    expect(await (await readWithClientToken(id, "alt=media")).text()).toBe("a".repeat(1000));
  });

  it("a link that already succeeded is refused with 412", async () => {
    const id = newDocumentId();
    const upload = await documents.issueUpload(USER_ID!, { documentId: id, size: 10, version: 1 });
    expect((await put(upload, "0123456789")).status).toBe(200);

    expect((await put(upload, "0123456789")).status).toBe(412);
  });

  it("bytes over the size that was asked for are refused with 400", async () => {
    const id = newDocumentId();
    const upload = await documents.issueUpload(USER_ID!, { documentId: id, size: 500, version: 1 });

    expect((await put(upload, "b".repeat(501))).status).toBe(400);
    expect((await put(upload, "b".repeat(500))).status).toBe(200);
  });

  it("an existing object can be overwritten once per link, and a link signed before the overwrite is refused", async () => {
    const id = newDocumentId();
    const first = await documents.issueUpload(USER_ID!, { documentId: id, size: 10, version: 1 });
    expect((await put(first, "first-0001")).status).toBe(200);

    const overwrite = await documents.issueUpload(USER_ID!, { documentId: id, size: 10, version: 2 });
    const stale = await documents.issueUpload(USER_ID!, { documentId: id, size: 10, version: 2 });
    expect((await put(overwrite, "second-002")).status).toBe(200);

    expect((await put(stale, "third-0003")).status).toBe(412);
    expect(await (await readWithClientToken(id, "alt=media")).text()).toBe("second-002");
    expect(((await (await readWithClientToken(id, "fields=metadata")).json()) as { metadata: { version: string } }).metadata.version).toBe("2");
  });

  it("refuses an object over the limit without signing anything", async () => {
    await expect(documents.issueUpload(USER_ID!, { documentId: newDocumentId(), size: OBJECT_LIMIT + 1, version: 1 })).rejects.toBeInstanceOf(ObjectTooLargeError);
  });

  it("counts old versions toward the total, stops signing past it, and does not free space on delete", async () => {
    const usedAtStart = await store.totalBytes(bucket);
    const limited = services(usedAtStart + 2500).documents; // room for two 1000-byte writes, not three
    const id = newDocumentId();

    const writeOnce = async (version: number) => {
      const upload = await limited.issueUpload(USER_ID!, { documentId: id, size: 1000, version });
      expect((await put(upload, "c".repeat(1000))).status).toBe(200);
    };
    await writeOnce(1);
    await writeOnce(2); // the first write is now an old version, and still counts
    expect(await store.totalBytes(bucket)).toBe(usedAtStart + 2000);

    const refused = await limited.issueUpload(USER_ID!, { documentId: id, size: 1000, version: 3 }).catch((caught: unknown) => caught);
    expect(refused).toBeInstanceOf(QuotaExceededError);
    expect((refused as QuotaExceededError).usedBytes).toBe(usedAtStart + 2000);
    expect((refused as QuotaExceededError).limitBytes).toBe(usedAtStart + 2500);

    // Deleting the document removes the live object, but both versions stay until the 3-day rule runs.
    await limited.deleteDocument(USER_ID!, id);
    expect(await store.currentGeneration(bucket, id)).toBeNull();
    expect(await store.totalBytes(bucket)).toBe(usedAtStart + 2000);
    await expect(limited.issueUpload(USER_ID!, { documentId: newDocumentId(), size: 1000, version: 1 })).rejects.toBeInstanceOf(QuotaExceededError);
  });

  it("deleting removes only that document, and deleting again, or one that never existed, succeeds", async () => {
    const kept = newDocumentId();
    const removed = newDocumentId();
    for (const id of [kept, removed]) {
      expect((await put(await documents.issueUpload(USER_ID!, { documentId: id, size: 4, version: 1 }), "data")).status).toBe(200);
    }

    await documents.deleteDocument(USER_ID!, removed);
    await documents.deleteDocument(USER_ID!, removed);
    await documents.deleteDocument(USER_ID!, newDocumentId());

    expect(await store.currentGeneration(bucket, removed)).toBeNull();
    expect(await store.currentGeneration(bucket, kept)).not.toBeNull();
  });
});
