package com.lulu.spark.android.agent.tools.stage

import com.lulu.spark.android.agent.session.HistoryTurn
import com.lulu.spark.android.agent.session.SessionRegistry
import com.lulu.spark.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class StageStoreTest {
    @Test
    fun createWritesMetaAndMarkdownBody() {
        val storage = MemoryStorage()
        val store = StageStore(storage)
        val item = store.create("Commute note", "# Hello\n\nBody", "sess_a", "first line")
        assertTrue(item.id.startsWith("stg_"))
        assertEquals("F1", item.handle)
        assertEquals("# Hello\n\nBody", storage.read(bodyPath(item.id))?.decodeToString())
        val loaded = store.get(item.id)
        assertEquals("Commute note", loaded?.title)
        assertEquals("sess_a", loaded?.sourceSessionId)
        assertEquals("first line", loaded?.sourceSessionTitle)
        assertEquals("# Hello\n\nBody", loaded?.body)
    }

    @Test
    fun emptyTitleUsesFirstBodyLine() {
        val store = StageStore(MemoryStorage())
        val item = store.create("  ", "# Heading\nmore", "sess_a", "chat")
        assertEquals("# Heading", item.title)
    }

    @Test
    fun listOmitsBodyAndFiltersBySession() {
        val store = StageStore(MemoryStorage())
        store.create("A", "a", "sess_a", "chat a")
        store.create("B", "b", "sess_b", "chat b")
        val all = store.list()
        assertEquals(2, all.size)
        assertTrue(all.all { it.body.isEmpty() })
        assertEquals(listOf("A"), store.listForSession("sess_a").map { it.title })
    }

    @Test
    fun deleteSessionKeepsStagedFiles() {
        val storage = MemoryStorage()
        val sessions = SessionRegistry(storage)
        val id = sessions.create()
        sessions.saveTurns(id, listOf(HistoryTurn("user", "keep me")))
        val store = StageStore(storage)
        val item = store.create("Kept", "body", id.value, "keep me")
        sessions.delete(id)
        assertTrue(sessions.list().isEmpty())
        assertNotNull(store.get(item.id))
        assertEquals("body", storage.read(bodyPath(item.id))?.decodeToString())
    }

    @Test
    fun updateRewritesTitleAndBody() {
        val store = StageStore(MemoryStorage())
        val item = store.create("Old", "old body", "sess_a", "chat")
        val next = store.update(item.id, "New", "new body")
        assertEquals("New", next?.title)
        assertEquals("new body", store.get(item.id)?.body)
        assertEquals("F1", next?.handle)
    }

    @Test
    fun handlesIncrementGloballyAndGetAcceptsHandle() {
        val store = StageStore(MemoryStorage())
        val first = store.create("A", "a", "sess_a", "chat")
        val second = store.create("B", "b", "sess_b", "chat")
        assertEquals("F1", first.handle)
        assertEquals("F2", second.handle)
        assertEquals("A", store.get("F1")?.title)
        assertEquals("B", store.get("f2")?.title)
        assertEquals(listOf("F1", "F2"), store.list().map { it.handle })
    }

    @Test
    fun deleteRemovesFilesAndDoesNotRecycleHandles() {
        val storage = MemoryStorage()
        val store = StageStore(storage)
        val first = store.create("A", "a", "sess_a", "chat")
        val second = store.create("B", "b", "sess_a", "chat")
        assertEquals(true, store.delete(first.id))
        assertEquals(null, store.get(first.id))
        assertEquals(null, storage.read(metaPath(first.id)))
        assertEquals(null, storage.read(bodyPath(first.id)))
        assertNotNull(store.get(second.id))
        val third = store.create("C", "c", "sess_a", "chat")
        assertEquals("F3", third.handle)
        assertEquals(null, store.get("F1"))
        assertEquals(listOf("F2", "F3"), store.list().map { it.handle })
    }

    @Test
    fun recoverCollapsedNewlinesRestoresMarkdownBreaks() {
        assertEquals(
            "# Hi\n\n正文",
            recoverCollapsedNewlines("# Hinn正文"),
        )
        assertEquals(
            "# A\n\n## B",
            recoverCollapsedNewlines("# Ann## B"),
        )
        assertEquals("dinner", recoverCollapsedNewlines("dinner"))
        assertEquals("has\nline", recoverCollapsedNewlines("has\nline"))
    }

    @Test
    fun listBackfillsMissingHandles() {
        val storage = MemoryStorage()
        storage.write(
            "staged/stg_old/meta",
            """{"id":"stg_old","title":"Legacy","source_session_id":"sess_a","source_session_title":"chat"}""".encodeToByteArray(),
        )
        storage.write("staged/stg_old/body.md", "legacy".encodeToByteArray())
        val store = StageStore(storage)
        val listed = store.list()
        assertEquals(listOf("F1"), listed.map { it.handle })
        assertEquals("Legacy", store.get("F1")?.title)
        assertEquals("1", storage.read(SEQ_PATH)?.decodeToString())
    }
}
