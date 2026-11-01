import type { BucketService } from "./bucket-service";
import { InvalidRequestError, ObjectTooLargeError, QuotaExceededError } from "./errors";
import type { Limits } from "./limits";
import type { ObjectStore } from "./object-store";
import type { SignedUpload, UploadUrlSigner } from "./signed-upload-url";

// The object name is the document id: 32 lower-case hex characters.
const DOCUMENT_ID_PATTERN = /^[0-9a-f]{32}$/;

export type UploadRequest = {
  documentId: unknown;
  /** The number of bytes about to be uploaded. */
  size: unknown;
  /** The document's version number, stored on the object. */
  version: unknown;
};

export type DocumentStorage = {
  /**
   * Checks the size and the user's total, then signs a 3-minute link that can write this one
   * object, at most `size` bytes, once.
   */
  issueUpload(userId: string, request: UploadRequest): Promise<SignedUpload>;
  /** Deletes the document's object from the user's bucket. A document that is not there counts as deleted. */
  deleteDocument(userId: string, documentId: unknown): Promise<void>;
};

export type DocumentStorageDeps = {
  buckets: BucketService;
  store: ObjectStore;
  signer: UploadUrlSigner;
  limits: Limits;
};

function readDocumentId(value: unknown): string {
  if (typeof value !== "string" || !DOCUMENT_ID_PATTERN.test(value)) {
    throw new InvalidRequestError("documentId must be 32 lower-case hex characters");
  }
  return value;
}

function readWholeNumber(value: unknown, name: string, minimum: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum) {
    throw new InvalidRequestError(`${name} must be a whole number of at least ${minimum}`);
  }
  return value;
}

export function createDocumentStorage(deps: DocumentStorageDeps): DocumentStorage {
  const { buckets, store, signer, limits } = deps;

  return {
    async issueUpload(userId, request) {
      const documentId = readDocumentId(request.documentId);
      const size = readWholeNumber(request.size, "size", 1);
      const version = readWholeNumber(request.version, "version", 0);

      // Cheapest check first: it needs nothing from Google.
      if (size > limits.maxObjectBytes) throw new ObjectTooLargeError(limits.maxObjectBytes);

      const { bucket } = await buckets.ensureUserBucket(userId);
      const used = await store.totalBytes(bucket);
      if (used + size > limits.maxUserTotalBytes) throw new QuotaExceededError(limits.maxUserTotalBytes, used);

      // 0 means "must not exist yet"; otherwise the link only works while the object is unchanged.
      const generation = (await store.currentGeneration(bucket, documentId)) ?? "0";
      return signer.signUpload({ bucket, object: documentId, maxBytes: size, ifGenerationMatch: generation, version });
    },

    async deleteDocument(userId, documentId) {
      const id = readDocumentId(documentId);
      // The bucket comes from the signed-in user alone, so there is no way to name another user's.
      // Not ensureUserBucket: deleting must not create a bucket.
      await store.deleteObject(await buckets.nameFor(userId), id);
    },
  };
}
