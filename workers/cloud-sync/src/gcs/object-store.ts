import { GcsRequestError, ListingTooLargeError } from "./errors";

const STORAGE_API = "https://storage.googleapis.com/storage/v1";
const PAGE_SIZE = 1000;
// 20,000 versions is far beyond what one user will have. Past it, adding up would cost too many
// requests, so the answer is a failure rather than a total that is too small.
const MAX_PAGES = 20;

export type ObjectStore = {
  /** Bytes in every version of every object, including old versions the 3-day rule has not removed yet. */
  totalBytes(bucket: string): Promise<number>;
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

  return {
    async totalBytes(bucket) {
      let total = 0;
      let pageToken: string | undefined;
      for (let page = 0; page < MAX_PAGES; page += 1) {
        const query = new URLSearchParams({
          versions: "true",
          maxResults: String(PAGE_SIZE),
          fields: "items(size),nextPageToken",
        });
        if (pageToken) query.set("pageToken", pageToken);
        const response = await callStorage(`${STORAGE_API}/b/${encodeURIComponent(bucket)}/o?${query}`);
        if (!response.ok) throw await failure("list objects", response);
        const body = (await response.json()) as { items?: { size?: string }[]; nextPageToken?: string };
        for (const item of body.items ?? []) total += Number(item.size ?? 0);
        if (!body.nextPageToken) return total;
        pageToken = body.nextPageToken;
      }
      throw new ListingTooLargeError(`more than ${MAX_PAGES * PAGE_SIZE} object versions`);
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
