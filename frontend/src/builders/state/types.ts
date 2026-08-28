export type BuilderTweet = {
  url?: string;
  text?: string;
  createdAt?: string;
  likes?: number;
  retweets?: number;
  replies?: number;
};

export type BuilderXUser = {
  name?: string;
  handle?: string;
  tweets?: BuilderTweet[];
};

export type BuilderPodcast = {
  url?: string;
  title?: string;
  name?: string;
  publishedAt?: string;
  transcript?: string;
};

export type BuilderBlog = {
  url?: string;
  title?: string;
  author?: string;
  name?: string;
  publishedAt?: string;
};

export type XFeedData = {
  generatedAt?: string;
  error?: string;
  x?: BuilderXUser[];
};

export type PodcastsFeedData = {
  generatedAt?: string;
  error?: string;
  podcasts?: BuilderPodcast[];
};

export type BlogsFeedData = {
  generatedAt?: string;
  error?: string;
  blogs?: BuilderBlog[];
};

export type FeedViewState =
  | { kind: 'loading' }
  | { kind: 'error'; message: string }
  | {
      kind: 'ready';
      generatedAt: string;
      podcastsData: PodcastsFeedData;
      blogsData: BlogsFeedData;
      xData: XFeedData;
      xUsers: BuilderXUser[];
    };

export function errMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'message' in err) {
    const msg = (err as { message?: unknown }).message;
    if (typeof msg === 'string' && msg) return msg;
  }
  return fallback;
}
