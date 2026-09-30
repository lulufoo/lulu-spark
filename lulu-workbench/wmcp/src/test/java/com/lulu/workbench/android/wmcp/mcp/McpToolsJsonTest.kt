package com.lulu.workbench.android.wmcp.mcp

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class McpToolsJsonTest {
    @Test
    fun jsonRpcPayloadReadsSseData() {
        val raw = "event: message\ndata: {\"result\":{\"tools\":[{\"name\":\"get_notes\"}]}}\n\n"
        assertEquals(
            listOf("get_notes"),
            parseToolNames(jsonRpcPayload(raw)),
        )
    }

    @Test
    fun parseToolsKeepsHostInputSchemaAndDescription() {
        val tools = parseTools(LIST_TOOLS_WITH_SCHEMA)
        assertEquals(1, tools.size)
        assertEquals("create_note", tools[0].name)
        assertEquals("Create a note from Markdown content.", tools[0].description)
        assertTrue(tools[0].inputSchemaJson.contains("\"properties\""))
        assertTrue(tools[0].inputSchemaJson.contains("\"title\""))
        assertTrue(tools[0].inputSchemaJson.contains("\"content\""))
        assertTrue(tools[0].inputSchemaJson.contains("\"required\""))
    }

    @Test
    fun parseToolsDoesNotTreatNestedNameAsTool() {
        val body =
            """{"result":{"tools":[{"name":"create_note","description":"Add a note.","inputSchema":{"type":"object","properties":{"title":{"type":"string","description":"name of the note"}}}}]}}"""
        assertEquals(listOf("create_note"), parseToolNames(body))
    }

    @Test
    fun parseToolsFallsBackWhenSchemaMissing() {
        val tools = parseTools("""{"result":{"tools":[{"name":"get_notes"}]}}""")
        assertEquals("get_notes", tools.single().name)
        assertEquals("""{"type":"object"}""", tools.single().inputSchemaJson)
    }
}
