// The link-lifetime question, against real Google Cloud Storage:
// 1. an upload that runs longer than the old 3 minutes succeeds when the link lives 10 minutes, and
//    the object that arrives has the right bytes and version;
// 2. a link first used after it expired is refused with ExpiredToken.
// Takes about 4 minutes, so it only runs when GCS_LIVE_SLOW=1. Same environment as upload-live.test.ts.
import { appendFileSync } from "node:fs";
import { request } from "node:https";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createAccessTokenProvider } from "../src/gcs/access-token-provider";
import { parseServiceAccountKey } from "../src/gcs/service-account-key";
import { createUploadUrlSigner } from "../src/gcs/signed-upload-url";
import { KEY_JSON, STORAGE, makeRandomHex, pickLiveBucket } from "./support";

const LINK_LIFETIME_SECONDS = 600;
const SHORT_LINK_LIFETIME_SECONDS = 5;
const OLD_LIFETIME_SECONDS = 180;
const SLOW_BODY_BYTES = 240;
const BYTE_INTERVAL_MS = 1000; // 240 bytes at one per second: 4 minutes, 60 s past the old 3-minute life

function putSlowly(url: string, headers: Record<string, string>): Promise<{ status: number; body: string; seconds: number }> {
  const startedAt = Date.now();
  return new Promise((resolve, reject) => {
    const req = request(url, { method: "PUT", headers: { ...headers, "content-length": String(SLOW_BODY_BYTES) } }, (res) => {
      let body = "";
      res.on("data", (chunk) => (body += chunk));
      res.on("end", () => resolve({ status: res.statusCode ?? 0, body, seconds: (Date.now() - startedAt) / 1000 }));
    });
    req.on("error", reject);
    let sent = 0;
    const timer = setInterval(() => {
      req.write("x");
      sent += 1;
      if (sent === SLOW_BODY_BYTES) {
        clearInterval(timer);
        req.end();
      }
    }, BYTE_INTERVAL_MS);
  });
}

describe.skipIf(!KEY_JSON || process.env.GCS_LIVE_SLOW !== "1")("the lifetime of a signed upload link", () => {
  const key = parseServiceAccountKey(KEY_JSON);
  const longLived = createAccessTokenProvider({ key });
  const signer = createUploadUrlSigner({ key, lifetimeSeconds: LINK_LIFETIME_SECONDS });
  const shortSigner = createUploadUrlSigner({ key, lifetimeSeconds: SHORT_LINK_LIFETIME_SECONDS });
  const prefix = `lw726-slow-${makeRandomHex(4)}-`;
  let bucket = "";

  beforeAll(async () => {
    bucket = await pickLiveBucket(longLived);
  });

  afterAll(async () => {
    const token = await longLived.getAccessToken();
    for (const name of ["slow", "late"]) {
      await fetch(`${STORAGE}/b/${bucket}/o/${encodeURIComponent(prefix + name)}`, { method: "DELETE", headers: { Authorization: `Bearer ${token}` } });
    }
  });

  it("finishes an upload that runs past 3 minutes, and refuses a link first used after it expired", async () => {
    const slow = await signer.signUpload({ bucket, object: prefix + "slow", maxBytes: SLOW_BODY_BYTES, ifGenerationMatch: "0", version: 3 });
    const late = await shortSigner.signUpload({ bucket, object: prefix + "late", maxBytes: SLOW_BODY_BYTES, ifGenerationMatch: "0", version: 3 });

    const slowUpload = putSlowly(slow.url, slow.headers);
    // First use of the short link, after its few seconds are over, while the slow upload is running.
    await new Promise((resolve) => setTimeout(resolve, (SHORT_LINK_LIFETIME_SECONDS + 5) * 1000));
    const lateUse = await fetch(late.url, { method: "PUT", headers: late.headers, body: "x" });
    const lateText = await lateUse.text();
    const slowResult = await slowUpload;

    // What arrived in the bucket: size, version and the bytes themselves.
    const token = await longLived.getAccessToken();
    const objectUrl = `${STORAGE}/b/${bucket}/o/${encodeURIComponent(prefix + "slow")}`;
    const metadata = (await (await fetch(`${objectUrl}?fields=size,metadata`, { headers: { Authorization: `Bearer ${token}` } })).json()) as {
      size?: string;
      metadata?: { version?: string };
    };
    const content = await (await fetch(`${objectUrl}?alt=media`, { headers: { Authorization: `Bearer ${token}` } })).text();

    // The console is not shown in every environment; keep the evidence in a (git-ignored) file.
    appendFileSync(
      ".live-test-results.log",
      `${new Date().toISOString()} ` +
        JSON.stringify({
          lifetimeSeconds: LINK_LIFETIME_SECONDS,
          slowResult: { status: slowResult.status, seconds: slowResult.seconds, body: slowResult.body.slice(0, 200) },
          arrived: { size: metadata.size, version: metadata.metadata?.version, contentIsAllX: content === "x".repeat(SLOW_BODY_BYTES) },
          lateUse: { status: lateUse.status, body: lateText.slice(0, 200) },
        }) +
        "\n",
    );

    expect(slowResult.seconds).toBeGreaterThan(OLD_LIFETIME_SECONDS);
    expect(slowResult.status).toBe(200);
    expect(metadata.size).toBe(String(SLOW_BODY_BYTES));
    expect(metadata.metadata?.version).toBe("3");
    expect(content).toBe("x".repeat(SLOW_BODY_BYTES));
    expect(lateUse.status).toBeGreaterThanOrEqual(400);
    expect(lateText).toContain("ExpiredToken");
  }, 6 * 60 * 1000);
});
