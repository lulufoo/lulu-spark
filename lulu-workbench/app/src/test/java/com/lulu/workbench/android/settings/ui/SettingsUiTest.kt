package com.lulu.workbench.android.settings.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SettingsUiTest {
    @Test
    fun apiKeyForSaveIgnoresBlankAndMask() {
        assertEquals("", apiKeyForSave(""))
        assertEquals("", apiKeyForSave("   "))
        assertEquals("", apiKeyForSave(SAVED_API_KEY_MASK))
        assertEquals("sk-new", apiKeyForSave("sk-new"))
    }
}
