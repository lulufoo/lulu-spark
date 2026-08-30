package com.lulu.workbench.android.stage.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.stage.commands.StageCommands

class StageStore(
    private val commands: StageCommands,
    currentSessionId: String?,
    initialId: String?,
) {
    var state: StageState by mutableStateOf(load(currentSessionId, initialId))
        private set

    fun dispatch(intent: StageIntent) {
        when (intent) {
            is StageIntent.Open -> {
                val item = commands.get(intent.id) ?: return
                state = state.copy(opened = item, openedFromList = true)
            }
            StageIntent.CloseFile -> {
                state = state.copy(opened = null, openedFromList = false, items = commands.list())
            }
            is StageIntent.Save -> {
                val id = state.opened?.id ?: return
                val saved = commands.update(id, intent.title, intent.body) ?: return
                state = state.copy(opened = saved, items = commands.list())
            }
            StageIntent.Delete -> {
                val id = state.opened?.id ?: return
                if (!commands.delete(id)) return
                state = state.copy(opened = null, openedFromList = false, items = commands.list())
            }
        }
    }

    private fun load(currentSessionId: String?, initialId: String?): StageState {
        val items = commands.list()
        val opened = initialId?.let { commands.get(it) }
        return StageState(
            currentSessionId = currentSessionId,
            items = items,
            opened = opened,
            openedFromList = false,
        )
    }
}
