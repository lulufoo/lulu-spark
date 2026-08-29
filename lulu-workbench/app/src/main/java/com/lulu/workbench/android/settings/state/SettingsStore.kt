package com.lulu.workbench.android.settings.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.settings.commands.SettingsCommands

class SettingsStore(
    private val commands: SettingsCommands,
) {
    var state: SettingsState by mutableStateOf(SettingsState())
        private set

    fun dispatch(intent: SettingsIntent) {
        when (intent) {
            SettingsIntent.Load -> reload()
            is SettingsIntent.Select -> {
                commands.select(intent.id)
                reload()
            }
            is SettingsIntent.Save -> {
                commands.save(intent.baseUrl, intent.model, intent.apiKey)
                reload()
            }
            SettingsIntent.Reset -> {
                commands.reset()
                reload()
            }
        }
    }

    private fun reload() {
        state = SettingsState(catalog = commands.catalog(), active = commands.loadActive())
    }
}
