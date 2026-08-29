package com.lulu.workbench.android.llm

import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.HttpResponse
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LlmClientTest {
    @Test(expected = LlmNotConfiguredException::class)
    fun completeWithoutKeyFails() {
        val llm = LlmClientImpl(MemoryStorage(), RejectNetwork())
        llm.complete(listOf(LlmMessage("user", "hi")))
    }

    @Test
    fun completeUsesSelectedProfileUrlAndMigratesLegacy() {
        val storage = MemoryStorage()
        storage.write("llm/model.txt", "legacy-model".encodeToByteArray())
        storage.putSecret("llm_api_key", "legacy-key")
        val network = ScriptedNetwork(
            HttpResponse(
                status = 200,
                body = """{"choices":[{"message":{"content":"hello"}}]}""".encodeToByteArray(),
            ),
        )
        val llm = LlmClientImpl(storage, network)
        val active = llm.loadActive()
        assertEquals("glm", active.id)
        assertEquals("legacy-model", active.model)
        assertTrue(active.hasApiKey)
        assertEquals("hello", llm.complete(listOf(LlmMessage("user", "hi"))).text)
        assertEquals("https://open.bigmodel.cn/api/paas/v4/chat/completions", network.last?.url)
    }

    @Test
    fun selectKimiUsesKimiUrlAndOwnKey() {
        val storage = MemoryStorage()
        val network = ScriptedNetwork(
            HttpResponse(
                status = 200,
                body = """{"choices":[{"message":{"content":"ok"}}]}""".encodeToByteArray(),
            ),
        )
        val llm = LlmClientImpl(storage, network)
        llm.saveActive(llm.loadActive().baseUrl, "glm-4", "glm-key")
        llm.select("kimi")
        assertFalse(llm.loadActive().hasApiKey)
        llm.saveActive("https://api.moonshot.ai/v1", "kimi-k3", "kimi-key")
        llm.complete(listOf(LlmMessage("user", "hi")))
        assertEquals("https://api.moonshot.ai/v1/chat/completions", network.last?.url)
        llm.select("glm")
        assertTrue(llm.loadActive().hasApiKey)
        assertEquals("glm-4", llm.loadActive().model)
        val glmFile = storage.read("llm/glm")!!.decodeToString()
        assertTrue(glmFile.contains("\"model\":\"glm-4\""))
        assertTrue(storage.read("llm/glm/model") == null)
    }

    @Test
    fun migratesSplitFilesIntoOneProfileFile() {
        val storage = MemoryStorage()
        storage.write("llm/glm/base_url", "https://old.example/v4".encodeToByteArray())
        storage.write("llm/glm/model", "old-model".encodeToByteArray())
        val llm = LlmClientImpl(storage, RejectNetwork())
        assertEquals("https://old.example/v4", llm.loadActive().baseUrl)
        assertEquals("old-model", llm.loadActive().model)
        assertTrue(storage.read("llm/glm") != null)
        assertTrue(storage.read("llm/glm/model") == null)
    }

    @Test
    fun resetActiveRestoresPresetUrlAndModel() {
        val llm = LlmClientImpl(MemoryStorage(), RejectNetwork())
        llm.saveActive("https://example.test/v1", "custom-model", "")
        assertEquals("https://example.test/v1", llm.loadActive().baseUrl)
        llm.resetActive()
        val glm = llmPreset("glm")!!
        assertEquals(glm.defaultBaseUrl, llm.loadActive().baseUrl)
        assertEquals(glm.defaultModel, llm.loadActive().model)
    }

    @Test
    fun catalogHasThreeProviders() {
        val catalog = LlmClientImpl(MemoryStorage(), RejectNetwork()).catalog()
        assertEquals(listOf("glm", "kimi", "openai"), catalog.map { it.id })
        val glm = catalog.single { it.id == "glm" }
        assertEquals("Agent Loop / GLM", glm.label)
        assertEquals("https://open.bigmodel.cn/api/paas/v4", glm.defaultBaseUrl)
        assertEquals("", glm.defaultModel)
    }

    @Test
    fun decodeFinishReasonAndToolCallsKey() {
        val raw =
            """{"choices":[{"finish_reason":"stop","message":{"content":"ok"}}]}"""
        assertEquals("stop", decodeFinishReason(raw))
        assertFalse(hasToolCallsKey(raw))
        val withCalls =
            """{"choices":[{"finish_reason":"tool_calls","message":{"content":null,"tool_calls":[]}}]}"""
        assertEquals("tool_calls", decodeFinishReason(withCalls))
        assertTrue(hasToolCallsKey(withCalls))
    }

    @Test
    fun decodeToolCallsReadsFunction() {
        val calls =
            decodeToolCalls(
                """{"choices":[{"message":{"content":null,"tool_calls":[{"id":"c1","type":"function","function":{"name":"write","arguments":"{\"path\":\"a\"}"}}]}}]}""",
            )
        assertEquals("write", calls.single().name)
        assertEquals("c1", calls.single().id)
        assertEquals("""{"path":"a"}""", calls.single().arguments)
    }

    @Test
    fun chatUrlAppendsOnV4() {
        assertEquals(
            "https://open.bigmodel.cn/api/paas/v4/chat/completions",
            chatUrl("https://open.bigmodel.cn/api/paas/v4"),
        )
    }
}

private class RejectNetwork : NetworkClient {
    override fun execute(request: HttpRequest): HttpResponse {
        throw AssertionError("network should not be called")
    }
}

private class ScriptedNetwork(
    private val response: HttpResponse,
) : NetworkClient {
    var last: HttpRequest? = null

    override fun execute(request: HttpRequest): HttpResponse {
        last = request
        return response
    }
}
