package com.lulu.workbench.android.llm.host

import com.lulu.workbench.android.llm.LlmActive
import com.lulu.workbench.android.llm.LlmClient
import com.lulu.workbench.android.llm.LlmCompletion
import com.lulu.workbench.android.llm.LlmException
import com.lulu.workbench.android.llm.LlmHttpException
import com.lulu.workbench.android.llm.LlmMessage
import com.lulu.workbench.android.llm.LlmNotConfiguredException
import com.lulu.workbench.android.llm.LlmPreset
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.llm.chat.chatUrl
import com.lulu.workbench.android.llm.chat.decodeAssistantText
import com.lulu.workbench.android.llm.chat.decodeFinishReason
import com.lulu.workbench.android.llm.chat.decodeToolCalls
import com.lulu.workbench.android.llm.chat.encodeChatBody
import com.lulu.workbench.android.llm.chat.hasToolCallsKey
import com.lulu.workbench.android.llm.chat.toolCallsPreview
import com.lulu.workbench.android.llm.llmCatalog
import com.lulu.workbench.android.llm.profile.LlmProfiles
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.storage.Storage
import java.io.IOException
import java.net.SocketTimeoutException

internal class LlmClientImpl(
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
        val response = try {
            network.execute(
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
        } catch (_: SocketTimeoutException) {
            log.w("complete timeout id=${active.id}")
            throw LlmException("llm timeout")
        } catch (error: IOException) {
            log.w("complete network ${error.javaClass.simpleName}")
            throw LlmException("llm network failed")
        }
        if (response.status !in 200..299) {
            log.w("complete http ${response.status}")
            throw LlmHttpException(response.status)
        }
        val raw = response.body.decodeToString()
        val calls = decodeToolCalls(raw)
        val finish = decodeFinishReason(raw)
        val hasKey = hasToolCallsKey(raw)
        log.d(
            "complete ok id=${active.id} tools=${tools.size} " +
                "calls=${calls.size} finish=${finish.ifEmpty { "-" }} " +
                "hasKey=$hasKey",
        )
        if (calls.isEmpty() && hasKey) {
            log.d("complete unread tool_calls preview=${toolCallsPreview(raw)}")
        }
        return LlmCompletion(
            text = decodeAssistantText(raw),
            toolCalls = calls,
        )
    }
}

private val log = WbLog.module(LogModule.LLM)
