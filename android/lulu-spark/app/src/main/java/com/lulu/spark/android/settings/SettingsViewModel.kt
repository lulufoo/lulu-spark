package com.lulu.spark.android.settings

import androidx.lifecycle.ViewModel
import com.lulu.spark.android.settings.commands.SettingsCommands
import com.lulu.spark.android.settings.state.SettingsIntent
import com.lulu.spark.android.settings.state.SettingsState
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

@HiltViewModel
class SettingsViewModel @Inject constructor(
    private val commands: SettingsCommands,
) : ViewModel() {
    private val _state = MutableStateFlow(SettingsState())
    val state: StateFlow<SettingsState> = _state.asStateFlow()

    init {
        reload()
    }

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
            is SettingsIntent.SaveAsr -> {
                commands.saveAsr(intent.appId, intent.secretId, intent.secretKey)
                reload()
            }
            SettingsIntent.ClearAsr -> {
                commands.clearAsr()
                reload()
            }
            is SettingsIntent.SaveWebSearch -> {
                commands.saveWebSearch(intent.apiKey)
                reload()
            }
            SettingsIntent.ClearWebSearch -> {
                commands.clearWebSearch()
                reload()
            }
        }
    }

    private fun reload() {
        _state.value = SettingsState(
            catalog = commands.catalog(),
            active = commands.loadActive(),
            asr = commands.loadAsr(),
            webSearch = commands.loadWebSearch(),
        )
    }
}
