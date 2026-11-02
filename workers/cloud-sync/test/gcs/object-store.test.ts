import { describe, expect, it } from "vitest";
import { GcsRequestError, ListingTooLargeError } from "../../src/gcs/errors";
import { createObjectStore } from "../../src/gcs/object-store";
import { jsonReply, makeFakeFetch, replyInOrder, type RecordedRequest } from "./fake-fetch";

const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86";
const OBJECT = "0123456789abcdef0123456789abcdef";
const LIST_URL = `https://storage.googleapis.com/storage/v1/b/${BUCKET}/o`;

function makeStore(respond: (request: RecordedRequest) => Response | Promise<Response>) {
  const { fetchFn, requests } = makeFakeFetch(respond);
  return { store: createObjectStore({ getAccessToken: async () => "long-lived-token", fetchFn }), requests };
}

describe("totalBytes", () => {
  it("adds up every version of every object, asking for versions, with the long-lived token", async () => {
    const { store, requests } = makeStore(replyInOrder(jsonReply(200, { items: [{ size: "100" }, { size: "250" }, { size: "7" }] })));

    expect(await store.totalBytes(BUCKET)).toBe(357);

    expect(requests).toHaveLength(1);
    const url = new URL(requests[0].url);
    expect(`${url.origin}${url.pathname}`).toBe(LIST_URL);
    expect(url.searchParams.get("versions")).toBe("true");
    expect(url.searchParams.get("fields")).toBe("items(size),nextPageToken");
    expect(requests[0].headers.get("Authorization")).toBe("Bearer long-lived-token");
  });

  it("answers 0 for an empty bucket, which Google reports with no items at all", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(200, {})));
    expect(await store.totalBytes(BUCKET)).toBe(0);
  });

  it("follows the page token until there is none, adding every page", async () => {
    const { store, requests } = makeStore(
      replyInOrder(
        jsonReply(200, { items: [{ size: "10" }], nextPageToken: "page-2" }),
        jsonReply(200, { items: [{ size: "20" }], nextPageToken: "page-3" }),
        jsonReply(200, { items: [{ size: "30" }] }),
      ),
    );

    expect(await store.totalBytes(BUCKET)).toBe(60);

    expect(requests.map((request) => new URL(request.url).searchParams.get("pageToken"))).toEqual([null, "page-2", "page-3"]);
  });

  it("fails instead of answering with a total that is too small when there are too many pages", async () => {
    const { store, requests } = makeStore(() => jsonReply(200, { items: [{ size: "1" }], nextPageToken: "more" }));

    await expect(store.totalBytes(BUCKET)).rejects.toBeInstanceOf(ListingTooLargeError);

    expect(requests).toHaveLength(20);
  });

  it("fails with the status and Google's error code, and nothing else from the answer", async () => {
    const { store } = makeStore(() => jsonReply(403, { error: { status: "PERMISSION_DENIED", message: "secret detail" } }));

    const error = await store.totalBytes(BUCKET).catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(GcsRequestError);
    expect(String((error as Error).message)).toContain("403");
    expect(String((error as Error).message)).toContain("PERMISSION_DENIED");
    expect(String((error as Error).message)).not.toContain("secret detail");
  });

  it("fails on an answer that is not JSON, without leaking it", async () => {
    const { store } = makeStore(() => new Response("<html>oops</html>", { status: 502 }));
    const error = await store.totalBytes(BUCKET).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(GcsRequestError);
    expect(String((error as Error).message)).not.toContain("oops");
  });
});

