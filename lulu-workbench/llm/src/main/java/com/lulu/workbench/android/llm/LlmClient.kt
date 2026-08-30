package com.lulu.workbench.android.llm

import com.lulu.workbench.android.llm.host.LlmClientImpl
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

object LlmFactory {
    fun create(storage: Storage): LlmClient = LlmClientImpl(storage, NetworkFactory.create())
}
