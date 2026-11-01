// What Google really does with the signed upload links. Run it with
//   GCS_SERVICE_ACCOUNT_KEY="$(cat ~/.config/lulu-spark/gcs-sync-sa-key.json)" GCS_LIVE_BUCKET=<an existing test bucket> npm run test:live
// Objects it writes are named `lw726-<random>-...` and deleted at the end (old versions stay until
// the bucket's 3-day rule removes them).
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createAccessTokenProvider } from "../src/gcs/access-token-provider";
import { parseServiceAccountKey } from "../src/gcs/service-account-key";
import { createUploadUrlSigner, type SignedUpload } from "../src/gcs/signed-upload-url";
import { KEY_JSON, STORAGE, makeRandomHex, pickLiveBucket } from "./support";

describe.skipIf(!KEY_JSON)("signed upload links against real Google Cloud Storage", () => {
  const key = parseServiceAccountKey(KEY_JSON);
  const longLived = createAccessTokenProvider({ key });
  const signer = createUploadUrlSigner({ key, lifetimeSeconds: 600 });
  const prefix = `lw726-${makeRandomHex(4)}-`;
  const written = new Set<string>();
  let bucket = "";

  const callAsServer = async (url: string, init: RequestInit = {}) => {
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${await longLived.getAccessToken()}`);
    return fetch(url, { ...init, headers });
  };

  /** What a client does with a signed upload: PUT the bytes to the link with the headers it was given. */
  const put = (signed: SignedUpload, body: string, headers: Record<string, string> = signed.headers) =>
    fetch(signed.url, { method: "PUT", headers, body });

  const sign = (name: string, over: { maxBytes?: number; generation?: string; version?: number } = {}) => {
    written.add(`${prefix}${name}`);
    return signer.signUpload({
      bucket,
      object: `${prefix}${name}`,
      maxBytes: over.maxBytes ?? 100,
      ifGenerationMatch: over.generation ?? "0",
      version: over.version ?? 7,
    });
  };

  const readObject = async (name: string) => {
    const response = await callAsServer(
      `${STORAGE}/b/${bucket}/o/${encodeURIComponent(`${prefix}${name}`)}?fields=size,generation,metadata`,
    );
    return { status: response.status, body: (await response.json()) as Record<string, unknown> };
  };

  beforeAll(async () => {
    bucket = await pickLiveBucket(longLived);
  });

  afterAll(async () => {
    for (const name of written) await callAsServer(`${STORAGE}/b/${bucket}/o/${encodeURIComponent(name)}`, { method: "DELETE" });
  });

  it("is accepted by Google when signed locally with the service account key", async () => {
    const response = await put(await sign("accepted"), "twenty bytes of text");
    expect(response.status).toBe(200);
  });

  it("stores the signed version as metadata that listing returns", async () => {
    expect((await put(await sign("version", { version: 41 }), "hello")).status).toBe(200);

    const object = await readObject("version");
    expect(object.body.size).toBe("5");
    expect(object.body.metadata).toEqual({ version: "41" });

    const listing = await callAsServer(`${STORAGE}/b/${bucket}/o?prefix=${encodeURIComponent(`${prefix}version`)}&fields=items(name,metadata)`);
    expect(await listing.json()).toEqual({ items: [{ name: `${prefix}version`, metadata: { version: "41" } }] });
  });

  it("refuses a second use of the same link for a new object (412)", async () => {
    const signed = await sign("single-use-new");
    expect((await put(signed, "first")).status).toBe(200);
    expect((await put(signed, "second")).status).toBe(412);
    expect((await callAsServer(`${STORAGE}/b/${bucket}/o/${encodeURIComponent(`${prefix}single-use-new`)}?alt=media`)).status).toBe(200);
  });

  it("refuses a second use of the same link for an existing object, once it changed (412)", async () => {
    expect((await put(await sign("single-use-existing"), "original")).status).toBe(200);
    const before = (await readObject("single-use-existing")).body.generation as string;

    const signed = await sign("single-use-existing", { generation: before });
    expect((await put(signed, "replaced")).status).toBe(200);
    expect((await readObject("single-use-existing")).body.generation).not.toBe(before);
    expect((await put(signed, "replaced again")).status).toBe(412);
  });

  it("refuses a link whose generation is already stale because someone else changed the object (412)", async () => {
    expect((await put(await sign("stale"), "v1")).status).toBe(200);
    const stale = (await readObject("stale")).body.generation as string;
    expect((await put(await sign("stale", { generation: stale }), "v2")).status).toBe(200);

    expect((await put(await sign("stale", { generation: stale }), "v3")).status).toBe(412);
  });

  it("refuses a body bigger than the signed limit (400), and accepts one of exactly that size", async () => {
    const tooBig = await put(await sign("too-big", { maxBytes: 10 }), "12345678901");
    expect(tooBig.status).toBe(400);
    expect(await tooBig.text()).toContain("EntityTooLarge");
    expect((await readObject("too-big")).status).toBe(404);

    expect((await put(await sign("exact", { maxBytes: 10 }), "1234567890")).status).toBe(200);
    expect((await put(await sign("smaller", { maxBytes: 10 }), "123")).status).toBe(200);
  });

  it("refuses an upload whose headers differ from, or are missing from, what was signed", async () => {
    const signed = await sign("tampered", { maxBytes: 10, version: 7 });
    const wrongVersion = await put(signed, "hello", { ...signed.headers, "x-goog-meta-version": "8" });
    const biggerRange = await put(signed, "hello", { ...signed.headers, "x-goog-content-length-range": "0,100000" });
    const noVersion = await put(signed, "hello", {
      "x-goog-content-length-range": signed.headers["x-goog-content-length-range"],
      "x-goog-if-generation-match": signed.headers["x-goog-if-generation-match"],
    });

    // A changed value breaks the signature (403); a header that was signed but not sent is malformed (400).
    expect([wrongVersion.status, biggerRange.status]).toEqual([403, 403]);
    expect(await wrongVersion.text()).toContain("SignatureDoesNotMatch");
    expect(noVersion.status).toBe(400);
    expect(await noVersion.text()).toContain("MalformedSecurityHeader");
    expect((await readObject("tampered")).status).toBe(404);
  });

  it("works for its own object only, and only for PUT", async () => {
    const signed = await sign("own-object");
    const otherObject = signed.url.replace(`${prefix}own-object`, `${prefix}another-object`);
    expect((await put({ ...signed, url: otherObject }, "x")).status).toBe(403);
    expect((await fetch(signed.url, { method: "GET", headers: signed.headers })).status).toBe(403);
    expect((await fetch(signed.url, { method: "DELETE", headers: signed.headers })).status).toBe(403);
  });
});
