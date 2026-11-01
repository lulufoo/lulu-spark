import type { JWTVerifyGetKey } from "jose";
import { describe, expect, it, vi } from "vitest";
import type { CredentialsService, UserCredentials } from "../src/gcs/credentials-service";
import type { DocumentList, DocumentStorage } from "../src/gcs/document-storage";
import {
  BucketNameTakenError,
  GcsRequestError,
  InvalidRequestError,
  ObjectTooLargeError,
  QuotaExceededError,
  TokenExchangeError,
} from "../src/gcs/errors";
import type { SignedUpload } from "../src/gcs/signed-upload-url";
import { handleRequest, type Services } from "../src/handler";
import { ISSUER, USER_ID, makeSigner } from "./signing-fixture";

function makeRequest(path: string, token?: string, method = "GET", body?: BodyInit): Request {
  return new Request(`https://worker.test${path}`, {
    method,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body,
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

const UPLOAD: SignedUpload = {
  method: "PUT",
  url: "https://storage.googleapis.com/spark-dee0434c91366b18561ffdcad4380e86/0123456789abcdef0123456789abcdef?X-Goog-Signature=abc",
  headers: {
    "x-goog-content-length-range": "0,1024",
    "x-goog-if-generation-match": "0",
    "x-goog-meta-version": "3",
  },
  expiresAt: 1793490180,
};
const DOCUMENT_ID = "0123456789abcdef0123456789abcdef";

function makeDocuments(overrides: Partial<DocumentStorage> = {}) {
  const issueUpload = vi.fn(overrides.issueUpload ?? (async () => UPLOAD));
  const deleteDocument = vi.fn(overrides.deleteDocument ?? (async () => {}));
  const listDocuments = vi.fn(overrides.listDocuments ?? (async (): Promise<DocumentList> => ({ documents: [], skippedObjects: 0 })));
  return { documents: { issueUpload, deleteDocument, listDocuments } satisfies DocumentStorage, issueUpload, deleteDocument, listDocuments };
}

function makeServices(keys: JWTVerifyGetKey, credentials?: CredentialsService, documents?: DocumentStorage): Services {
  const refuse = () => {
    throw new Error("storage must not be touched by this request");
  };
  return {
    verifier: { issuer: ISSUER, keys },
    credentials: credentials ?? makeCredentials(async () => refuse()).credentials,
    documents: documents ?? { issueUpload: async () => refuse(), deleteDocument: async () => refuse(), listDocuments: async () => refuse() },
  };
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

function uploadBody(fields: Record<string, unknown> = {}): string {
  return JSON.stringify({ documentId: DOCUMENT_ID, size: 1024, version: 3, ...fields });
}

describe("POST /uploads", () => {
  it("signs a link for the signed-in user and returns it uncached", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, issueUpload } = makeDocuments();

    const response = await handleRequest(
      makeRequest("/uploads", token, "POST", uploadBody()),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual(UPLOAD);
    expect(response.headers.get("Cache-Control")).toBe("no-store");
    expect(issueUpload).toHaveBeenCalledExactlyOnceWith(USER_ID, { documentId: DOCUMENT_ID, size: 1024, version: 3 });
  });

  it("takes the user from the token alone, whatever else the body names", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, issueUpload } = makeDocuments();
    const hostile = uploadBody({ userId: "someone-else", user: "someone-else", bucket: "spark-someone-else", object: "other" });

    const response = await handleRequest(
      makeRequest("/uploads", token, "POST", hostile),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(200);
    expect(issueUpload).toHaveBeenCalledExactlyOnceWith(USER_ID, { documentId: DOCUMENT_ID, size: 1024, version: 3 });
  });

  it("answers 401 without touching storage when there is no token", async () => {
    const signer = await makeSigner();
    const { documents, issueUpload } = makeDocuments();

    const response = await handleRequest(
      makeRequest("/uploads", undefined, "POST", uploadBody()),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(401);
    expect(issueUpload).not.toHaveBeenCalled();
  });

  it("serves POST only", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, issueUpload } = makeDocuments();

    const response = await handleRequest(makeRequest("/uploads", token, "GET"), makeServices(signer.keys, undefined, documents));

    expect(response.status).toBe(404);
    expect(issueUpload).not.toHaveBeenCalled();
  });

  it.each([
    ["no body", undefined],
    ["a body that is not JSON", "not json"],
    ["a JSON array", "[1,2]"],
    ["JSON null", "null"],
    ["a body over 4 KiB", JSON.stringify({ documentId: DOCUMENT_ID, size: 1, version: 0, padding: "x".repeat(5000) })],
  ])("answers 400 for %s, without touching storage", async (_label, body) => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, issueUpload } = makeDocuments();

    const response = await handleRequest(makeRequest("/uploads", token, "POST", body), makeServices(signer.keys, undefined, documents));

    expect(response.status).toBe(400);
    expect(await response.json()).toMatchObject({ error: "invalid_request" });
    expect(issueUpload).not.toHaveBeenCalled();
  });

  it("answers 400 with the reason when the request values are refused", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents } = makeDocuments({
      issueUpload: async () => {
        throw new InvalidRequestError("size must be a whole number of at least 1");
      },
    });

    const response = await handleRequest(
      makeRequest("/uploads", token, "POST", uploadBody({ size: 0 })),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(400);
    expect(await response.json()).toEqual({ error: "invalid_request", detail: "size must be a whole number of at least 1" });
  });

  it("answers 413 object_too_large and 413 quota_exceeded, and tells the two apart", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});

    const tooLarge = await handleRequest(
      makeRequest("/uploads", token, "POST", uploadBody()),
      makeServices(signer.keys, undefined, makeDocuments({ issueUpload: async () => { throw new ObjectTooLargeError(100); } }).documents),
    );
    const overQuota = await handleRequest(
      makeRequest("/uploads", token, "POST", uploadBody()),
      makeServices(signer.keys, undefined, makeDocuments({ issueUpload: async () => { throw new QuotaExceededError(500, 450); } }).documents),
    );

    expect(tooLarge.status).toBe(413);
    expect(await tooLarge.json()).toEqual({ error: "object_too_large", limitBytes: 100 });
    expect(overQuota.status).toBe(413);
    expect(await overQuota.json()).toEqual({ error: "quota_exceeded", limitBytes: 500, usedBytes: 450 });
    expect(logged).not.toHaveBeenCalled(); // answers about the caller's own request are not failures
    logged.mockRestore();
  });

  it("answers 503 and logs when storage fails", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    const { documents } = makeDocuments({
      issueUpload: async () => {
        throw new GcsRequestError("list objects", 500, "backend error");
      },
    });

    const response = await handleRequest(makeRequest("/uploads", token, "POST", uploadBody()), makeServices(signer.keys, undefined, documents));

    expect(response.status).toBe(503);
    expect(await response.json()).toEqual({ error: "storage_unavailable" });
    expect(logged).toHaveBeenCalledOnce();
    expect(String(logged.mock.calls[0][0])).not.toContain(token);
    logged.mockRestore();
  });
});

