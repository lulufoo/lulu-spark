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
        val llm = WorkbenchRuntime.createForTest().llm
        val store = SettingsStore(SettingsCommands(llm))
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
        val store = SettingsStore(SettingsCommands(WorkbenchRuntime.createForTest().llm))
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
}
