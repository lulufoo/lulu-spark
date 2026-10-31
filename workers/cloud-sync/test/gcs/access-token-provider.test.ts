import { decodeProtectedHeader, jwtVerify } from "jose";
import { describe, expect, it } from "vitest";
import { createAccessTokenProvider } from "../../src/gcs/access-token-provider";
import { AccessTokenError } from "../../src/gcs/errors";
import { parseServiceAccountKey } from "../../src/gcs/service-account-key";
import { jsonReply, makeFakeFetch } from "./fake-fetch";
import { CLIENT_EMAIL, TOKEN_URI, makeServiceAccount } from "./service-account-fixture";

const START_MS = Date.parse("2026-11-01T00:00:00Z");
const MINUTE_MS = 60 * 1000;

async function makeProvider(respond = () => jsonReply(200, { access_token: "tok-1", expires_in: 3600 })) {
  const account = await makeServiceAccount();
  const clock = { now: START_MS };
  const fake = makeFakeFetch(respond);
  const provider = createAccessTokenProvider({
    key: parseServiceAccountKey(account.keyJson),
    fetchFn: fake.fetchFn,
    now: () => clock.now,
  });
  return { account, clock, provider, requests: fake.requests };
}

describe("createAccessTokenProvider", () => {
  it("asks Google's token endpoint with a signed JWT bearer grant", async () => {
    const { account, provider, requests } = await makeProvider();

    expect(await provider.getAccessToken()).toBe("tok-1");

    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe(TOKEN_URI);
    expect(requests[0].method).toBe("POST");
    const form = new URLSearchParams(requests[0].body);
    expect(form.get("grant_type")).toBe("urn:ietf:params:oauth:grant-type:jwt-bearer");

    const assertion = form.get("assertion") ?? "";
    expect(decodeProtectedHeader(assertion)).toMatchObject({ alg: "RS256", typ: "JWT" });
    const { payload } = await jwtVerify(assertion, account.publicKey, {
      issuer: CLIENT_EMAIL,
      audience: TOKEN_URI,
      currentDate: new Date(START_MS),
    });
    expect(payload.scope).toBe("https://www.googleapis.com/auth/cloud-platform");
    expect(payload.iat).toBe(START_MS / 1000);
    expect((payload.exp ?? 0) - (payload.iat ?? 0)).toBe(3600);
  });

  it("reuses the token until five minutes before it expires", async () => {
    const { clock, provider, requests } = await makeProvider();
    await provider.getAccessToken();

    clock.now = START_MS + 54 * MINUTE_MS;
    await provider.getAccessToken();
    expect(requests).toHaveLength(1);

    clock.now = START_MS + 56 * MINUTE_MS;
    await provider.getAccessToken();
    expect(requests).toHaveLength(2);
  });

  it("shares one request between callers that arrive together", async () => {
    const { provider, requests } = await makeProvider();
    const tokens = await Promise.all([provider.getAccessToken(), provider.getAccessToken()]);
    expect(tokens).toEqual(["tok-1", "tok-1"]);
    expect(requests).toHaveLength(1);
  });

  it("reports Google's error code without echoing the assertion, and tries again next time", async () => {
    let attempts = 0;
    const { provider, requests } = await makeProvider(() => {
      attempts += 1;
      return attempts === 1
        ? jsonReply(400, { error: "invalid_grant", error_description: "Invalid JWT Signature." })
        : jsonReply(200, { access_token: "tok-2", expires_in: 3600 });
    });

    const failure = await provider.getAccessToken().catch((error: unknown) => error);
    expect(failure).toBeInstanceOf(AccessTokenError);
    const message = (failure as Error).message;
    expect(message).toContain("HTTP 400");
    expect(message).toContain("invalid_grant");
    expect(message).not.toContain(new URLSearchParams(requests[0].body).get("assertion") ?? "?");

    expect(await provider.getAccessToken()).toBe("tok-2");
    expect(requests).toHaveLength(2);
  });

  it("rejects an answer that has no access token", async () => {
    const { provider } = await makeProvider(() => jsonReply(200, { expires_in: 3600 }));
    await expect(provider.getAccessToken()).rejects.toBeInstanceOf(AccessTokenError);
  });
});
