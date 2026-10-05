package com.lulu.spark.android.agent.tools.stage

import com.lulu.spark.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class StageToolsTest {
    @Test
    fun stageThenGetRoundTrip() {
        val tools = StageTools(MemoryStorage())
        val staged = tools.call(
            "stage",
            """{"title":"Note","content":"# Hi"}""",
            "sess_a",
            "first",
        )
        assertTrue(staged.startsWith("Staged F1 "))
        assertTrue(staged.contains("id=stg_"))
        val body = tools.call("get_staged", """{"id":"F1"}""", "sess_a", "first")
        assertTrue(body.contains("handle: F1"))
        assertTrue(body.contains("title: Note"))
        assertTrue(body.contains("# Hi"))
        val listed = tools.call("list_staged", "{}", "sess_a", "first")
        assertTrue(listed.contains("handle=F1"))
        assertTrue(listed.contains("this_chat=true"))
        val other = tools.call("list_staged", "{}", "sess_b", "other")
        assertTrue(other.contains("this_chat=false"))
        val only = tools.call("list_staged", """{"this_chat_only":true}""", "sess_b", "other")
        assertEquals("No staged files.", only)
    }

    @Test
    fun stageKeepsJsonNewlinesForPreview() {
        val tools = StageTools(MemoryStorage())
        tools.call(
            "stage",
            """{"title":"Note","content":"# Hi\n\nPara"}""",
            "sess_a",
            "first",
        )
        val body = tools.call("get_staged", """{"id":"F1"}""", "sess_a", "first")
        assertTrue(body.contains("# Hi\n\nPara"))
        assertTrue(!body.contains("nnPara"))
    }

    @Test
    fun missingContentIsError() {
        val tools = StageTools(MemoryStorage())
        assertEquals("missing content", tools.call("stage", "{}", "sess_a", "x"))
        assertEquals("staged file not found", tools.call("get_staged", """{"id":"stg_nope"}""", "", ""))
    }

    @Test
    fun catalogHasNoDelete() {
        val tools = StageTools(MemoryStorage())
        assertEquals(listOf("stage", "list_staged", "get_staged"), tools.names)
        assertEquals(
            "unknown stage tool 'delete_staged'",
            tools.call("delete_staged", "{}", "sess_a", "first"),
        )
    }
}
