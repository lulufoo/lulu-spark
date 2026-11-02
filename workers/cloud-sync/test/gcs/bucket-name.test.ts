import { describe, expect, it } from "vitest";
import { deriveBucketName } from "../../src/gcs/bucket-name";

const KEY = "test-bucket-name-key-0123456789abcdef";
const USER_ID = "123e4567-e89b-12d3-a456-426614174000";

describe("deriveBucketName", () => {
  // Expected values come from Python: "spark-" + hmac.new(key, user.lower().encode(), sha256).hexdigest()[:32].
  // They pin the algorithm: changing it would orphan every user's data.
  it("matches an independent HMAC-SHA256 implementation", async () => {
    expect(await deriveBucketName(USER_ID, KEY)).toBe("spark-dee0434c91366b18561ffdcad4380e86");
    expect(await deriveBucketName("6d8a6427-a930-4030-bb9f-96564d90d11d", KEY)).toBe(
      "spark-92f2f07d9a6783e87e9b74cd699d0c8d",
    );
  });

  it("gives 38 characters: the prefix and 32 lowercase hex digits", async () => {
    const name = await deriveBucketName(USER_ID, KEY);
    expect(name).toMatch(/^spark-[0-9a-f]{32}$/);
    expect(name).toHaveLength(38);
  });

  it("ignores the case of the user id", async () => {
    expect(await deriveBucketName("6D8A6427-A930-4030-BB9F-96564D90D11D", KEY)).toBe(
      await deriveBucketName("6d8a6427-a930-4030-bb9f-96564d90d11d", KEY),
    );
  });

  it("gives different users different names", async () => {
    const other = "123e4567-e89b-12d3-a456-426614174001";
    expect(await deriveBucketName(other, KEY)).not.toBe(await deriveBucketName(USER_ID, KEY));
  });

  it("gives the same user different names under a different key", async () => {
    expect(await deriveBucketName(USER_ID, `${KEY}x`)).not.toBe(await deriveBucketName(USER_ID, KEY));
  });

  it("does not contain the user id", async () => {
    const name = await deriveBucketName(USER_ID, KEY);
    expect(name).not.toContain(USER_ID.slice(0, 8));
  });
});
