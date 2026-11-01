import { readBearerToken, verifyAccessToken, type TokenVerifier } from "./access-token";
import type { CredentialsService } from "./gcs/credentials-service";
import type { DocumentStorage } from "./gcs/document-storage";
import { BucketNameTakenError, InvalidRequestError, ObjectTooLargeError, QuotaExceededError } from "./gcs/errors";

export type Services = {
  verifier: TokenVerifier;
  credentials: CredentialsService;
  documents: DocumentStorage;
};

// A real upload request is under 200 bytes. Anything near this is not one.
const MAX_REQUEST_BODY_BYTES = 4096;

function makeJsonResponse(body: unknown, status: number, headers: HeadersInit = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...headers },
  });
}

/** One structured line. Errors from our own modules never contain keys or tokens. */
function logFailure(message: string, error: unknown): void {
  console.error(
    JSON.stringify({
      message,
      errorName: error instanceof Error ? error.name : "unknown",
      error: error instanceof Error ? error.message : String(error),
    }),
  );
}

/**
 * Reads a small JSON object from the request. It stops reading at the limit instead of trusting
 * Content-Length, which a caller can set to anything.
 */
async function readJsonObject(request: Request): Promise<Record<string, unknown>> {
  const reader = request.body?.getReader();
  if (!reader) throw new InvalidRequestError("a JSON body is required");

  const chunks: Uint8Array[] = [];
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    received += value.byteLength;
    if (received > MAX_REQUEST_BODY_BYTES) {
      await reader.cancel();
      throw new InvalidRequestError("the request body is too large");
    }
    chunks.push(value);
  }

  const bytes = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw new InvalidRequestError("the request body is not valid JSON");
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new InvalidRequestError("the request body must be a JSON object");
  }
  return parsed as Record<string, unknown>;
}

/** Turns a failure into a response. Answers about the caller's own request are not logged as failures. */
function respondToFailure(operation: string, error: unknown): Response {
  if (error instanceof InvalidRequestError) {
    return makeJsonResponse({ error: "invalid_request", detail: error.message }, 400);
  }
  // 413 for both limits, told apart by `error`, so a client can say which limit it hit.
  if (error instanceof ObjectTooLargeError) {
    return makeJsonResponse({ error: "object_too_large", limitBytes: error.limitBytes }, 413);
  }
  if (error instanceof QuotaExceededError) {
    return makeJsonResponse(
      { error: "quota_exceeded", limitBytes: error.limitBytes, usedBytes: error.usedBytes },
      413,
    );
  }
  logFailure(`${operation} failed`, error);
  if (error instanceof BucketNameTakenError) {
    return makeJsonResponse({ error: "bucket_name_taken" }, 409);
  }
  return makeJsonResponse({ error: "storage_unavailable" }, 503);
}

async function handleIssueCredentials(userId: string, credentials: CredentialsService): Promise<Response> {
  try {
    // A token response must never be stored by a cache (RFC 6749, section 5.1).
    return makeJsonResponse(await credentials.issueUserCredentials(userId), 200, { "Cache-Control": "no-store" });
  } catch (error) {
    return respondToFailure("issuing credentials", error);
  }
}

async function handleIssueUpload(userId: string, request: Request, documents: DocumentStorage): Promise<Response> {
  try {
    const body = await readJsonObject(request);
    const upload = await documents.issueUpload(userId, {
      documentId: body.documentId,
      size: body.size,
      version: body.version,
    });
    // The link is a credential for its few minutes of life.
    return makeJsonResponse(upload, 200, { "Cache-Control": "no-store" });
  } catch (error) {
    return respondToFailure("issuing an upload link", error);
  }
}

async function handleDeleteDocument(userId: string, documentId: string, documents: DocumentStorage): Promise<Response> {
  try {
    await documents.deleteDocument(userId, documentId);
    return new Response(null, { status: 204 });
  } catch (error) {
    return respondToFailure("deleting a document", error);
  }
}

/** A path piece such as `/documents/<id>`. Anything that does not decode is passed on as it is and rejected later. */
function readDocumentIdFromPath(pathname: string): string | null {
  const match = /^\/documents\/([^/]+)$/.exec(pathname);
  if (!match?.[1]) return null;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return match[1];
  }
}

/** Every request needs a valid Supabase session token. Whose bucket is touched comes from that token alone. */
export async function handleRequest(request: Request, services: Services): Promise<Response> {
  const token = readBearerToken(request);

  let user;
  try {
    user = token ? await verifyAccessToken(token, services.verifier) : null;
  } catch (error) {
    logFailure("token check failed", error);
    return makeJsonResponse({ error: "auth_unavailable" }, 503);
  }
  if (!user) {
    return makeJsonResponse({ error: "unauthorized" }, 401, { "WWW-Authenticate": "Bearer" });
  }

  const { pathname } = new URL(request.url);
  if (request.method === "GET" && pathname === "/whoami") {
    return makeJsonResponse({ userId: user.userId }, 200);
  }
  if (request.method === "POST" && pathname === "/credentials") {
    return handleIssueCredentials(user.userId, services.credentials);
  }
  if (request.method === "POST" && pathname === "/uploads") {
    return handleIssueUpload(user.userId, request, services.documents);
  }
  if (request.method === "DELETE") {
    const documentId = readDocumentIdFromPath(pathname);
    if (documentId !== null) return handleDeleteDocument(user.userId, documentId, services.documents);
  }
  return makeJsonResponse({ error: "not_found" }, 404);
}
