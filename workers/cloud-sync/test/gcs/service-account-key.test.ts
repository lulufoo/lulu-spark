import { describe, expect, it } from "vitest";
import { InvalidServiceAccountKeyError } from "../../src/gcs/errors";
import { parseServiceAccountKey } from "../../src/gcs/service-account-key";

const PEM = "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----\n";

function makeKeyJson(fields: Record<string, unknown>): string {
  return JSON.stringify({ client_email: "a@b.iam.gserviceaccount.com", private_key: PEM, ...fields });
}

describe("parseServiceAccountKey", () => {
  it("reads the fields it needs", () => {
    const key = parseServiceAccountKey(makeKeyJson({ token_uri: "https://oauth2.test/token" }));
    expect(key).toEqual({
      clientEmail: "a@b.iam.gserviceaccount.com",
      privateKey: PEM,
      tokenUri: "https://oauth2.test/token",
    });
  });

  it("falls back to Google's token endpoint", () => {
    expect(parseServiceAccountKey(makeKeyJson({})).tokenUri).toBe("https://oauth2.googleapis.com/token");
  });

  it("rejects a missing secret", () => {
    expect(() => parseServiceAccountKey(undefined)).toThrow(InvalidServiceAccountKeyError);
  });

  it("rejects text that is not JSON, without quoting it", () => {
    const secretLooking = "{private_key: MIIEvQIBADANBgkqhkiG9w0BAQEFAASC";
    let message = "";
    try {
      parseServiceAccountKey(secretLooking);
    } catch (error) {
      message = (error as Error).message;
    }
    expect(message).not.toBe("");
    expect(message).not.toContain("MIIEvQ");
  });

  it("rejects a key without client_email", () => {
    expect(() => parseServiceAccountKey(makeKeyJson({ client_email: undefined }))).toThrow(
      /client_email/,
    );
  });

  it("rejects a private_key that is not PKCS#8 PEM", () => {
    expect(() => parseServiceAccountKey(makeKeyJson({ private_key: "nope" }))).toThrow(/private_key/);
  });
});
