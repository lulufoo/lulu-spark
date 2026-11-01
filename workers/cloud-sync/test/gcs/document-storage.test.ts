import { describe, expect, it, vi } from "vitest";
import type { BucketService } from "../../src/gcs/bucket-service";
import { createDocumentStorage } from "../../src/gcs/document-storage";
import { GcsRequestError, InvalidRequestError, ListingTooLargeError, ObjectTooLargeError, QuotaExceededError } from "../../src/gcs/errors";
import type { ObjectStore } from "../../src/gcs/object-store";
import type { UploadUrlRequest, UploadUrlSigner } from "../../src/gcs/signed-upload-url";

const MAX_OBJECT = 1000;
const MAX_TOTAL = 5000;
const USER_ID = "123e4567-e89b-12d3-a456-426614174000";
const OTHER_USER_ID = "00000000-0000-4000-8000-000000000001";
const DOCUMENT_ID = "0123456789abcdef0123456789abcdef";

/** Bucket names the way the real service would give them: one per user, never shared. */
function bucketOf(userId: string): string {
  return `spark-for-${userId}`;
}

function makeFixture(options: { usedBytes?: number; generation?: string | null } = {}) {
  const calls: string[] = [];
  const buckets = {
    ensureUserBucket: vi.fn(async (userId: string) => {
      calls.push("ensureUserBucket");
      return { bucket: bucketOf(userId), created: false };
    }),
    nameFor: vi.fn(async (userId: string) => bucketOf(userId)),
  } satisfies BucketService;
  const store = {
    totalBytes: vi.fn(async (_bucket: string) => {
      calls.push("totalBytes");
      return options.usedBytes ?? 0;
    }),
    currentGeneration: vi.fn(async (_bucket: string, _object: string) => {
      calls.push("currentGeneration");
      return options.generation === undefined ? null : options.generation;
    }),
    deleteObject: vi.fn(async (_bucket: string, _object: string) => {
      calls.push("deleteObject");
    }),
  } satisfies ObjectStore;
  const signer = {
    signUpload: vi.fn(async (request: UploadUrlRequest) => {
      calls.push("signUpload");
      return {
        method: "PUT" as const,
        url: `https://signed.test/${request.bucket}/${request.object}`,
        headers: { "x-goog-content-length-range": `0,${request.maxBytes}` },
        expiresAt: 1793490600,
      };
    }),
  } satisfies UploadUrlSigner;
  const storage = createDocumentStorage({ buckets, store, signer, limits: { maxObjectBytes: MAX_OBJECT, maxUserTotalBytes: MAX_TOTAL } });
  return { storage, buckets, store, signer, calls };
}

const REQUEST = { documentId: DOCUMENT_ID, size: 400, version: 3 };

