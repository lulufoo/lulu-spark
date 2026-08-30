package com.lulu.workbench.android.agent.tools.web

import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.HttpResponse
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.net.SocketTimeoutException

class WebSearchToolsTest {
    @Test
    fun saveKeyThenClear() {
        val tools = WebSearchTools(MemoryStorage(), RejectNetwork())
        assertFalse(tools.loadConfig().hasApiKey)
        tools.saveKey("  tvly-secret  ")
        assertTrue(tools.loadConfig().hasApiKey)
        tools.saveKey("   ")
        assertTrue(tools.loadConfig().hasApiKey)
        tools.clear()
        assertFalse(tools.loadConfig().hasApiKey)
    }

    @Test
    fun missingQuery() {
        val tools = configured()
        assertEquals("missing query", tools.call("web_search", """{"query":"  "}"""))
    }

    @Test
    fun noKeySendsKeylessHeader() {
        val network =
            ScriptedNetwork(
                HttpResponse(
                    status = 200,
                    body = """{"results":[{"title":"K","url":"https://k.test","content":"ok"}]}"""
                        .encodeToByteArray(),
                ),
            )
        val tools = WebSearchTools(MemoryStorage(), network)
        val out = tools.call("web_search", """{"query":"kotlin"}""")
        val sent = network.last!!
        assertEquals(KEYLESS_MODE, sent.headers[KEYLESS_HEADER])
        assertTrue(!sent.headers.containsKey("Authorization"))
        assertTrue(out.contains("https://k.test"))
    }

    @Test
    fun postsBearerAndFormatsHits() {
        val storage = MemoryStorage()
        val network =
            ScriptedNetwork(
                HttpResponse(
                    status = 200,
                    body =
                        """{"answer":"ignore","results":[{"title":"Kotlin","url":"https://kotlin.test","content":"A language"},{"title":"Docs","url":"https://docs.test","content":"Reference"}]}"""
                            .encodeToByteArray(),
                ),
            )
        val tools = WebSearchTools(storage, network)
        tools.saveKey("tvly-key")
        val out = tools.call("web_search", """{"query":"kotlin lang","max_results":3}""")
        val sent = network.last!!
        assertEquals("POST", sent.method)
        assertEquals(TAVILY_URL, sent.url)
        assertEquals("Bearer tvly-key", sent.headers["Authorization"])
        assertTrue(!sent.headers.containsKey(KEYLESS_HEADER))
        assertTrue(sent.body!!.decodeToString().contains("\"max_results\":3"))
        assertTrue(sent.body!!.decodeToString().contains("\"include_answer\":false"))
        assertEquals(
            "1. Kotlin\nhttps://kotlin.test\nA language\n\n2. Docs\nhttps://docs.test\nReference",
            out,
        )
        assertTrue(!out.contains("ignore"))
    }

    @Test
    fun emptyResults() {
        val tools =
            configured(
                HttpResponse(status = 200, body = """{"results":[]}""".encodeToByteArray()),
            )
        assertEquals("No results.", tools.call("web_search", """{"query":"zzzz"}"""))
    }

    @Test
    fun http401FailsClosed() {
        val tools =
            configured(HttpResponse(status = 401, body = """{"detail":{"error":"bad"}}""".encodeToByteArray()))
        assertEquals("web search http 401", tools.call("web_search", """{"query":"x"}"""))
    }

    @Test
    fun timeoutFailsClosed() {
        val storage = MemoryStorage()
        storage.putSecret(SECRET_NAME, "tvly-key")
        val tools = WebSearchTools(storage, TimeoutNetwork())
        assertEquals("web search timeout", tools.call("web_search", """{"query":"x"}"""))
    }

    @Test
    fun clampsMaxResults() {
        val network =
            ScriptedNetwork(
                HttpResponse(status = 200, body = """{"results":[]}""".encodeToByteArray()),
            )
        val storage = MemoryStorage()
        storage.putSecret(SECRET_NAME, "tvly-key")
        WebSearchTools(storage, network).call("web_search", """{"query":"x","max_results":99}""")
        assertTrue(network.last!!.body!!.decodeToString().contains("\"max_results\":10"))
    }

    @Test
    fun parseClipsSnippetAndSkipsMissingUrl() {
        val long = "a".repeat(300)
        val hits =
            parseTavilyHits(
                """{"results":[{"title":"NoUrl","content":"x"},{"title":"Ok","url":"https://ok.test","content":"$long"}]}""",
            )
        assertEquals(1, hits.size)
        assertEquals("https://ok.test", hits.single().url)
        assertEquals(SNIPPET_MAX + 1, hits.single().snippet.length)
        assertTrue(hits.single().snippet.endsWith("…"))
    }
}

private fun configured(
    response: HttpResponse =
        HttpResponse(status = 200, body = """{"results":[]}""".encodeToByteArray()),
): WebSearchTools {
    val storage = MemoryStorage()
    storage.putSecret(SECRET_NAME, "tvly-key")
    return WebSearchTools(storage, ScriptedNetwork(response))
}

private class RejectNetwork : NetworkClient {
    override fun execute(request: HttpRequest): HttpResponse {
        throw AssertionError("network should not be called")
    }
}

private class TimeoutNetwork : NetworkClient {
    override fun execute(request: HttpRequest): HttpResponse {
        throw SocketTimeoutException("timeout")
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