describe("listObjects", () => {
  it("lists the live objects only, asking for names and metadata, with the long-lived token", async () => {
    const { store, requests } = makeStore(
      replyInOrder(jsonReply(200, { items: [{ name: "a".repeat(32), metadata: { version: "3" } }, { name: "b".repeat(32), metadata: { version: "12" } }] })),
    );

    expect(await store.listObjects(BUCKET)).toEqual([
      { name: "a".repeat(32), metadata: { version: "3" } },
      { name: "b".repeat(32), metadata: { version: "12" } },
    ]);

    expect(requests).toHaveLength(1);
    const url = new URL(requests[0].url);
    expect(`${url.origin}${url.pathname}`).toBe(LIST_URL);
    expect(url.searchParams.has("versions")).toBe(false); // old versions are not documents
    expect(url.searchParams.get("fields")).toBe("items(name,metadata),nextPageToken");
    expect(url.searchParams.get("maxResults")).toBe("1000"); // the default page size, never a larger one
    expect(requests[0].headers.get("Authorization")).toBe("Bearer long-lived-token");
  });

  it("gives an object that has no metadata an empty metadata object", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(200, { items: [{ name: "c".repeat(32) }] })));
    expect(await store.listObjects(BUCKET)).toEqual([{ name: "c".repeat(32), metadata: {} }]);
  });

  it("leaves out an item that has no name", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(200, { items: [{ metadata: { version: "1" } }, { name: "d".repeat(32), metadata: { version: "1" } }] })));
    expect(await store.listObjects(BUCKET)).toEqual([{ name: "d".repeat(32), metadata: { version: "1" } }]);
  });

  it("returns an empty list for a bucket with no objects, which Google reports with no items", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(200, {})));
    expect(await store.listObjects(BUCKET)).toEqual([]);
  });

  it("returns null when the bucket does not exist", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(404, { error: { status: "NOT_FOUND" } })));
    expect(await store.listObjects(BUCKET)).toBeNull();
  });

  it("follows the page token until there is none, keeping every object in order", async () => {
    const page = (names: string[], next?: string) => jsonReply(200, { items: names.map((name) => ({ name, metadata: { version: "1" } })), ...(next ? { nextPageToken: next } : {}) });
    const { store, requests } = makeStore(replyInOrder(page(["a", "b"], "page-2"), page(["c"], "page-3"), page(["d", "e"])));

    expect((await store.listObjects(BUCKET))?.map((object) => object.name)).toEqual(["a", "b", "c", "d", "e"]);

    expect(requests.map((request) => new URL(request.url).searchParams.get("pageToken"))).toEqual([null, "page-2", "page-3"]);
  });

  it("fails instead of returning a short list when there are too many pages", async () => {
    const { store, requests } = makeStore(() => jsonReply(200, { items: [{ name: "x", metadata: {} }], nextPageToken: "more" }));
    await expect(store.listObjects(BUCKET)).rejects.toBeInstanceOf(ListingTooLargeError);
    expect(requests).toHaveLength(20);
  });

  it("fails on any other error rather than answering with an empty list", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(403, { error: { status: "PERMISSION_DENIED", message: "secret detail" } })));
    const error = await store.listObjects(BUCKET).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(GcsRequestError);
    expect(String((error as Error).message)).toContain("PERMISSION_DENIED");
    expect(String((error as Error).message)).not.toContain("secret detail");
  });

  it("fails when a later page fails, rather than answering with the pages so far", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(200, { items: [{ name: "a", metadata: {} }], nextPageToken: "p2" }), jsonReply(500, { error: { status: "INTERNAL" } })));
    await expect(store.listObjects(BUCKET)).rejects.toBeInstanceOf(GcsRequestError);
  });
});

describe("currentGeneration", () => {
  it("returns the generation of the live object", async () => {
    const { store, requests } = makeStore(replyInOrder(jsonReply(200, { generation: "1730419200123456" })));

    expect(await store.currentGeneration(BUCKET, OBJECT)).toBe("1730419200123456");

    expect(requests[0].method).toBe("GET");
    expect(requests[0].url).toBe(`${LIST_URL}/${OBJECT}?fields=generation`);
  });

  it("returns null when there is no such object", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(404, { error: { status: "NOT_FOUND" } })));
    expect(await store.currentGeneration(BUCKET, OBJECT)).toBeNull();
  });

  it("fails on any other error rather than treating it as 'no object'", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(500, { error: { status: "INTERNAL" } })));
    await expect(store.currentGeneration(BUCKET, OBJECT)).rejects.toBeInstanceOf(GcsRequestError);
  });

  it.each([
    ["has no generation", {}],
    ["has a generation that is not digits", { generation: "abc" }],
    ["has a generation that is a number", { generation: 5 }],
  ])("fails when the answer %s", async (_label, body) => {
    const { store } = makeStore(replyInOrder(jsonReply(200, body)));
    await expect(store.currentGeneration(BUCKET, OBJECT)).rejects.toBeInstanceOf(GcsRequestError);
  });

  it("percent-encodes the object name in the path", async () => {
    const { store, requests } = makeStore(replyInOrder(jsonReply(404, {})));
    await store.currentGeneration(BUCKET, "a/b c");
    expect(requests[0].url).toContain("/o/a%2Fb%20c?");
  });
});

describe("deleteObject", () => {
  it("deletes the object with the long-lived token", async () => {
    const { store, requests } = makeStore(replyInOrder(new Response(null, { status: 204 })));

    await store.deleteObject(BUCKET, OBJECT);

    expect(requests[0].method).toBe("DELETE");
    expect(requests[0].url).toBe(`${LIST_URL}/${OBJECT}`);
    expect(requests[0].headers.get("Authorization")).toBe("Bearer long-lived-token");
  });

  it("counts an object that is not there as deleted", async () => {
    const { store } = makeStore(replyInOrder(jsonReply(404, { error: { status: "NOT_FOUND" } })));
    await expect(store.deleteObject(BUCKET, OBJECT)).resolves.toBeUndefined();
  });

  it.each([403, 429, 500, 503])("fails on %s", async (status) => {
    const { store } = makeStore(replyInOrder(jsonReply(status, { error: { status: "SOME_CODE" } })));
    await expect(store.deleteObject(BUCKET, OBJECT)).rejects.toBeInstanceOf(GcsRequestError);
  });
});
