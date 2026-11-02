import { buildBucketSpec } from "./bucket-spec";
import { BucketNameTakenError, GcsRequestError } from "./errors";

const STORAGE_API = "https://storage.googleapis.com/storage/v1";

/** `ours`: in this project. `foreign`: exists but not ours (or not readable by us). */
type BucketState = "ours" | "foreign" | "missing";

export type BucketProvisioner = {
  /** Makes sure the bucket exists in this project. `created` is false when it was already there. */
  ensureBucket(name: string): Promise<{ created: boolean }>;
};

export type BucketProvisionerOptions = {
  projectId: string;
  /** The numeric project id. A bucket is ours only if it reports this number. */
  projectNumber: string;
  getAccessToken: () => Promise<string>;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
};

async function readErrorDetail(response: Response): Promise<string | undefined> {
  try {
    const body = (await response.json()) as { error?: { message?: unknown } };
    const message = body.error?.message;
    return typeof message === "string" ? message.slice(0, 200) : undefined;
  } catch {
    return undefined;
  }
}

export function createBucketProvisioner(options: BucketProvisionerOptions): BucketProvisioner {
  const { projectId, projectNumber, getAccessToken } = options;
  const fetchFn = options.fetchFn ?? ((input, init) => fetch(input, init));

  async function sendRequest(url: string, init: RequestInit = {}): Promise<Response> {
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${await getAccessToken()}`);
    return fetchFn(url, { ...init, headers });
  }

  async function inspectBucket(name: string): Promise<BucketState> {
    const response = await sendRequest(
      `${STORAGE_API}/b/${encodeURIComponent(name)}?fields=projectNumber`,
    );
    if (response.status === 404) return "missing";
    // A private bucket of another project answers 403: the name is in use and not ours.
    if (response.status === 403) return "foreign";
    if (!response.ok) {
      throw new GcsRequestError("read bucket", response.status, await readErrorDetail(response));
    }
    const body = (await response.json()) as { projectNumber?: unknown };
    return String(body.projectNumber) === projectNumber ? "ours" : "foreign";
  }

  /** Returns false when the name is already taken (409), whoever holds it. */
  async function createBucket(name: string): Promise<boolean> {
    const response = await sendRequest(`${STORAGE_API}/b?project=${encodeURIComponent(projectId)}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(buildBucketSpec(name)),
    });
    if (response.status === 409) return false;
    if (!response.ok) {
      throw new GcsRequestError("create bucket", response.status, await readErrorDetail(response));
    }
    return true;
  }

  return {
    async ensureBucket(name) {
      const before = await inspectBucket(name);
      if (before === "ours") return { created: false };
      if (before === "foreign") throw new BucketNameTakenError("bucket name belongs to another project");

      if (await createBucket(name)) return { created: true };

      // 409 reads the same whether we already own the bucket (another device was first) or someone
      // else holds the name, and the message text is no way to tell. Read the bucket instead.
      const after = await inspectBucket(name);
      if (after === "ours") return { created: false };
      if (after === "foreign") throw new BucketNameTakenError("bucket name belongs to another project");
      throw new GcsRequestError("create bucket", 409, "name reported taken but the bucket cannot be found");
    },
  };
}
