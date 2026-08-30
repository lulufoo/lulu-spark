package com.lulu.workbench.android.agent.tools

import com.lulu.workbench.android.agent.tools.stage.StageTools
import com.lulu.workbench.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class NoteContentStageTest {
    @Test
    fun titlePrefersHeadingThenNoteId() {
        assertEquals("Park note", titleFromNoteRaw("# Park note\n\nbody", "id1"))
        assertEquals("id1", titleFromNoteRaw("no heading", "id1"))
        assertEquals("Untitled", titleFromNoteRaw("", ""))
    }

    @Test
    fun unwrapsMcpEnvelopeAndStages() {
        val storage = MemoryStorage()
        val noteId = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        val host = """{"id":"$noteId","ok":true,"content":"# Heading\n\nbody"}"""
        val envelope =
            """{"result":{"content":[{"type":"text","text":${escapeJson(host)}}],"isError":false}}"""
        val out = stageNoteContentResult(envelope, StageTools(storage), "sess_a", "chat")
        assertTrue(out.startsWith("success handle=F1 "))
        assertTrue(out.contains("note_id=$noteId"))
        assertTrue(!out.contains("body"))
        val second = stageNoteContentResult(envelope, StageTools(storage), "sess_a", "chat")
        assertTrue(second.contains("handle=F2"))
    }

    @Test
    fun mcpIsErrorDoesNotStage() {
        val storage = MemoryStorage()
        val envelope =
            """{"result":{"content":[{"type":"text","text":"HTTP 503: unavailable"}],"isError":true}}"""
        assertEquals(
            "HTTP 503: unavailable",
            stageNoteContentResult(envelope, StageTools(storage), "sess_a", "chat"),
        )
        assertTrue(storage.list("staged").isEmpty())
    }
}

private fun escapeJson(value: String): String =
    "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\""
