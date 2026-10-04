package com.lulu.spark.android

import org.junit.Assert.assertEquals
import org.junit.Test

class AppNavTest {
    @Test
    fun homeSwipeOpensHiddenDrawer() {
        assertEquals(HomeEdgeAction.OpenDrawer, homeEdgeAction(drawerOpen = false))
    }

    @Test
    fun homeSwipeClosesOpenDrawer() {
        assertEquals(HomeEdgeAction.CloseDrawer, homeEdgeAction(drawerOpen = true))
    }

    @Test
    fun settingsGestureFinishesSettings() {
        assertEquals(
            SettingsBackAction.FinishSettings,
            settingsBackAction(scanning = false),
        )
    }

    @Test
    fun settingsScanGestureCancelsScan() {
        assertEquals(
            SettingsBackAction.CancelScan,
            settingsBackAction(scanning = true),
        )
    }

    @Test
    fun stagedFileFromListGoesBackToList() {
        assertEquals(
            StageBackAction.CloseFile,
            stageBackAction(fileOpen = true, openedFromList = true),
        )
    }

    @Test
    fun stagedDeepLinkOrListFinishesActivity() {
        assertEquals(
            StageBackAction.FinishStage,
            stageBackAction(fileOpen = true, openedFromList = false),
        )
        assertEquals(
            StageBackAction.FinishStage,
            stageBackAction(fileOpen = false, openedFromList = false),
        )
    }
}