describe("DELETE /documents/<id>", () => {
  it("deletes the signed-in user's document and answers 204", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, deleteDocument } = makeDocuments();

    const response = await handleRequest(
      makeRequest(`/documents/${DOCUMENT_ID}`, token, "DELETE"),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(204);
    expect(await response.text()).toBe("");
    expect(deleteDocument).toHaveBeenCalledExactlyOnceWith(USER_ID, DOCUMENT_ID);
  });

  it("answers 401 without touching storage when there is no token", async () => {
    const signer = await makeSigner();
    const { documents, deleteDocument } = makeDocuments();

    const response = await handleRequest(
      makeRequest(`/documents/${DOCUMENT_ID}`, undefined, "DELETE"),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(401);
    expect(deleteDocument).not.toHaveBeenCalled();
  });

  it("serves DELETE only", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, deleteDocument } = makeDocuments();

    const response = await handleRequest(
      makeRequest(`/documents/${DOCUMENT_ID}`, token, "GET"),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(404);
    expect(deleteDocument).not.toHaveBeenCalled();
  });

  it("answers 404 for a path that is not exactly /documents/<id>", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, deleteDocument } = makeDocuments();

    for (const path of ["/documents", "/documents/", `/documents/${DOCUMENT_ID}/more`]) {
      const response = await handleRequest(makeRequest(path, token, "DELETE"), makeServices(signer.keys, undefined, documents));
      expect(response.status).toBe(404);
    }
    expect(deleteDocument).not.toHaveBeenCalled();
  });

  it("passes whatever sits in the path on, so the service can refuse it with 400", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, deleteDocument } = makeDocuments({
      deleteDocument: async () => {
        throw new InvalidRequestError("documentId must be 32 lower-case hex characters");
      },
    });

    const response = await handleRequest(
      makeRequest("/documents/..%2Fother-bucket", token, "DELETE"),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(400);
    expect(deleteDocument).toHaveBeenCalledExactlyOnceWith(USER_ID, "../other-bucket");
  });

  it("answers 503 and logs when storage fails", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    const { documents } = makeDocuments({
      deleteDocument: async () => {
        throw new GcsRequestError("delete object", 500, "backend error");
      },
    });

    const response = await handleRequest(
      makeRequest(`/documents/${DOCUMENT_ID}`, token, "DELETE"),
      makeServices(signer.keys, undefined, documents),
    );

    expect(response.status).toBe(503);
    expect(logged).toHaveBeenCalledOnce();
    logged.mockRestore();
  });
});

