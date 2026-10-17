package com.lulu.spark.android.agent.tools

import com.lulu.spark.android.agent.tools.stage.StageTools
import com.lulu.spark.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DocContentStageTest {
    @Test
    fun titlePrefersHeadingThenDocId() {
        assertEquals("Park doc", titleFromDocRaw("# Park doc\n\nbody", "id1"))
        assertEquals("id1", titleFromDocRaw("no heading", "id1"))
        assertEquals("Untitled", titleFromDocRaw("", ""))
    }

    @Test
    fun unwrapsMcpEnvelopeAndStages() {
        val storage = MemoryStorage()
        val docId = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        val host = """{"id":"$docId","ok":true,"content":"# Heading\n\nbody"}"""
        val envelope =
            """{"result":{"content":[{"type":"text","text":${escapeJson(host)}}],"isError":false}}"""
        val out = stageDocContentResult(envelope, StageTools(storage), "sess_a", "chat")
        assertTrue(out.startsWith("success handle=F1 "))
        assertTrue(out.contains("doc_id=$docId"))
        assertTrue(!out.contains("body"))
        val second = stageDocContentResult(envelope, StageTools(storage), "sess_a", "chat")
        assertTrue(second.contains("handle=F2"))
    }

    @Test
    fun mcpIsErrorDoesNotStage() {
        val storage = MemoryStorage()
        val envelope =
            """{"result":{"content":[{"type":"text","text":"HTTP 503: unavailable"}],"isError":true}}"""
        assertEquals(
            "HTTP 503: unavailable",
            stageDocContentResult(envelope, StageTools(storage), "sess_a", "chat"),
        )
        assertTrue(storage.list("staged").isEmpty())
    }
}

private fun escapeJson(value: String): String =
    "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\""
