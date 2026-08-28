import type { BlogsFeedData, PodcastsFeedData, XFeedData } from '../state/types.ts';

export const FEED_X_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-x.json';
export const FEED_PODCASTS_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-podcasts.json';
export const FEED_BLOGS_URL =
  'https://raw.githubusercontent.com/zarazhangrui/follow-builders/main/feed-blogs.json';

export async function fetchFeed(url: string): Promise<unknown> {
  const ts = Date.now();
  const res = await fetch(`${url}?_=${ts}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function loadBuilderFeeds(): Promise<{
  xData: XFeedData;
  podcastsData: PodcastsFeedData;
  blogsData: BlogsFeedData;
}> {
  const [xData, podcastsData, blogsData] = await Promise.all([
    fetchFeed(FEED_X_URL)
      .then((data) => data as XFeedData)
      .catch((e: { message?: string }) => ({ error: e.message })),
    fetchFeed(FEED_PODCASTS_URL)
      .then((data) => data as PodcastsFeedData)
      .catch((e: { message?: string }) => ({ error: e.message })),
    fetchFeed(FEED_BLOGS_URL)
      .then((data) => data as BlogsFeedData)
      .catch((e: { message?: string }) => ({ error: e.message })),
  ]);
  return { xData, podcastsData, blogsData };
}
