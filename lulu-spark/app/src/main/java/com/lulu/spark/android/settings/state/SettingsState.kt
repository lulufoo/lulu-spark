package com.lulu.spark.android.settings.state

import com.lulu.spark.android.agent.tools.web.WebSearchConfig
import com.lulu.spark.android.asr.AsrConfig
import com.lulu.spark.android.llm.LlmActive
import com.lulu.spark.android.llm.LlmPreset

data class SettingsState(
    val catalog: List<LlmPreset> = emptyList(),
    val active: LlmActive = LlmActive(
        id = "",
        label = "",
        baseUrl = "",
        model = "",
        hasApiKey = false,
    ),
    val asr: AsrConfig = AsrConfig(),
    val webSearch: WebSearchConfig = WebSearchConfig(),
)

sealed class SettingsIntent {
    data object Load : SettingsIntent()

    data class Select(val id: String) : SettingsIntent()

    data class Save(
        val baseUrl: String,
        val model: String,
        val apiKey: String,
    ) : SettingsIntent()

    data object Reset : SettingsIntent()

    data class SaveAsr(
        val appId: String,
        val secretId: String,
        val secretKey: String,
    ) : SettingsIntent()

    data object ClearAsr : SettingsIntent()

    data class SaveWebSearch(val apiKey: String) : SettingsIntent()

    data object ClearWebSearch : SettingsIntent()
}