describe("issueUpload", () => {
  it("signs a link for the user's own bucket, for a new object, with the size and version asked for", async () => {
    const { storage, signer } = makeFixture();

    const upload = await storage.issueUpload(USER_ID, REQUEST);

    expect(signer.signUpload).toHaveBeenCalledExactlyOnceWith({
      bucket: bucketOf(USER_ID),
      object: DOCUMENT_ID,
      maxBytes: 400,
      ifGenerationMatch: "0",
      version: 3,
    });
    expect(upload).toEqual({
      method: "PUT",
      url: `https://signed.test/${bucketOf(USER_ID)}/${DOCUMENT_ID}`,
      headers: { "x-goog-content-length-range": "0,400" },
      expiresAt: 1793490600,
    });
  });

  it("ties the link to the object's current generation when it already exists", async () => {
    const { storage, signer, store } = makeFixture({ generation: "1730419200123456" });

    await storage.issueUpload(USER_ID, REQUEST);

    expect(store.currentGeneration).toHaveBeenCalledExactlyOnceWith(bucketOf(USER_ID), DOCUMENT_ID);
    expect(signer.signUpload).toHaveBeenCalledWith(expect.objectContaining({ ifGenerationMatch: "1730419200123456" }));
  });

  it("makes sure the bucket exists, then measures it, then looks up the generation, then signs", async () => {
    const { storage, calls } = makeFixture();
    await storage.issueUpload(USER_ID, REQUEST);
    expect(calls).toEqual(["ensureUserBucket", "totalBytes", "currentGeneration", "signUpload"]);
  });

  it("measures the bucket of the user who asked, not another one", async () => {
    const { storage, store } = makeFixture();
    await storage.issueUpload(OTHER_USER_ID, REQUEST);
    expect(store.totalBytes).toHaveBeenCalledExactlyOnceWith(bucketOf(OTHER_USER_ID));
  });

  describe("the size of one object", () => {
    it("signs an object exactly as large as the limit", async () => {
      const { storage, signer } = makeFixture();
      await storage.issueUpload(USER_ID, { ...REQUEST, size: MAX_OBJECT });
      expect(signer.signUpload).toHaveBeenCalledWith(expect.objectContaining({ maxBytes: MAX_OBJECT }));
    });

    it("refuses one byte more, and says what the limit is, before asking Google anything", async () => {
      const { storage, calls } = makeFixture();

      const error = await storage.issueUpload(USER_ID, { ...REQUEST, size: MAX_OBJECT + 1 }).catch((caught: unknown) => caught);

      expect(error).toBeInstanceOf(ObjectTooLargeError);
      expect((error as ObjectTooLargeError).limitBytes).toBe(MAX_OBJECT);
      expect(calls).toEqual([]);
    });
  });

  describe("the user's total", () => {
    it("signs when the new object brings the total exactly to the limit", async () => {
      const { storage, signer } = makeFixture({ usedBytes: MAX_TOTAL - 400 });
      await storage.issueUpload(USER_ID, REQUEST);
      expect(signer.signUpload).toHaveBeenCalledOnce();
    });

    it("refuses when it would go one byte over, says the limit and the use, and signs nothing", async () => {
      const { storage, signer, store } = makeFixture({ usedBytes: MAX_TOTAL - 399 });

      const error = await storage.issueUpload(USER_ID, REQUEST).catch((caught: unknown) => caught);

      expect(error).toBeInstanceOf(QuotaExceededError);
      expect((error as QuotaExceededError).limitBytes).toBe(MAX_TOTAL);
      expect((error as QuotaExceededError).usedBytes).toBe(MAX_TOTAL - 399);
      expect(signer.signUpload).not.toHaveBeenCalled();
      expect(store.currentGeneration).not.toHaveBeenCalled();
    });

    it("counts the new object in full even when it replaces an existing one, so the old version counts too", async () => {
      // 4700 in use includes the old version of this very object; the new one adds 400 more.
      const { storage, signer } = makeFixture({ usedBytes: 4700, generation: "5" });
      await expect(storage.issueUpload(USER_ID, REQUEST)).rejects.toBeInstanceOf(QuotaExceededError);
      expect(signer.signUpload).not.toHaveBeenCalled();
    });

    it("tells the two limits apart", async () => {
      const tooLarge = makeFixture();
      const overTotal = makeFixture({ usedBytes: MAX_TOTAL });
      const first = await tooLarge.storage.issueUpload(USER_ID, { ...REQUEST, size: MAX_OBJECT + 1 }).catch((caught: unknown) => caught);
      const second = await overTotal.storage.issueUpload(USER_ID, REQUEST).catch((caught: unknown) => caught);
      expect(first).not.toBeInstanceOf(QuotaExceededError);
      expect(second).not.toBeInstanceOf(ObjectTooLargeError);
    });
  });

  describe("a request that is not valid", () => {
    it.each([
      ["a document id with capital letters", { documentId: DOCUMENT_ID.toUpperCase() }],
      ["a document id that is too short", { documentId: DOCUMENT_ID.slice(1) }],
      ["a document id that is too long", { documentId: `${DOCUMENT_ID}0` }],
      ["a document id with a slash", { documentId: `../${DOCUMENT_ID.slice(3)}` }],
      ["a document id that is not text", { documentId: 12345 }],
      ["no document id", { documentId: undefined }],
      ["a size of zero", { size: 0 }],
      ["a negative size", { size: -1 }],
      ["a fractional size", { size: 1.5 }],
      ["a size written as text", { size: "400" }],
      ["a size that is not a number", { size: Number.NaN }],
      ["no size", { size: undefined }],
      ["a negative version", { version: -1 }],
      ["a fractional version", { version: 1.5 }],
      ["a version written as text", { version: "3" }],
      ["no version", { version: undefined }],
    ])("answers InvalidRequestError for %s, before asking Google anything", async (_label, change) => {
      const { storage, calls } = makeFixture();
      await expect(storage.issueUpload(USER_ID, { ...REQUEST, ...change })).rejects.toBeInstanceOf(InvalidRequestError);
      expect(calls).toEqual([]);
    });

    it("accepts version 0", async () => {
      const { storage, signer } = makeFixture();
      await storage.issueUpload(USER_ID, { ...REQUEST, version: 0 });
      expect(signer.signUpload).toHaveBeenCalledWith(expect.objectContaining({ version: 0 }));
    });
  });

  describe("when Google cannot answer", () => {
    it("fails, and signs nothing, when the total cannot be worked out", async () => {
      const { storage, store, signer } = makeFixture();
      store.totalBytes.mockRejectedValueOnce(new ListingTooLargeError("more than 20000 object versions"));
      await expect(storage.issueUpload(USER_ID, REQUEST)).rejects.toBeInstanceOf(ListingTooLargeError);
      expect(signer.signUpload).not.toHaveBeenCalled();
    });

    it("fails, and signs nothing, when the object's generation cannot be read", async () => {
      const { storage, store, signer } = makeFixture();
      store.currentGeneration.mockRejectedValueOnce(new GcsRequestError("read object", 500, "INTERNAL"));
      await expect(storage.issueUpload(USER_ID, REQUEST)).rejects.toBeInstanceOf(GcsRequestError);
      expect(signer.signUpload).not.toHaveBeenCalled();
    });

    it("fails, and measures nothing, when the bucket cannot be made sure of", async () => {
      const { storage, buckets, store } = makeFixture();
      buckets.ensureUserBucket.mockRejectedValueOnce(new GcsRequestError("create bucket", 503));
      await expect(storage.issueUpload(USER_ID, REQUEST)).rejects.toBeInstanceOf(GcsRequestError);
      expect(store.totalBytes).not.toHaveBeenCalled();
    });
  });
});

