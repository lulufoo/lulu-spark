package com.lulu.spark.android.agent.tools.stage

import com.lulu.spark.android.storage.Storage
import java.util.UUID

data class StagedItem(
    val id: String,
    val handle: String,
    val title: String,
    val sourceSessionId: String,
    val sourceSessionTitle: String,
    val body: String = "",
)

class StageStore(
    private val storage: Storage,
) {
    fun create(
        title: String,
        body: String,
        sourceSessionId: String,
        sourceSessionTitle: String,
    ): StagedItem {
        backfillHandles()
        val id = "stg_" + UUID.randomUUID().toString().replace("-", "").take(12)
        val item = StagedItem(
            id = id,
            handle = allocateHandle(),
            title = resolveTitle(title, body),
            sourceSessionId = sourceSessionId,
            sourceSessionTitle = sourceSessionTitle.ifBlank { "New chat" },
            body = recoverCollapsedNewlines(body),
        )
        writeItem(item)
        return item
    }

    fun list(): List<StagedItem> {
        backfillHandles()
        return listMetas().sortedBy { handleNumber(it.handle) ?: Int.MAX_VALUE }
    }

    fun listForSession(sessionId: String): List<StagedItem> =
        list().filter { it.sourceSessionId == sessionId }

    fun get(ref: String): StagedItem? {
        backfillHandles()
        val id = resolveRef(ref) ?: return null
        val meta = readMeta(id) ?: return null
        val body = recoverCollapsedNewlines(
            storage.read(bodyPath(id))?.decodeToString().orEmpty(),
        )
        return meta.copy(body = body)
    }

    fun update(id: String, title: String, body: String): StagedItem? {
        val current = get(id) ?: return null
        val next = current.copy(title = resolveTitle(title, body), body = body)
        writeItem(next)
        return next
    }

    fun delete(id: String): Boolean {
        val current = get(id) ?: return false
        storage.delete(metaPath(current.id))
        storage.delete(bodyPath(current.id))
        return true
    }

    private fun listMetas(): List<StagedItem> =
        storage.list(ROOT)
            .filter { it.endsWith("/meta") }
            .mapNotNull { path -> readMeta(idFromMetaPath(path)) }

    private fun resolveRef(ref: String): String? {
        val key = ref.trim()
        if (ID.matches(key)) return key
        val handle = key.uppercase()
        if (!HANDLE.matches(handle)) return null
        return listMetas().firstOrNull { it.handle == handle }?.id
    }

    private fun backfillHandles() {
        val items = listMetas()
        var high = maxOf(readSeq(), items.mapNotNull { handleNumber(it.handle) }.maxOrNull() ?: 0)
        items.filter { !HANDLE.matches(it.handle) }.sortedBy { it.id }.forEach { item ->
            high += 1
            writeMeta(item.copy(handle = "F$high"))
        }
        if (high > readSeq()) writeSeq(high)
    }

    private fun allocateHandle(): String {
        val n = readSeq() + 1
        writeSeq(n)
        return "F$n"
    }

    private fun readSeq(): Int =
        storage.read(SEQ_PATH)?.decodeToString()?.trim()?.toIntOrNull()?.coerceAtLeast(0) ?: 0

    private fun writeSeq(value: Int) {
        storage.write(SEQ_PATH, value.toString().encodeToByteArray())
    }

    private fun writeItem(item: StagedItem) {
        writeMeta(item)
        storage.write(bodyPath(item.id), item.body.encodeToByteArray())
    }

    private fun writeMeta(item: StagedItem) {
        storage.write(metaPath(item.id), encodeMeta(item).encodeToByteArray())
    }

    private fun readMeta(id: String): StagedItem? {
        if (!ID.matches(id)) return null
        val raw = storage.read(metaPath(id))?.decodeToString() ?: return null
        val parsedId = field(raw, "id") ?: return null
        if (parsedId != id) return null
        return StagedItem(
            id = parsedId,
            handle = field(raw, "handle").orEmpty(),
            title = field(raw, "title").orEmpty(),
            sourceSessionId = field(raw, "source_session_id").orEmpty(),
            sourceSessionTitle = field(raw, "source_session_title").orEmpty(),
        )
    }
}

internal fun recoverCollapsedNewlines(body: String): String {
    if (body.contains('\n') || !body.contains("nn")) return body
    return body.replace(COLLAPSED_BREAK, "\n\n")
}

internal fun resolveTitle(title: String, body: String): String {
    val trimmed = title.trim()
    if (trimmed.isNotEmpty()) return trimmed
    val line = body.lineSequence().firstOrNull()?.trim().orEmpty()
    return if (line.isEmpty()) "Untitled" else line.take(40)
}

internal fun metaPath(id: String): String = "$ROOT/$id/meta"

internal fun bodyPath(id: String): String = "$ROOT/$id/body.md"

internal fun handleNumber(handle: String): Int? {
    if (!HANDLE.matches(handle)) return null
    return handle.drop(1).toIntOrNull()
}

private fun idFromMetaPath(path: String): String {
    val parts = path.split('/')
    return parts.getOrElse(1) { "" }
}

private fun encodeMeta(item: StagedItem): String =
    """{"id":"${escape(item.id)}","handle":"${escape(item.handle)}","title":"${escape(item.title)}","source_session_id":"${escape(item.sourceSessionId)}","source_session_title":"${escape(item.sourceSessionTitle)}"}"""

private fun field(json: String, key: String): String? {
    val needle = "\"$key\""
    val at = json.indexOf(needle)
    if (at < 0) return null
    val colon = json.indexOf(':', startIndex = at + needle.length)
    val quote = json.indexOf('"', startIndex = colon + 1)
    if (quote < 0) return null
    return readJsonString(json, quote)
}

private fun readJsonString(source: String, openQuote: Int): String {
    val out = StringBuilder()
    var i = openQuote + 1
    while (i < source.length) {
        val c = source[i]
        if (c == '"') break
        if (c != '\\' || i + 1 >= source.length) {
            out.append(c)
            i += 1
            continue
        }
        when (val next = source[i + 1]) {
            'n' -> out.append('\n')
            '"' -> out.append('"')
            '\\' -> out.append('\\')
            else -> out.append(next)
        }
        i += 2
    }
    return out.toString()
}

private fun escape(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")

private const val ROOT = "staged"

internal const val SEQ_PATH = "staged/seq"

private val ID = Regex("^stg_[A-Za-z0-9]+$")

private val HANDLE = Regex("^F[1-9][0-9]*$")

private val COLLAPSED_BREAK =
    Regex("nn(?=(#{1,6}\\s|[-*]\\s|\\d+\\.\\s|```|>\\s|\\||\\p{IsHan}))")
