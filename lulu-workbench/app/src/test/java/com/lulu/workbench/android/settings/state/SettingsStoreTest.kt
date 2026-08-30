package com.lulu.workbench.android.settings.state

import com.lulu.workbench.android.agent.facade.WorkbenchRuntime
import com.lulu.workbench.android.settings.commands.SettingsCommands
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SettingsStoreTest {
    @Test
    fun saveWritesCurrentProfileAndKeepsKeyWhenBlank() {
        val runtime = WorkbenchRuntime.createForTest()
        val store = SettingsStore(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        store.dispatch(
            SettingsIntent.Save(
                baseUrl = store.state.active.baseUrl,
                model = "  glm-4  ",
                apiKey = "  secret  ",
            ),
        )
        assertEquals("glm-4", store.state.active.model)
        assertTrue(store.state.active.hasApiKey)
        store.dispatch(
            SettingsIntent.Save(
                baseUrl = store.state.active.baseUrl,
                model = "glm-4-air",
                apiKey = "   ",
            ),
        )
        assertEquals("glm-4-air", store.state.active.model)
        assertTrue(store.state.active.hasApiKey)
    }

    @Test
    fun selectThenResetUsesPresetDefaults() {
        val runtime = WorkbenchRuntime.createForTest()
        val store = SettingsStore(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        assertEquals("glm", store.state.active.id)
        assertFalse(store.state.active.hasApiKey)
        store.dispatch(SettingsIntent.Select("kimi"))
        assertEquals("kimi", store.state.active.id)
        assertEquals("https://api.moonshot.ai/v1", store.state.active.baseUrl)
        store.dispatch(
            SettingsIntent.Save(
                baseUrl = "https://example.test/v1",
                model = "custom",
                apiKey = "",
            ),
        )
        store.dispatch(SettingsIntent.Reset)
        assertEquals("https://api.moonshot.ai/v1", store.state.active.baseUrl)
        assertEquals("kimi-k3", store.state.active.model)
    }

    @Test
    fun saveAsrWritesKeysAndClearRemovesThem() {
        val runtime = WorkbenchRuntime.createForTest()
        val store = SettingsStore(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        assertFalse(store.state.asr.hasSecretKey)
        store.dispatch(SettingsIntent.SaveAsr("125", "AKID", "secret"))
        assertEquals("125", store.state.asr.appId)
        assertEquals("AKID", store.state.asr.secretId)
        assertTrue(store.state.asr.hasSecretKey)
        store.dispatch(SettingsIntent.ClearAsr)
        assertFalse(store.state.asr.hasSecretKey)
        assertEquals("", store.state.asr.appId)
    }

    @Test
    fun saveWebSearchWritesKeyAndClearRemovesIt() {
        val runtime = WorkbenchRuntime.createForTest()
        val store = SettingsStore(SettingsCommands(runtime.llm, runtime.asr, runtime.webSearch))
        store.dispatch(SettingsIntent.Load)
        assertFalse(store.state.webSearch.hasApiKey)
        store.dispatch(SettingsIntent.SaveWebSearch("  tvly-test  "))
        assertTrue(store.state.webSearch.hasApiKey)
        store.dispatch(SettingsIntent.SaveWebSearch("   "))
        assertTrue(store.state.webSearch.hasApiKey)
        store.dispatch(SettingsIntent.ClearWebSearch)
        assertFalse(store.state.webSearch.hasApiKey)
    }
}
