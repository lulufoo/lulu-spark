import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { parseServiceAccountKey } from "../../src/gcs/service-account-key";
import { createUploadUrlSigner, type UploadUrlRequest } from "../../src/gcs/signed-upload-url";
import { CLIENT_EMAIL, makeServiceAccount } from "./service-account-fixture";

const START_MS = Date.parse("2026-11-01T00:00:00Z");
const LIFETIME_SECONDS = 600;
const REQUEST: UploadUrlRequest = {
  bucket: "spark-dee0434c91366b18561ffdcad4380e86",
  object: "0123456789abcdef0123456789abcdef",
  maxBytes: 1234,
  ifGenerationMatch: "0",
  version: 7,
};

async function makeSigner(lifetimeSeconds = LIFETIME_SECONDS) {
  const account = await makeServiceAccount();
  const signer = createUploadUrlSigner({
    key: parseServiceAccountKey(account.keyJson),
    lifetimeSeconds,
    now: () => START_MS,
  });
  return { account, signer };
}

function readSignature(url: string): { signature: Uint8Array<ArrayBuffer>; urlWithoutSignature: string } {
  const [urlWithoutSignature, signatureHex] = url.split("&X-Goog-Signature=");
  const signature = Uint8Array.from(signatureHex.match(/../g) ?? [], (pair) => Number.parseInt(pair, 16));
  return { signature, urlWithoutSignature };
}

describe("createUploadUrlSigner", () => {
  it("builds the link and signs the string Google's V4 process describes", async () => {
    const { account, signer } = await makeSigner();

    const { url } = await signer.signUpload(REQUEST);

    // Written out by hand from https://cloud.google.com/storage/docs/authentication/canonical-requests
    const query =
      "X-Goog-Algorithm=GOOG4-RSA-SHA256" +
      `&X-Goog-Credential=${encodeURIComponent(CLIENT_EMAIL)}%2F20261101%2Fauto%2Fstorage%2Fgoog4_request` +
      "&X-Goog-Date=20261101T000000Z" +
      "&X-Goog-Expires=600" +
      "&X-Goog-SignedHeaders=host%3Bx-goog-content-length-range%3Bx-goog-if-generation-match%3Bx-goog-meta-version";
    const canonicalRequest = [
      "PUT",
      `/${REQUEST.bucket}/${REQUEST.object}`,
      query,
      "host:storage.googleapis.com",
      "x-goog-content-length-range:0,1234",
      "x-goog-if-generation-match:0",
      "x-goog-meta-version:7",
      "",
      "host;x-goog-content-length-range;x-goog-if-generation-match;x-goog-meta-version",
      "UNSIGNED-PAYLOAD",
    ].join("\n");
    const stringToSign = [
      "GOOG4-RSA-SHA256",
      "20261101T000000Z",
      "20261101/auto/storage/goog4_request",
      createHash("sha256").update(canonicalRequest).digest("hex"),
    ].join("\n");

    const { signature, urlWithoutSignature } = readSignature(url);
    expect(urlWithoutSignature).toBe(`https://storage.googleapis.com/${REQUEST.bucket}/${REQUEST.object}?${query}`);
    const valid = await crypto.subtle.verify(
      "RSASSA-PKCS1-v1_5",
      account.publicKey,
      signature,
      new TextEncoder().encode(stringToSign),
    );
    expect(valid).toBe(true);
  });

  it("returns the headers the upload must carry, and when the link stops working", async () => {
    const { signer } = await makeSigner();

    const signed = await signer.signUpload(REQUEST);

    expect(signed.headers).toEqual({
      "x-goog-content-length-range": "0,1234",
      "x-goog-if-generation-match": "0",
      "x-goog-meta-version": "7",
    });
    expect(signed.method).toBe("PUT");
    expect(signed.expiresAt).toBe(START_MS / 1000 + LIFETIME_SECONDS);
  });

  it("uses the lifetime it was given, both in what is signed and in the expiry it reports", async () => {
    const { signer } = await makeSigner(90);

    const signed = await signer.signUpload(REQUEST);

    expect(signed.url).toContain("&X-Goog-Expires=90&");
    expect(signed.expiresAt).toBe(START_MS / 1000 + 90);
  });

  it.each([0, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY])("refuses a lifetime of %s when it is created", async (lifetimeSeconds) => {
    const account = await makeServiceAccount();
    expect(() => createUploadUrlSigner({ key: parseServiceAccountKey(account.keyJson), lifetimeSeconds })).toThrow(RangeError);
  });

  it("signs the generation of an existing object, which can be larger than a JavaScript safe integer", async () => {
    const { signer } = await makeSigner();
    const { headers } = await signer.signUpload({ ...REQUEST, ifGenerationMatch: "9007199254740993123" });
    expect(headers["x-goog-if-generation-match"]).toBe("9007199254740993123");
  });

  it("gives the same link for the same request, and a different one when any rule differs", async () => {
    const { signer } = await makeSigner();
    const base = (await signer.signUpload(REQUEST)).url;

    expect((await signer.signUpload(REQUEST)).url).toBe(base);
    for (const change of [
      { maxBytes: 1235 },
      { ifGenerationMatch: "1" },
      { version: 8 },
      { object: "fedcba9876543210fedcba9876543210" },
      { bucket: "spark-other" },
    ]) {
      expect((await signer.signUpload({ ...REQUEST, ...change })).url).not.toBe(base);
    }
  });

  it("percent-encodes each part of the object name the way the V4 process asks", async () => {
    const { signer } = await makeSigner();
    const { url } = await signer.signUpload({ ...REQUEST, object: "folder/a b!'()*é" });
    expect(url).toContain("/folder/a%20b%21%27%28%29%2A%C3%A9?");
  });

  it.each([
    ["a negative size", { maxBytes: -1 }],
    ["a fractional size", { maxBytes: 1.5 }],
    ["a generation that is not digits", { ifGenerationMatch: "abc" }],
    ["an empty generation", { ifGenerationMatch: "" }],
    ["a negative generation", { ifGenerationMatch: "-1" }],
    ["a negative version", { version: -1 }],
    ["a fractional version", { version: 1.5 }],
  ])("refuses %s", async (_label, change) => {
    const { signer } = await makeSigner();
    await expect(signer.signUpload({ ...REQUEST, ...change })).rejects.toBeInstanceOf(RangeError);
  });
});
