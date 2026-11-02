import { describe, expect, it } from "vitest";
import { CLIENT_TOKEN_MIN_VALIDITY_MS, createClientTokenIssuer } from "../../src/gcs/client-token-issuer";
import { TokenExchangeError } from "../../src/gcs/errors";
import { jsonReply, makeFakeFetch, type RecordedRequest } from "./fake-fetch";

const STS_URL = "https://sts.googleapis.com/v1/token";
const START_MS = Date.parse("2026-11-01T00:00:00Z");
const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86";
const SOURCE_TOKEN = "source-token-secret";

function makeIssuer(respond: (request: RecordedRequest, clock: { now: number }) => Response = () => jsonReply(200, { access_token: "client-token", expires_in: 3574 })) {
  const clock = { now: START_MS };
  const asked: number[] = [];
  const fake = makeFakeFetch((request) => respond(request, clock));
  const issuer = createClientTokenIssuer({
    getAccessToken: async (minValidityMs) => {
      asked.push(minValidityMs);
      return SOURCE_TOKEN;
    },
    fetchFn: fake.fetchFn,
    now: () => clock.now,
  });
  return { issuer, requests: fake.requests, asked, clock };
}

describe("createClientTokenIssuer", () => {
  it("exchanges the source token for one limited to the user's bucket and to reading", async () => {
    const { issuer, requests } = makeIssuer();

    await issuer.issueReadOnlyToken(BUCKET);

    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe(STS_URL);
    expect(requests[0].method).toBe("POST");
    const form = new URLSearchParams(requests[0].body);
    expect(form.get("grant_type")).toBe("urn:ietf:params:oauth:grant-type:token-exchange");
    expect(form.get("requested_token_type")).toBe("urn:ietf:params:oauth:token-type:access_token");
    expect(form.get("subject_token_type")).toBe("urn:ietf:params:oauth:token-type:access_token");
    expect(form.get("subject_token")).toBe(SOURCE_TOKEN);
    // Google wants the options JSON encoded once on its own, on top of the form encoding.
    expect(JSON.parse(decodeURIComponent(form.get("options") ?? ""))).toEqual({
      accessBoundary: {
        accessBoundaryRules: [
          {
            availableResource: `//storage.googleapis.com/projects/_/buckets/${BUCKET}`,
            availablePermissions: ["inRole:roles/storage.objectViewer"],
          },
        ],
      },
    });
  });

  it("asks for a source token that lasts at least half an hour", async () => {
    const { issuer, asked } = makeIssuer();
    await issuer.issueReadOnlyToken(BUCKET);
    expect(asked).toEqual([CLIENT_TOKEN_MIN_VALIDITY_MS]);
    expect(CLIENT_TOKEN_MIN_VALIDITY_MS).toBe(30 * 60 * 1000);
  });

  it("returns the token with an expiry counted from before the request, so it is never too late", async () => {
    const { issuer } = makeIssuer((_request, clock) => {
      clock.now += 5000; // the exchange takes five seconds
      return jsonReply(200, { access_token: "client-token", expires_in: 3574 });
    });

    expect(await issuer.issueReadOnlyToken(BUCKET)).toEqual({
      accessToken: "client-token",
      expiresAt: START_MS / 1000 + 3574,
    });
  });

  it("reports the status and Google's error code, never the source token", async () => {
    const { issuer } = makeIssuer(() =>
      jsonReply(400, { error: "invalid_request", error_description: `bad token ${SOURCE_TOKEN}` }),
    );

    const failure = await issuer.issueReadOnlyToken(BUCKET).catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(TokenExchangeError);
    const message = (failure as Error).message;
    expect(message).toContain("HTTP 400");
    expect(message).toContain("invalid_request");
    expect(message).not.toContain(SOURCE_TOKEN);
  });

  it("reports the status when the answer is not JSON at all", async () => {
    const { issuer } = makeIssuer(() => new Response("<html>bad gateway</html>", { status: 502 }));
    await expect(issuer.issueReadOnlyToken(BUCKET)).rejects.toThrow("HTTP 502");
  });

  it.each([
    ["no access token", { expires_in: 3574 }],
    ["no lifetime", { access_token: "client-token" }],
    ["a lifetime of zero", { access_token: "client-token", expires_in: 0 }],
  ])("rejects an answer with %s", async (_label, body) => {
    const { issuer } = makeIssuer(() => jsonReply(200, body));
    await expect(issuer.issueReadOnlyToken(BUCKET)).rejects.toBeInstanceOf(TokenExchangeError);
  });

  it.each([["empty", ""], ["upper case", "Spark-ABC"], ["with a slash", "spark-a/b"], ["with dots up", "../other"], ["too long", "a".repeat(64)]])(
    "refuses a bucket name that is %s, before asking Google for anything",
    async (_label, bucket) => {
      const { issuer, requests, asked } = makeIssuer();
      await expect(issuer.issueReadOnlyToken(bucket)).rejects.toBeInstanceOf(TokenExchangeError);
      expect(requests).toHaveLength(0);
      expect(asked).toHaveLength(0);
    },
  );
});
