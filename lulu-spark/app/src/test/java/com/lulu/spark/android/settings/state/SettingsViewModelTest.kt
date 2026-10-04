package com.lulu.spark.android.settings.state

import com.lulu.spark.android.agent.facade.SparkRuntime
import com.lulu.spark.android.settings.SettingsViewModel
import com.lulu.spark.android.settings.commands.SettingsCommands
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SettingsViewModelTest {
    @Test
    fun saveWritesCurrentProfileAndKeepsKeyWhenBlank() {
        val runtime = SparkRuntime.createForTest()
        val store = SettingsViewModel(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        store.dispatch(
            SettingsIntent.Save(
                baseUrl = store.state.value.active.baseUrl,
                model = "  glm-4  ",
                apiKey = "  secret  ",
            ),
        )
        assertEquals("glm-4", store.state.value.active.model)
        assertTrue(store.state.value.active.hasApiKey)
        store.dispatch(
            SettingsIntent.Save(
                baseUrl = store.state.value.active.baseUrl,
                model = "glm-4-air",
                apiKey = "   ",
            ),
        )
        assertEquals("glm-4-air", store.state.value.active.model)
        assertTrue(store.state.value.active.hasApiKey)
    }

    @Test
    fun selectThenResetUsesPresetDefaults() {
        val runtime = SparkRuntime.createForTest()
        val store = SettingsViewModel(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        assertEquals("glm", store.state.value.active.id)
        assertFalse(store.state.value.active.hasApiKey)
        store.dispatch(SettingsIntent.Select("kimi"))
        assertEquals("kimi", store.state.value.active.id)
        assertEquals("https://api.moonshot.ai/v1", store.state.value.active.baseUrl)
        store.dispatch(
            SettingsIntent.Save(
                baseUrl = "https://example.test/v1",
                model = "custom",
                apiKey = "",
            ),
        )
        store.dispatch(SettingsIntent.Reset)
        assertEquals("https://api.moonshot.ai/v1", store.state.value.active.baseUrl)
        assertEquals("kimi-k3", store.state.value.active.model)
    }

    @Test
    fun saveAsrWritesKeysAndClearRemovesThem() {
        val runtime = SparkRuntime.createForTest()
        val store = SettingsViewModel(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        assertFalse(store.state.value.asr.hasSecretKey)
        store.dispatch(SettingsIntent.SaveAsr("125", "AKID", "secret"))
        assertEquals("125", store.state.value.asr.appId)
        assertEquals("AKID", store.state.value.asr.secretId)
        assertTrue(store.state.value.asr.hasSecretKey)
        store.dispatch(SettingsIntent.ClearAsr)
        assertFalse(store.state.value.asr.hasSecretKey)
        assertEquals("", store.state.value.asr.appId)
    }

    @Test
    fun saveWebSearchWritesKeyAndClearRemovesIt() {
        val runtime = SparkRuntime.createForTest()
        val store = SettingsViewModel(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        assertFalse(store.state.value.webSearch.hasApiKey)
        store.dispatch(SettingsIntent.SaveWebSearch("  tvly-test  "))
        assertTrue(store.state.value.webSearch.hasApiKey)
        store.dispatch(SettingsIntent.SaveWebSearch("   "))
        assertTrue(store.state.value.webSearch.hasApiKey)
        store.dispatch(SettingsIntent.ClearWebSearch)
        assertFalse(store.state.value.webSearch.hasApiKey)
    }
}
