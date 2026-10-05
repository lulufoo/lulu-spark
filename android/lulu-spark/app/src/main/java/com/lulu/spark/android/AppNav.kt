package com.lulu.spark.android

internal enum class HomeEdgeAction {
    OpenDrawer,
    CloseDrawer,
}

internal fun homeEdgeAction(drawerOpen: Boolean): HomeEdgeAction =
    if (drawerOpen) HomeEdgeAction.CloseDrawer else HomeEdgeAction.OpenDrawer

internal enum class SettingsBackAction {
    FinishSettings,
    CancelScan,
}

internal fun settingsBackAction(scanning: Boolean): SettingsBackAction =
    if (scanning) SettingsBackAction.CancelScan else SettingsBackAction.FinishSettings

internal enum class StageBackAction {
    FinishStage,
    CloseFile,
}

internal fun stageBackAction(fileOpen: Boolean, openedFromList: Boolean): StageBackAction =
    if (fileOpen && openedFromList) StageBackAction.CloseFile else StageBackAction.FinishStage
