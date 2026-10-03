package com.lulu.spark.android

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.lulu.spark.android.log.LogModule
import com.lulu.spark.android.log.WbLog
import com.lulu.spark.android.stage.StageViewModel
import com.lulu.spark.android.stage.state.StageIntent
import com.lulu.spark.android.stage.ui.StageFileScreen
import com.lulu.spark.android.stage.ui.StageListScreen
import com.lulu.spark.android.ui.theme.LuLuWorkbenchTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class StageActivity : ComponentActivity() {
    private val stageViewModel: StageViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WbLog.module(LogModule.APP).i("stage activity create")
        enableEdgeToEdge()
        setContent {
            val state by stageViewModel.state.collectAsStateWithLifecycle()
            fun goBack() {
                when (stageBackAction(state.opened != null, state.openedFromList)) {
                    StageBackAction.CloseFile -> stageViewModel.dispatch(StageIntent.CloseFile)
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
                    val opened = state.opened
                    if (opened != null) {
                        StageFileScreen(
                            item = opened,
                            onBack = { goBack() },
                            onSave = { title, body ->
                                stageViewModel.dispatch(StageIntent.Save(title, body))
                            },
                            onDelete = {
                                val leave =
                                    stageBackAction(true, state.openedFromList) ==
                                        StageBackAction.FinishStage
                                stageViewModel.dispatch(StageIntent.Delete)
                                if (leave) finish()
                            },
                            modifier = pane,
                        )
                    } else {
                        StageListScreen(
                            state = state,
                            onBack = { goBack() },
                            onOpen = { id -> stageViewModel.dispatch(StageIntent.Open(id)) },
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