describe("deleteDocument", () => {
  it("deletes the object from the bucket that belongs to the signed-in user", async () => {
    const { storage, store } = makeFixture();

    await storage.deleteDocument(USER_ID, DOCUMENT_ID);

    expect(store.deleteObject).toHaveBeenCalledExactlyOnceWith(bucketOf(USER_ID), DOCUMENT_ID);
  });

  it("cannot reach another user's bucket: two users with the same document id touch their own buckets", async () => {
    const { storage, store } = makeFixture();

    await storage.deleteDocument(USER_ID, DOCUMENT_ID);
    await storage.deleteDocument(OTHER_USER_ID, DOCUMENT_ID);

    expect(store.deleteObject.mock.calls).toEqual([
      [bucketOf(USER_ID), DOCUMENT_ID],
      [bucketOf(OTHER_USER_ID), DOCUMENT_ID],
    ]);
  });

  it("does not create a bucket just to delete from it", async () => {
    const { storage, buckets } = makeFixture();
    await storage.deleteDocument(USER_ID, DOCUMENT_ID);
    expect(buckets.ensureUserBucket).not.toHaveBeenCalled();
  });

  it.each([
    ["a path that climbs out", "../other-bucket"],
    ["capital letters", DOCUMENT_ID.toUpperCase()],
    ["an empty id", ""],
    ["something that is not text", 42],
    ["nothing", undefined],
  ])("refuses %s before deleting anything", async (_label, documentId) => {
    const { storage, store } = makeFixture();
    await expect(storage.deleteDocument(USER_ID, documentId)).rejects.toBeInstanceOf(InvalidRequestError);
    expect(store.deleteObject).not.toHaveBeenCalled();
  });

  it("lets a failure from Google through, so the caller does not think the document is gone", async () => {
    const { storage, store } = makeFixture();
    store.deleteObject.mockRejectedValueOnce(new GcsRequestError("delete object", 500, "INTERNAL"));
    await expect(storage.deleteDocument(USER_ID, DOCUMENT_ID)).rejects.toBeInstanceOf(GcsRequestError);
  });
});
