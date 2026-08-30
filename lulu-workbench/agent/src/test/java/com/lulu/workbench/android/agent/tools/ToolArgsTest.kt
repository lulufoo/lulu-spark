package com.lulu.workbench.android.agent.tools

import org.junit.Assert.assertEquals
import org.junit.Test

class ToolArgsTest {
    @Test
    fun jsonStringKeepsRealNewlinesFromEscapes() {
        val json = """{"content":"# Hi\n\nPara"}"""
        assertEquals("# Hi\n\nPara", jsonStringField(json, "content"))
    }

    @Test
    fun jsonStringKeepsBackslashNWhenDoubled() {
        val json = """{"content":"a\\n"}"""
        assertEquals("a\\n", jsonStringField(json, "content"))
    }
}
