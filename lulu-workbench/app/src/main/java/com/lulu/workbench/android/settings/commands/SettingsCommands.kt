package com.lulu.workbench.android.settings.commands

import com.lulu.workbench.android.asr.AsrClient
import com.lulu.workbench.android.asr.AsrConfig
import com.lulu.workbench.android.llm.LlmActive
import com.lulu.workbench.android.llm.LlmClient
import com.lulu.workbench.android.llm.LlmPreset

class SettingsCommands(
    private val llm: LlmClient,
    private val asr: AsrClient,
) {
    fun catalog(): List<LlmPreset> = llm.catalog()

    fun loadActive(): LlmActive = llm.loadActive()

    fun select(id: String) {
        llm.select(id)
    }

    fun save(baseUrl: String, model: String, apiKey: String) {
        llm.saveActive(baseUrl, model, apiKey)
    }

    fun reset() {
        llm.resetActive()
    }

    fun loadAsr(): AsrConfig = asr.loadConfig()

    fun saveAsr(appId: String, secretId: String, secretKey: String) {
        asr.saveConfig(appId, secretId, secretKey)
    }

    fun clearAsr() {
        asr.clearConfig()
    }
}
