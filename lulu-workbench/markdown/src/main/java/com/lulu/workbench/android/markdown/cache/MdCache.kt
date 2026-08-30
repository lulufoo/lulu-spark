package com.lulu.workbench.android.markdown.cache

import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.markdown.model.MdBlock
import com.lulu.workbench.android.markdown.parse.parseMdBlocks
import java.security.MessageDigest
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap

fun prefetchMarkdown(source: String) {
    cachedMdBlocks(source)
}

internal fun cachedMdBlocks(source: String): List<MdBlock> = MdBlockCache.getOrParse(source)

internal object MdBlockCache {
    private val blocks = ConcurrentHashMap<String, CachedMd>()
    private val log = WbLog.module(LogModule.MARKDOWN)

    fun getOrParse(source: String): List<MdBlock> {
        blocks[source]?.let { cached ->
            log.i(
                "parse hit key=${cached.key} chars=${source.length} " +
                    "blocks=${cached.blocks.size} thread=${Thread.currentThread().name}",
            )
            return cached.blocks
        }
        return blocks.computeIfAbsent(source) { text ->
            val started = System.nanoTime()
            val parsed = parseMdBlocks(text)
            val key = md5Hex(text)
            val ms = (System.nanoTime() - started) / 1_000_000.0
            log.i(
                "parse miss key=$key chars=${text.length} blocks=${parsed.size} " +
                    "ms=${"%.1f".format(Locale.US, ms)} thread=${Thread.currentThread().name}",
            )
            CachedMd(key = key, blocks = parsed)
        }.blocks
    }

    fun size(): Int = blocks.size

    fun clear() {
        blocks.clear()
    }
}

private class CachedMd(
    val key: String,
    val blocks: List<MdBlock>,
)

private fun md5Hex(text: String): String {
    val digest = MessageDigest.getInstance("MD5").digest(text.toByteArray(Charsets.UTF_8))
    return digest.joinToString("") { byte -> "%02x".format(Locale.US, byte) }
}
