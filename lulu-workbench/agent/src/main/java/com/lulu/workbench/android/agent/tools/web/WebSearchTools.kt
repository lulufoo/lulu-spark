package com.lulu.workbench.android.agent.tools.web

import com.lulu.workbench.android.agent.tools.jsonIntField
import com.lulu.workbench.android.agent.tools.jsonStringField
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.network.NetworkFactory
import com.lulu.workbench.android.storage.Storage
import java.io.IOException
import java.net.SocketTimeoutException

data class WebSearchConfig(
    val hasApiKey: Boolean = false,
)

class WebSearchTools(
    private val storage: Storage,
    private val network: NetworkClient = NetworkFactory.create(),
) {
    val names: List<String> = listOf("web_search")

    fun definitions(): List<LlmToolDef> =
        listOf(
            LlmToolDef(
                "web_search",
                "Search the public web. Returns short snippets and URLs only. Does not fetch full pages. Use for current facts. Not for the user's notes.",
                SEARCH_PARAMS,
            ),
        )

    fun loadConfig(): WebSearchConfig = WebSearchConfig(hasApiKey = !apiKey().isNullOrBlank())

    fun saveKey(apiKey: String) {
        if (apiKey.isNotBlank()) {
            storage.putSecret(SECRET_NAME, apiKey.trim())
        }
    }

    fun clear() {
        storage.deleteSecret(SECRET_NAME)
    }

    fun call(name: String, arguments: String): String {
        if (name != "web_search") return "unknown web tool '$name'"
        return search(arguments)
    }

    private fun search(arguments: String): String {
        val query = jsonStringField(arguments, "query")?.trim().orEmpty()
        if (query.isEmpty()) return "missing query"
        val limit =
            (jsonIntField(arguments, "max_results") ?: DEFAULT_LIMIT).coerceIn(1, MAX_LIMIT)
        val response = try {
            network.execute(
                HttpRequest(
                    method = "POST",
                    url = TAVILY_URL,
                    headers = searchHeaders(apiKey()),
                    body = encodeSearchBody(query, limit).encodeToByteArray(),
                ),
            )
        } catch (_: SocketTimeoutException) {
            log.w("search timeout")
            return "web search timeout"
        } catch (error: IOException) {
            log.w("search network ${error.javaClass.simpleName}")
            return "web search network failed"
        }
        if (response.status !in 200..299) {
            log.w("search http ${response.status}")
            return "web search http ${response.status}"
        }
        val hits = parseTavilyHits(response.body.decodeToString())
        if (hits.isEmpty()) return "No results."
        log.i("search ok hits=${hits.size}")
        return formatHits(hits)
    }

    private fun apiKey(): String? = storage.getSecret(SECRET_NAME)?.trim()?.ifBlank { null }
}

internal fun searchHeaders(apiKey: String?): Map<String, String> {
    val headers = mutableMapOf("Content-Type" to "application/json")
    if (apiKey.isNullOrBlank()) {
        headers[KEYLESS_HEADER] = KEYLESS_MODE
    } else {
        headers["Authorization"] = "Bearer $apiKey"
    }
    return headers
}

internal fun encodeSearchBody(query: String, maxResults: Int): String =
    """{"query":"${escapeJson(query)}","max_results":$maxResults,""" +
        """"search_depth":"basic","include_answer":false,"include_raw_content":false,""" +
        """"include_images":false}"""

internal fun formatHits(hits: List<WebHit>): String =
    hits.mapIndexed { index, hit ->
        val n = index + 1
        val snippet = hit.snippet.ifBlank { "-" }
        "$n. ${hit.title.ifBlank { "(untitled)" }}\n${hit.url}\n$snippet"
    }.joinToString("\n\n")

private fun escapeJson(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")

private val log = WbLog.module(LogModule.AGENT)

internal const val TAVILY_URL = "https://api.tavily.com/search"
internal const val KEYLESS_HEADER = "X-Tavily-Access-Mode"
internal const val KEYLESS_MODE = "keyless"
internal const val SECRET_NAME = "web_search_key"
internal const val DEFAULT_LIMIT = 5
internal const val MAX_LIMIT = 10

private val SEARCH_PARAMS =
    """{"type":"object","properties":{"query":{"type":"string"},"max_results":{"type":"integer"}},"required":["query"]}"""
