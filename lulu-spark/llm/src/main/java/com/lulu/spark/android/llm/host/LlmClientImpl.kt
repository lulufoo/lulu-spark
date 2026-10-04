package com.lulu.spark.android.llm.host

import com.lulu.spark.android.llm.LlmActive
import com.lulu.spark.android.llm.LlmClient
import com.lulu.spark.android.llm.LlmCompletion
import com.lulu.spark.android.llm.LlmException
import com.lulu.spark.android.llm.LlmHttpException
import com.lulu.spark.android.llm.LlmMessage
import com.lulu.spark.android.llm.LlmNotConfiguredException
import com.lulu.spark.android.llm.LlmPreset
import com.lulu.spark.android.llm.LlmToolDef
import com.lulu.spark.android.llm.chat.chatUrl
import com.lulu.spark.android.llm.chat.decodeAssistantText
import com.lulu.spark.android.llm.chat.decodeFinishReason
import com.lulu.spark.android.llm.chat.decodeToolCalls
import com.lulu.spark.android.llm.chat.encodeChatBody
import com.lulu.spark.android.llm.chat.hasToolCallsKey
import com.lulu.spark.android.llm.chat.toolCallsPreview
import com.lulu.spark.android.llm.llmCatalog
import com.lulu.spark.android.llm.profile.LlmProfiles
import com.lulu.spark.android.log.LogModule
import com.lulu.spark.android.log.WbLog
import com.lulu.spark.android.network.HttpRequest
import com.lulu.spark.android.network.NetworkClient
import com.lulu.spark.android.storage.Storage
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
