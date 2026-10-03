package com.lulu.spark.android.settings.commands

import com.lulu.spark.android.agent.facade.WorkbenchRuntime
import com.lulu.spark.android.agent.tools.web.WebSearchConfig
import com.lulu.spark.android.agent.tools.web.WebSearchTools
import com.lulu.spark.android.asr.AsrClient
import com.lulu.spark.android.asr.AsrConfig
import com.lulu.spark.android.llm.LlmActive
import com.lulu.spark.android.llm.LlmClient
import com.lulu.spark.android.llm.LlmPreset
import javax.inject.Inject

class SettingsCommands(
    private val llm: LlmClient,
    private val asr: AsrClient,
    private val web: WebSearchTools,
) {
    @Inject
    constructor(runtime: WorkbenchRuntime) : this(runtime.llm, runtime.asr, runtime.webSearch)

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

    fun loadWebSearch(): WebSearchConfig = web.loadConfig()

    fun saveWebSearch(apiKey: String) {
        web.saveKey(apiKey)
    }

    fun clearWebSearch() {
        web.clear()
    }
}