describe("GET /documents", () => {
  const LIST: DocumentList = {
    documents: [
      { documentId: "a".repeat(32), version: 3 },
      { documentId: "b".repeat(32), version: 12 },
    ],
    skippedObjects: 0,
  };

  it("returns the signed-in user's documents with their versions, uncached", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, listDocuments } = makeDocuments({ listDocuments: async () => LIST });

    const response = await handleRequest(makeRequest("/documents", token), makeServices(signer.keys, undefined, documents));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ documents: LIST.documents });
    expect(response.headers.get("Cache-Control")).toBe("no-store");
    expect(listDocuments).toHaveBeenCalledExactlyOnceWith(USER_ID);
  });

  it("returns an empty list, not an error, for a user with no documents", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();

    const response = await handleRequest(makeRequest("/documents", token), makeServices(signer.keys, undefined, makeDocuments().documents));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ documents: [] });
  });

  it("answers 401 without touching storage when there is no token, a bad token or a tampered one", async () => {
    const signer = await makeSigner();
    const good = await signer.signToken();
    const [head, payload, signature] = good.split(".");
    const tampered = [head, `${payload.slice(0, 10)}${payload[10] === "A" ? "B" : "A"}${payload.slice(11)}`, signature].join(".");
    const { documents, listDocuments } = makeDocuments();

    for (const token of [undefined, "abc", tampered]) {
      const response = await handleRequest(makeRequest("/documents", token), makeServices(signer.keys, undefined, documents));
      expect(response.status).toBe(401);
    }
    expect(listDocuments).not.toHaveBeenCalled();
  });

  it("serves GET only on /documents, and DELETE only on /documents/<id>", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const { documents, listDocuments, deleteDocument } = makeDocuments();
    const services = makeServices(signer.keys, undefined, documents);

    expect((await handleRequest(makeRequest("/documents", token, "POST", "{}"), services)).status).toBe(404);
    expect((await handleRequest(makeRequest("/documents", token, "DELETE"), services)).status).toBe(404);
    expect((await handleRequest(makeRequest(`/documents/${DOCUMENT_ID}`, token, "GET"), services)).status).toBe(404);
    expect((await handleRequest(makeRequest("/documents/", token, "GET"), services)).status).toBe(404);
    expect(listDocuments).not.toHaveBeenCalled();
    expect(deleteDocument).not.toHaveBeenCalled();
  });

  it("logs only a count, and keeps it out of the response, when objects were left out of the list", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const warned = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { documents } = makeDocuments({ listDocuments: async () => ({ ...LIST, skippedObjects: 2 }) });

    const response = await handleRequest(makeRequest("/documents", token), makeServices(signer.keys, undefined, documents));

    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ documents: LIST.documents });
    expect(warned).toHaveBeenCalledOnce();
    const line = String(warned.mock.calls[0][0]);
    expect(JSON.parse(line)).toMatchObject({ skippedObjects: 2 });
    expect(line).not.toContain("a".repeat(32));
    warned.mockRestore();
  });

  it("does not log when nothing was left out", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const warned = vi.spyOn(console, "warn").mockImplementation(() => {});

    await handleRequest(makeRequest("/documents", token), makeServices(signer.keys, undefined, makeDocuments({ listDocuments: async () => LIST }).documents));

    expect(warned).not.toHaveBeenCalled();
    warned.mockRestore();
  });

  it("answers 503 and logs when storage fails, rather than an empty list", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const logged = vi.spyOn(console, "error").mockImplementation(() => {});
    const { documents } = makeDocuments({
      listDocuments: async () => {
        throw new GcsRequestError("list objects", 500, "INTERNAL");
      },
    });

    const response = await handleRequest(makeRequest("/documents", token), makeServices(signer.keys, undefined, documents));

    expect(response.status).toBe(503);
    expect(await response.json()).toEqual({ error: "storage_unavailable" });
    expect(logged).toHaveBeenCalledOnce();
    expect(String(logged.mock.calls[0][0])).not.toContain(token);
    logged.mockRestore();
  });
});
