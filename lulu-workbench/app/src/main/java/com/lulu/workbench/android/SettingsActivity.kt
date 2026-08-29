package com.lulu.workbench.android

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import com.lulu.workbench.android.bind.commands.BindCommands
import com.lulu.workbench.android.bind.state.BindIntent
import com.lulu.workbench.android.bind.state.BindStore
import com.lulu.workbench.android.bind.ui.QrScanPane
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.settings.commands.SettingsCommands
import com.lulu.workbench.android.settings.state.SettingsIntent
import com.lulu.workbench.android.settings.state.SettingsStore
import com.lulu.workbench.android.settings.ui.SettingsScreen
import com.lulu.workbench.android.ui.theme.LuLuWorkbenchTheme

class SettingsActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WbLog.module(LogModule.APP).i("settings activity create")
        enableEdgeToEdge()
        val runtime = (application as WorkbenchApp).runtime
        setContent {
            val mainHandler = remember { Handler(Looper.getMainLooper()) }
            val settingsStore = remember { SettingsStore(SettingsCommands(runtime.llm)) }
            val bindStore = remember {
                BindStore(
                    BindCommands(runtime.wmcp, Build.MODEL),
                    runOffMain = { block -> Thread { block() }.start() },
                    runOnMain = { block -> mainHandler.post { block() } },
                )
            }
            val cameraLauncher = rememberLauncherForActivityResult(
                ActivityResultContracts.RequestPermission(),
            ) { granted ->
                if (granted) bindStore.dispatch(BindIntent.StartScan)
                else bindStore.dispatch(BindIntent.CameraDenied)
            }
            fun goBack() {
                when (settingsBackAction(bindStore.state.scanning)) {
                    SettingsBackAction.CancelScan ->
                        bindStore.dispatch(BindIntent.CancelScan)
                    SettingsBackAction.FinishSettings -> finish()
                }
            }
            LuLuWorkbenchTheme {
                BackHandler { goBack() }
                Scaffold(
                    modifier = Modifier.fillMaxSize(),
                    containerColor = MaterialTheme.colorScheme.background,
                    contentWindowInsets = WindowInsets(0, 0, 0, 0),
                ) { _ ->
                    val pane = Modifier.fillMaxSize()
                    if (bindStore.state.scanning) {
                        QrScanPane(
                            onQr = { qr -> bindStore.dispatch(BindIntent.Scanned(qr)) },
                            onCancel = { bindStore.dispatch(BindIntent.CancelScan) },
                            modifier = pane,
                        )
                    } else {
                        LaunchedEffect(Unit) {
                            settingsStore.dispatch(SettingsIntent.Load)
                            bindStore.dispatch(BindIntent.Query)
                        }
                        SettingsScreen(
                            state = settingsStore.state,
                            bindState = bindStore.state,
                            onBack = { goBack() },
                            onSelect = { id ->
                                settingsStore.dispatch(SettingsIntent.Select(id))
                            },
                            onSave = { baseUrl, model, apiKey ->
                                settingsStore.dispatch(
                                    SettingsIntent.Save(baseUrl, model, apiKey),
                                )
                            },
                            onReset = { settingsStore.dispatch(SettingsIntent.Reset) },
                            onStartScan = {
                                val granted = ContextCompat.checkSelfPermission(
                                    this@SettingsActivity,
                                    Manifest.permission.CAMERA,
                                ) == PackageManager.PERMISSION_GRANTED
                                if (granted) bindStore.dispatch(BindIntent.StartScan)
                                else cameraLauncher.launch(Manifest.permission.CAMERA)
                            },
                            modifier = pane,
                        )
                    }
                }
            }
        }
    }
}
