import { readBearerToken, verifyAccessToken, type TokenVerifier } from "./access-token";
import type { BucketService } from "./gcs/bucket-service";
import { BucketNameTakenError } from "./gcs/errors";

export type Services = {
  verifier: TokenVerifier;
  buckets: BucketService;
};

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

async function handleEnsureBucket(userId: string, buckets: BucketService): Promise<Response> {
  try {
    return makeJsonResponse(await buckets.ensureUserBucket(userId), 200);
  } catch (error) {
    logFailure("bucket check failed", error);
    if (error instanceof BucketNameTakenError) {
      return makeJsonResponse({ error: "bucket_name_taken" }, 409);
    }
    return makeJsonResponse({ error: "storage_unavailable" }, 503);
  }
}

/** Every request needs a valid Supabase session token; the routes behind it come in later issues. */
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
  if (request.method === "POST" && pathname === "/bucket") {
    return handleEnsureBucket(user.userId, services.buckets);
  }
  return makeJsonResponse({ error: "not_found" }, 404);
}
