import { GcsRequestError, ListingTooLargeError } from "./errors";

const STORAGE_API = "https://storage.googleapis.com/storage/v1";
const PAGE_SIZE = 1000;
// 20,000 listed items is far beyond what one user will have today. Past it, listing would cost too
// many requests, so the answer is a failure rather than a list or total that is too short.
const MAX_PAGES = 20;

/** A live object and its custom metadata. */
export type ListedObject = {
  name: string;
  metadata: Record<string, string>;
};

export type ObjectStore = {
  /** Bytes in every version of every object, including old versions the 3-day rule has not removed yet. */
  totalBytes(bucket: string): Promise<number>;
  /**
   * The live objects, without old versions, each with its custom metadata, in name order. Null when
   * the bucket does not exist.
   */
  listObjects(bucket: string): Promise<ListedObject[] | null>;
  /** The generation of the live object, or null when there is none. */
  currentGeneration(bucket: string, object: string): Promise<string | null>;
  /** Deletes the live object. An object that is not there counts as deleted. */
  deleteObject(bucket: string, object: string): Promise<void>;
};

export type ObjectStoreOptions = {
  /** The long-lived account's access token. */
  getAccessToken: () => Promise<string>;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
};

export function createObjectStore(options: ObjectStoreOptions): ObjectStore {
  const fetchFn = options.fetchFn ?? ((input, init) => fetch(input, init));

  async function callStorage(url: string, method = "GET"): Promise<Response> {
    return fetchFn(url, { method, headers: { Authorization: `Bearer ${await options.getAccessToken()}` } });
  }

  async function failure(operation: string, response: Response): Promise<GcsRequestError> {
    const body = (await response.json().catch(() => ({}))) as { error?: { status?: unknown } };
    const code = typeof body.error?.status === "string" ? body.error.status : undefined;
    return new GcsRequestError(operation, response.status, code);
  }

  /**
   * Every item of a listing, page by page, with the default page size of 1000 (a larger one makes
   * Google return everything at once). Null when the bucket does not exist.
   */
  async function listAll<Item>(bucket: string, parameters: Record<string, string>, fields: string): Promise<Item[] | null> {
    const items: Item[] = [];
    let pageToken: string | undefined;
    for (let page = 0; page < MAX_PAGES; page += 1) {
      const query = new URLSearchParams({ ...parameters, maxResults: String(PAGE_SIZE), fields: `${fields},nextPageToken` });
      if (pageToken) query.set("pageToken", pageToken);
      const response = await callStorage(`${STORAGE_API}/b/${encodeURIComponent(bucket)}/o?${query}`);
      if (response.status === 404) return null;
      if (!response.ok) throw await failure("list objects", response);
      const body = (await response.json()) as { items?: Item[]; nextPageToken?: string };
      items.push(...(body.items ?? []));
      if (!body.nextPageToken) return items;
      pageToken = body.nextPageToken;
    }
    throw new ListingTooLargeError(`more than ${MAX_PAGES * PAGE_SIZE} listed items`);
  }

  return {
    async totalBytes(bucket) {
      const items = await listAll<{ size?: string }>(bucket, { versions: "true" }, "items(size)");
      // The caller has just made sure the bucket exists, so a missing one is a failure here.
      if (items === null) throw new GcsRequestError("list objects", 404, "NOT_FOUND");
      return items.reduce((total, item) => total + Number(item.size ?? 0), 0);
    },

    async listObjects(bucket) {
      const items = await listAll<{ name?: unknown; metadata?: Record<string, string> }>(bucket, {}, "items(name,metadata)");
      if (items === null) return null;
      return items.flatMap((item) => (typeof item.name === "string" ? [{ name: item.name, metadata: item.metadata ?? {} }] : []));
    },

    async currentGeneration(bucket, object) {
      const response = await callStorage(
        `${STORAGE_API}/b/${encodeURIComponent(bucket)}/o/${encodeURIComponent(object)}?fields=generation`,
      );
      if (response.status === 404) return null;
      if (!response.ok) throw await failure("read object", response);
      const body = (await response.json()) as { generation?: unknown };
      if (typeof body.generation !== "string" || !/^\d+$/.test(body.generation)) {
        throw new GcsRequestError("read object", response.status, "answer has no generation");
      }
      return body.generation;
    },

    async deleteObject(bucket, object) {
      const response = await callStorage(
        `${STORAGE_API}/b/${encodeURIComponent(bucket)}/o/${encodeURIComponent(object)}`,
        "DELETE",
      );
      if (response.ok || response.status === 404) return;
      throw await failure("delete object", response);
    },
  };
}
