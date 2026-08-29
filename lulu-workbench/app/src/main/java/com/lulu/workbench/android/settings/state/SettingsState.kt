package com.lulu.workbench.android.settings.state

import com.lulu.workbench.android.asr.AsrConfig
import com.lulu.workbench.android.llm.LlmActive
import com.lulu.workbench.android.llm.LlmPreset

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
}
