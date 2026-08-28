export const FEED_X_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-x.json';
export const FEED_PODCASTS_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-podcasts.json';
export const FEED_BLOGS_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-blogs.json';

export async function fetchFeed(url: string) {
  const ts = Date.now();
  const res = await fetch(`${url}?_=${ts}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function loadBuilderFeeds() {
  const [xData, podcastsData, blogsData] = await Promise.all([
    fetchFeed(FEED_X_URL).catch((e: { message?: string }) => ({ error: e.message })),
    fetchFeed(FEED_PODCASTS_URL).catch((e: { message?: string }) => ({ error: e.message })),
    fetchFeed(FEED_BLOGS_URL).catch((e: { message?: string }) => ({ error: e.message })),
  ]);
  return { xData, podcastsData, blogsData };
}
