package com.lulu.workbench.android.agent.loop

import com.lulu.workbench.android.agent.session.SessionRegistry
import com.lulu.workbench.android.agent.tools.ToolDispatcher
import com.lulu.workbench.android.agent.tools.fs.FsTools
import com.lulu.workbench.android.llm.LlmActive
import com.lulu.workbench.android.llm.LlmClient
import com.lulu.workbench.android.llm.LlmCompletion
import com.lulu.workbench.android.llm.LlmMessage
import com.lulu.workbench.android.llm.LlmPreset
import com.lulu.workbench.android.llm.LlmToolCall
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.llm.llmCatalog
import com.lulu.workbench.android.storage.MemoryStorage
import com.lulu.workbench.android.wmcp.BindOffer
import com.lulu.workbench.android.wmcp.BindResult
import com.lulu.workbench.android.wmcp.McpTool
import com.lulu.workbench.android.wmcp.McpToolResult
import com.lulu.workbench.android.wmcp.WmcpClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class AgentLoopTest {
    @Test
    fun sendDispatchesWriteThenFinishes() {
        val storage = MemoryStorage()
        val sessions = SessionRegistry(storage)
        val id = sessions.create()
        val llm =
            ScriptedLlm(
                mutableListOf(
                    LlmCompletion(
                        text = "",
                        toolCalls =
                            listOf(
                                LlmToolCall(
                                    id = "c1",
                                    name = "write",
                                    arguments = """{"path":"note.txt","content":"hi"}""",
                                ),
                            ),
                    ),
                    LlmCompletion(text = "wrote it"),
                ),
            )
        val loop =
            AgentLoop(
                id,
                llm,
                ToolDispatcher(FsTools(), UnboundWmcp(), storage),
                sessions,
            )
        val seen = mutableListOf<TurnProgress>()
        loop.send("save this") { seen.add(it) }
        assertTrue(seen.any { it is TurnProgress.CallingTool && it.name == "write" })
        assertEquals("wrote it", (seen.last() as TurnProgress.Finished).reply)
        val written = storage.read("sessions/${id.value}/tools/note.txt")
        assertEquals("hi", written?.decodeToString())
    }

    @Test
    fun sendSurvivesMcpListFailureAndFinishes() {
        val storage = MemoryStorage()
        val sessions = SessionRegistry(storage)
        val id = sessions.create()
        val loop =
            AgentLoop(
                id,
                ScriptedLlm(mutableListOf(LlmCompletion(text = "hello"))),
                ToolDispatcher(FsTools(), ThrowingListWmcp(), storage),
                sessions,
            )
        val seen = mutableListOf<TurnProgress>()
        loop.send("hi") { seen.add(it) }
        assertEquals("hello", (seen.last() as TurnProgress.Finished).reply)
        assertTrue(!loop.inFlight)
    }
}

private class ScriptedLlm(
    private val replies: MutableList<LlmCompletion>,
) : LlmClient {
    override fun catalog(): List<LlmPreset> = llmCatalog()

    override fun loadActive(): LlmActive =
        LlmActive("glm", "GLM", "https://example.test", "m", true)

    override fun select(id: String) = Unit

    override fun saveActive(baseUrl: String, model: String, apiKey: String) = Unit

    override fun resetActive() = Unit

    override fun complete(
        messages: List<LlmMessage>,
        tools: List<LlmToolDef>,
    ): LlmCompletion = replies.removeFirst()
}

private class UnboundWmcp : WmcpClient {
    override fun isBound(): Boolean = false

    override fun deviceId(): String = "dev_unbound"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult =
        error("unused")

    override fun listTools(): List<McpTool> = emptyList()

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}

private class ThrowingListWmcp : WmcpClient {
    override fun isBound(): Boolean = true

    override fun deviceId(): String = "dev_throw"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult =
        error("unused")

    override fun listTools(): List<McpTool> =
        throw IllegalArgumentException("mcp tools/list failed 406")

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}
