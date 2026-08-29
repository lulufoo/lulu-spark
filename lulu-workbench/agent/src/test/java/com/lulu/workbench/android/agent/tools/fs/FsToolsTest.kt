package com.lulu.workbench.android.agent.tools.fs

import com.lulu.workbench.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class FsToolsTest {
    @Test
    fun writeReadEditAndGrepStayInsideToolRoot() {
        val storage = MemoryStorage()
        val tools = FsTools()
        val root = "sessions/sess_a/tools"
        assertEquals(
            "Wrote $root/note.txt",
            tools.call("write", """{"path":"note.txt","content":"hello world"}""", root, storage),
        )
        assertEquals("1:hello world", tools.call("read", """{"path":"note.txt"}""", root, storage))
        assertEquals(
            "Edited $root/note.txt",
            tools.call(
                "edit",
                """{"path":"note.txt","old_text":"world","new_text":"android"}""",
                root,
                storage,
            ),
        )
        val grep = tools.call("grep", """{"pattern":"android"}""", root, storage)
        assertTrue(grep.contains("note.txt:1:hello android"))
    }

    @Test
    fun rejectsParentTraversal() {
        val storage = MemoryStorage()
        val result =
            FsTools().call(
                "read",
                """{"path":"../history"}""",
                "sessions/sess_a/tools",
                storage,
            )
        assertEquals("path is outside the tool session root", result)
    }
}
