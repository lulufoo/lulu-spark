export type RecordedRequest = {
  url: string;
  method: string;
  headers: Headers;
  body: string;
};

type Responder = (request: RecordedRequest) => Response | Promise<Response>;

export function jsonReply(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

/** A `fetch` that records every request and answers through `respond`. Nothing leaves the process. */
export function makeFakeFetch(respond: Responder) {
  const requests: RecordedRequest[] = [];
  const fetchFn = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const request: RecordedRequest = {
      url: String(input),
      method: init?.method ?? "GET",
      headers: new Headers(init?.headers),
      body: init?.body === undefined ? "" : String(init.body),
    };
    requests.push(request);
    return respond(request);
  }) as typeof fetch;
  return { fetchFn, requests };
}

/** Answers the first request with the first reply, the second with the second, and so on. */
export function replyInOrder(...replies: Response[]): Responder {
  let next = 0;
  return (request) => {
    const reply = replies[next++];
    if (!reply) throw new Error(`unexpected request #${next}: ${request.method} ${request.url}`);
    return reply;
  };
}
