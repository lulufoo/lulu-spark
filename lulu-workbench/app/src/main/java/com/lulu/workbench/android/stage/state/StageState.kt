package com.lulu.workbench.android.stage.state

import com.lulu.workbench.android.agent.tools.stage.StagedItem

data class StageState(
    val currentSessionId: String? = null,
    val items: List<StagedItem> = emptyList(),
    val opened: StagedItem? = null,
    val openedFromList: Boolean = false,
)

sealed class StageIntent {
    data class Open(val id: String) : StageIntent()

    data object CloseFile : StageIntent()

    data class Save(val title: String, val body: String) : StageIntent()

    data object Delete : StageIntent()
}
