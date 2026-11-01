import { describe, expect, it } from "vitest";
import { InvalidLimitsError } from "../../src/gcs/errors";
import { parseLimits, parseUploadLinkLifetime } from "../../src/gcs/limits";

describe("parseLimits", () => {
  it("accepts numbers", () => {
    expect(parseLimits(10485760, 104857600)).toEqual({ maxObjectBytes: 10485760, maxUserTotalBytes: 104857600 });
  });

  it("accepts digits written as text, which is how a Workers setting can arrive", () => {
    expect(parseLimits("10485760", " 104857600 ")).toEqual({ maxObjectBytes: 10485760, maxUserTotalBytes: 104857600 });
  });

  it("accepts an object limit equal to the total", () => {
    expect(parseLimits(100, 100)).toEqual({ maxObjectBytes: 100, maxUserTotalBytes: 100 });
  });

  it.each([
    ["missing", undefined],
    ["null", null],
    ["zero", 0],
    ["negative", -5],
    ["fractional", 1.5],
    ["not a number", Number.NaN],
    ["too large to count exactly", 2 ** 53],
    ["empty text", ""],
    ["text with a unit", "10MB"],
    ["negative text", "-5"],
    ["a boolean", true],
  ])("refuses a limit that is %s, so a typo never switches the limit off", (_label, bad) => {
    expect(() => parseLimits(bad, 104857600)).toThrow(InvalidLimitsError);
    expect(() => parseLimits(10485760, bad)).toThrow(InvalidLimitsError);
  });

  it("refuses an object limit above the total limit, and names the setting", () => {
    expect(() => parseLimits(200, 100)).toThrow(/MAX_OBJECT_BYTES is larger than MAX_USER_TOTAL_BYTES/);
  });

  it("names the setting that is wrong", () => {
    expect(() => parseLimits(undefined, 100)).toThrow(/MAX_OBJECT_BYTES/);
    expect(() => parseLimits(100, undefined)).toThrow(/MAX_USER_TOTAL_BYTES/);
  });
});

describe("parseUploadLinkLifetime", () => {
  it.each([[600], ["600"], [60], [3600]])("accepts %s", (value) => {
    expect(parseUploadLinkLifetime(value)).toBe(Number(value));
  });

  it.each([
    ["missing", undefined],
    ["null", null],
    ["under a minute", 59],
    ["over an hour", 3601],
    ["zero", 0],
    ["negative", -600],
    ["fractional", 600.5],
    ["not a number", Number.NaN],
    ["text with a unit", "10m"],
    ["empty text", ""],
  ])("refuses a lifetime that is %s", (_label, bad) => {
    expect(() => parseUploadLinkLifetime(bad)).toThrow(InvalidLimitsError);
  });

  it("names the setting", () => {
    expect(() => parseUploadLinkLifetime(undefined)).toThrow(/UPLOAD_LINK_LIFETIME_SECONDS/);
  });
});
