import { describe, expect, it } from "vitest";
import { createBucketProvisioner } from "../../src/gcs/bucket-provisioner";
import { BucketNameTakenError, GcsRequestError } from "../../src/gcs/errors";
import { jsonReply, makeFakeFetch, replyInOrder } from "./fake-fetch";

const NAME = "spark-dee0434c91366b18561ffdcad4380e86";
const OUR_PROJECT_NUMBER = "55973988960";
const BUCKET_URL = `https://storage.googleapis.com/storage/v1/b/${NAME}?fields=projectNumber`;

const ours = () => jsonReply(200, { projectNumber: OUR_PROJECT_NUMBER });
const someoneElses = () => jsonReply(200, { projectNumber: "1234567890" });
const notFound = () => jsonReply(404, { error: { message: "The specified bucket does not exist." } });
const nameTaken = () => jsonReply(409, { error: { message: "Your previous request to create the named bucket succeeded and you already own it." } });
const created = () => jsonReply(200, { name: NAME });

function makeProvisioner(...replies: Response[]) {
  const fake = makeFakeFetch(replyInOrder(...replies));
  const provisioner = createBucketProvisioner({
    projectId: "spark-cloud-sync",
    projectNumber: OUR_PROJECT_NUMBER,
    getAccessToken: async () => "test-token",
    fetchFn: fake.fetchFn,
  });
  return { provisioner, requests: fake.requests };
}

describe("ensureBucket", () => {
  it("does nothing when the bucket is already ours", async () => {
    const { provisioner, requests } = makeProvisioner(ours());

    expect(await provisioner.ensureBucket(NAME)).toEqual({ created: false });

    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe(BUCKET_URL);
    expect(requests[0].headers.get("Authorization")).toBe("Bearer test-token");
  });

  it("creates a missing bucket with region, uniform access, versioning and the 3-day cleanup rule", async () => {
    const { provisioner, requests } = makeProvisioner(notFound(), created());

    expect(await provisioner.ensureBucket(NAME)).toEqual({ created: true });

    expect(requests).toHaveLength(2);
    expect(requests[1].method).toBe("POST");
    expect(requests[1].url).toBe("https://storage.googleapis.com/storage/v1/b?project=spark-cloud-sync");
    expect(requests[1].headers.get("Authorization")).toBe("Bearer test-token");
    expect(JSON.parse(requests[1].body)).toEqual({
      name: NAME,
      location: "US-WEST1",
      storageClass: "STANDARD",
      versioning: { enabled: true },
      iamConfiguration: { uniformBucketLevelAccess: { enabled: true } },
      lifecycle: {
        rule: [{ action: { type: "Delete" }, condition: { isLive: false, daysSinceNoncurrentTime: 3 } }],
      },
    });
  });

  it("counts a lost race as success: the other device created it first", async () => {
    const { provisioner, requests } = makeProvisioner(notFound(), nameTaken(), ours());

    expect(await provisioner.ensureBucket(NAME)).toEqual({ created: false });
    expect(requests.map((request) => request.method)).toEqual(["GET", "POST", "GET"]);
  });

  it("fails when a 409 turns out to be someone else's bucket, whatever the 409 message says", async () => {
    // The message here claims "you already own it". Only the project number decides.
    const { provisioner } = makeProvisioner(notFound(), nameTaken(), someoneElses());

    await expect(provisioner.ensureBucket(NAME)).rejects.toBeInstanceOf(BucketNameTakenError);
  });

  it("fails without creating anything when the name already belongs to another project", async () => {
    const { provisioner, requests } = makeProvisioner(someoneElses());

    await expect(provisioner.ensureBucket(NAME)).rejects.toBeInstanceOf(BucketNameTakenError);
    expect(requests).toHaveLength(1);
  });

  it("treats a bucket we are not allowed to read as someone else's", async () => {
    const { provisioner, requests } = makeProvisioner(jsonReply(403, { error: { message: "forbidden" } }));

    await expect(provisioner.ensureBucket(NAME)).rejects.toBeInstanceOf(BucketNameTakenError);
    expect(requests).toHaveLength(1);
  });

  it("fails when a 409 is followed by a bucket that cannot be found", async () => {
    const { provisioner } = makeProvisioner(notFound(), nameTaken(), notFound());

    await expect(provisioner.ensureBucket(NAME)).rejects.toBeInstanceOf(GcsRequestError);
  });

  it("reports the status and Google's message when creating fails", async () => {
    const { provisioner } = makeProvisioner(notFound(), jsonReply(403, { error: { message: "Caller lacks storage.buckets.create" } }));

    const failure = await provisioner.ensureBucket(NAME).catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(GcsRequestError);
    expect((failure as GcsRequestError).status).toBe(403);
    expect((failure as Error).message).toContain("Caller lacks storage.buckets.create");
  });

  it("reports the status when reading fails", async () => {
    const { provisioner } = makeProvisioner(jsonReply(500, { error: { message: "backend error" } }));

    await expect(provisioner.ensureBucket(NAME)).rejects.toMatchObject({ name: "GcsRequestError", status: 500 });
  });
});
