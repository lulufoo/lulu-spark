import { SignJWT, base64url, errors, type JWTVerifyGetKey } from "jose";
import { describe, expect, it } from "vitest";
import { readBearerToken, verifyAccessToken } from "../src/access-token";
import { ISSUER, USER_ID, makeSigner } from "./signing-fixture";

function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

/** The same token with a different payload and the original signature. */
function tamperWithPayload(token: string): string {
  const [header, payload, signature] = token.split(".");
  const claims = JSON.parse(new TextDecoder().decode(base64url.decode(payload)));
  const forged = base64url.encode(JSON.stringify({ ...claims, sub: "attacker" }));
  return `${header}.${forged}.${signature}`;
}

describe("verifyAccessToken", () => {
  it("returns the user id of a valid token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toEqual({
      userId: USER_ID,
    });
  });

  it("rejects an expired token", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken({ expiresAt: nowSeconds() - 60 });
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rejects a token whose payload was changed after signing", async () => {
    const signer = await makeSigner();
    const token = tamperWithPayload(await signer.signToken());
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rejects a token signed by another key", async () => {
    const trusted = await makeSigner();
    const stranger = await makeSigner();
    const token = await stranger.signToken();
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: trusted.keys })).toBeNull();
  });

  it("rejects a token from another issuer", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken({ issuer: "https://other.supabase.co/auth/v1" });
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rejects a token for another audience, such as the anon key", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken({ audience: "anon" });
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rejects a token without a subject", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken({ subject: null });
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rejects an HS256 token", async () => {
    const signer = await makeSigner();
    const token = await new SignJWT({ role: "authenticated" })
      .setProtectedHeader({ alg: "HS256" })
      .setIssuer(ISSUER)
      .setAudience("authenticated")
      .setSubject(USER_ID)
      .setExpirationTime("1h")
      .sign(new TextEncoder().encode("a-shared-secret-of-at-least-32-bytes"));
    expect(await verifyAccessToken(token, { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rejects text that is not a token", async () => {
    const signer = await makeSigner();
    expect(await verifyAccessToken("not-a-jwt", { issuer: ISSUER, keys: signer.keys })).toBeNull();
  });

  it("rethrows when the keys cannot be fetched, instead of calling the token bad", async () => {
    const signer = await makeSigner();
    const token = await signer.signToken();
    const networkDown: JWTVerifyGetKey = async () => {
      throw new TypeError("fetch failed");
    };
    const timedOut: JWTVerifyGetKey = async () => {
      throw new errors.JWKSTimeout();
    };
    await expect(verifyAccessToken(token, { issuer: ISSUER, keys: networkDown })).rejects.toThrow(
      "fetch failed",
    );
    await expect(verifyAccessToken(token, { issuer: ISSUER, keys: timedOut })).rejects.toBeInstanceOf(
      errors.JWKSTimeout,
    );
  });
});

describe("readBearerToken", () => {
  function requestWith(authorization?: string): Request {
    return new Request("https://worker.test/", {
      headers: authorization === undefined ? {} : { Authorization: authorization },
    });
  }

  it("reads the token after Bearer", () => {
    expect(readBearerToken(requestWith("Bearer abc.def.ghi"))).toBe("abc.def.ghi");
  });

  it("accepts the scheme in any letter case", () => {
    expect(readBearerToken(requestWith("bearer abc"))).toBe("abc");
  });

  it("returns null without a header", () => {
    expect(readBearerToken(requestWith())).toBeNull();
  });

  it("returns null for another scheme", () => {
    expect(readBearerToken(requestWith("Basic abc"))).toBeNull();
  });

  it("returns null when Bearer has no token", () => {
    expect(readBearerToken(requestWith("Bearer"))).toBeNull();
  });
});
