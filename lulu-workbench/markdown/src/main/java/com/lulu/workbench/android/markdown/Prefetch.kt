package com.lulu.workbench.android.markdown

import com.lulu.workbench.android.markdown.cache.prefetchMarkdown as warmCache

/** Preheat parse cache. Chat calls this off the UI thread; paint still goes through MarkdownBody. */
fun prefetchMarkdown(source: String) = warmCache(source)
