package com.lulu.workbench.android.llm

import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.network.NetworkFactory
import com.lulu.workbench.android.storage.Storage

data class LlmToolDef(
    val name: String,
    val description: String,
    val parametersJson: String,
)

data class LlmToolCall(
    val id: String,
    val name: String,
    val arguments: String,
)

data class LlmMessage(
    val role: String,
    val content: String,
    val toolCallId: String? = null,
    val toolCalls: List<LlmToolCall> = emptyList(),
)

data class LlmCompletion(
    val text: String,
    val toolCalls: List<LlmToolCall> = emptyList(),
)

open class LlmException(message: String) : Exception(message)

class LlmNotConfiguredException : LlmException("llm is not configured")

class LlmHttpException(val status: Int) : LlmException("llm http $status")

interface LlmClient {
    fun catalog(): List<LlmPreset>

    fun loadActive(): LlmActive

    fun select(id: String)

    fun saveActive(baseUrl: String, model: String, apiKey: String)

    fun resetActive()

    fun complete(
        messages: List<LlmMessage>,
        tools: List<LlmToolDef> = emptyList(),
    ): LlmCompletion
}

class LlmClientImpl(
    storage: Storage,
    private val network: NetworkClient,
) : LlmClient {
    private val profiles = LlmProfiles(storage)

    override fun catalog(): List<LlmPreset> = llmCatalog()

    override fun loadActive(): LlmActive = profiles.loadActive()

    override fun select(id: String) {
        profiles.select(id)
    }

    override fun saveActive(baseUrl: String, model: String, apiKey: String) {
        profiles.saveActive(baseUrl, model, apiKey)
    }

    override fun resetActive() {
        profiles.resetActive()
    }

    override fun complete(
        messages: List<LlmMessage>,
        tools: List<LlmToolDef>,
    ): LlmCompletion {
        val active = profiles.loadActive()
        val apiKey = profiles.apiKey(active.id)
        if (apiKey.isEmpty() || active.model.isBlank()) {
            log.w("complete skipped: not configured id=${active.id}")
            throw LlmNotConfiguredException()
        }
        val response = network.execute(
            HttpRequest(
                method = "POST",
                url = chatUrl(active.baseUrl),
                headers = mapOf(
                    "Authorization" to "Bearer $apiKey",
                    "Content-Type" to "application/json",
                ),
                body = encodeChatBody(active.model, messages, tools).encodeToByteArray(),
            ),
        )
        if (response.status !in 200..299) {
            log.w("complete http ${response.status}")
            throw LlmHttpException(response.status)
        }
        log.d("complete ok id=${active.id} tools=${tools.size}")
        val raw = response.body.decodeToString()
        return LlmCompletion(
            text = decodeAssistantText(raw),
            toolCalls = decodeToolCalls(raw),
        )
    }
}

object LlmFactory {
    fun create(storage: Storage): LlmClient = LlmClientImpl(storage, NetworkFactory.create())
}

private val log = WbLog.module(LogModule.LLM)
