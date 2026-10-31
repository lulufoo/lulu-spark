import { readBearerToken, verifyAccessToken, type TokenVerifier } from "./access-token";

function makeJsonResponse(body: unknown, status: number, headers: HeadersInit = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...headers },
  });
}

/** Every request needs a valid Supabase session token; the routes behind it come in later issues. */
export async function handleRequest(request: Request, verifier: TokenVerifier): Promise<Response> {
  const token = readBearerToken(request);

  let user;
  try {
    user = token ? await verifyAccessToken(token, verifier) : null;
  } catch (error) {
    console.error(
      JSON.stringify({
        message: "token check failed",
        error: error instanceof Error ? error.message : String(error),
      }),
    );
    return makeJsonResponse({ error: "auth_unavailable" }, 503);
  }
  if (!user) {
    return makeJsonResponse({ error: "unauthorized" }, 401, { "WWW-Authenticate": "Bearer" });
  }

  const { pathname } = new URL(request.url);
  if (request.method === "GET" && pathname === "/whoami") {
    return makeJsonResponse({ userId: user.userId }, 200);
  }
  return makeJsonResponse({ error: "not_found" }, 404);
}
