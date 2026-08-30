package com.lulu.workbench.android

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.stage.commands.StageCommands
import com.lulu.workbench.android.stage.state.StageIntent
import com.lulu.workbench.android.stage.state.StageStore
import com.lulu.workbench.android.stage.ui.StageFileScreen
import com.lulu.workbench.android.stage.ui.StageListScreen
import com.lulu.workbench.android.ui.theme.LuLuWorkbenchTheme

class StageActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WbLog.module(LogModule.APP).i("stage activity create")
        enableEdgeToEdge()
        val runtime = (application as WorkbenchApp).runtime
        val sessionId = intent.getStringExtra(EXTRA_SESSION_ID)
        val stagedId = intent.getStringExtra(EXTRA_STAGED_ID)
        setContent {
            val store = remember {
                StageStore(StageCommands(runtime.agent), sessionId, stagedId)
            }
            fun goBack() {
                when (stageBackAction(store.state.opened != null, store.state.openedFromList)) {
                    StageBackAction.CloseFile -> store.dispatch(StageIntent.CloseFile)
                    StageBackAction.FinishStage -> finish()
                }
            }
            LuLuWorkbenchTheme {
                BackHandler { goBack() }
                Scaffold(
                    modifier = Modifier.fillMaxSize(),
                    containerColor = MaterialTheme.colorScheme.background,
                    contentWindowInsets = WindowInsets(0, 0, 0, 0),
                ) { _ ->
                    val pane = Modifier
                        .fillMaxSize()
                        .windowInsetsPadding(WindowInsets.safeDrawing.exclude(WindowInsets.ime))
                    val opened = store.state.opened
                    if (opened != null) {
                        StageFileScreen(
                            item = opened,
                            onBack = { goBack() },
                            onSave = { title, body ->
                                store.dispatch(StageIntent.Save(title, body))
                            },
                            onDelete = {
                                val leave =
                                    stageBackAction(true, store.state.openedFromList) ==
                                        StageBackAction.FinishStage
                                store.dispatch(StageIntent.Delete)
                                if (leave) finish()
                            },
                            modifier = pane,
                        )
                    } else {
                        StageListScreen(
                            state = store.state,
                            onBack = { goBack() },
                            onOpen = { id -> store.dispatch(StageIntent.Open(id)) },
                            modifier = pane,
                        )
                    }
                }
            }
        }
    }

    companion object {
        const val EXTRA_SESSION_ID = "session_id"
        const val EXTRA_STAGED_ID = "staged_id"

        fun listIntent(context: Context, sessionId: String?): Intent =
            Intent(context, StageActivity::class.java).putExtra(EXTRA_SESSION_ID, sessionId)

        fun fileIntent(context: Context, sessionId: String?, stagedId: String): Intent =
            listIntent(context, sessionId).putExtra(EXTRA_STAGED_ID, stagedId)
    }
}
