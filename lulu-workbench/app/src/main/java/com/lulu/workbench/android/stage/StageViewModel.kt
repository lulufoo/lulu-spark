package com.lulu.workbench.android.stage

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import com.lulu.workbench.android.StageActivity
import com.lulu.workbench.android.stage.commands.StageCommands
import com.lulu.workbench.android.stage.state.StageIntent
import com.lulu.workbench.android.stage.state.StageState
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

@HiltViewModel
class StageViewModel(
    private val commands: StageCommands,
    currentSessionId: String?,
    initialId: String?,
) : ViewModel() {
    @Inject
    constructor(
        commands: StageCommands,
        savedState: SavedStateHandle,
    ) : this(
        commands,
        savedState.get<String>(StageActivity.EXTRA_SESSION_ID),
        savedState.get<String>(StageActivity.EXTRA_STAGED_ID),
    )

    private val _state = MutableStateFlow(restore(currentSessionId, initialId))
    val state: StateFlow<StageState> = _state.asStateFlow()

    fun dispatch(intent: StageIntent) {
        when (intent) {
            is StageIntent.Open -> openItem(intent.id)
            StageIntent.CloseFile ->
                _state.update {
                    it.copy(opened = null, openedFromList = false, items = commands.list())
                }
            is StageIntent.Save -> saveOpened(intent.title, intent.body)
            StageIntent.Delete -> deleteOpened()
        }
    }

    private fun openItem(id: String) {
        val item = commands.get(id) ?: return
        _state.update { it.copy(opened = item, openedFromList = true) }
    }

    private fun saveOpened(title: String, body: String) {
        val id = _state.value.opened?.id ?: return
        val saved = commands.update(id, title, body) ?: return
        _state.update { it.copy(opened = saved, items = commands.list()) }
    }

    private fun deleteOpened() {
        val id = _state.value.opened?.id ?: return
        if (!commands.delete(id)) return
        _state.update { it.copy(opened = null, openedFromList = false, items = commands.list()) }
    }

    private fun restore(currentSessionId: String?, initialId: String?): StageState {
        val opened = initialId?.let { commands.get(it) }
        return StageState(
            currentSessionId = currentSessionId,
            items = commands.list(),
            opened = opened,
            openedFromList = false,
        )
    }
}
