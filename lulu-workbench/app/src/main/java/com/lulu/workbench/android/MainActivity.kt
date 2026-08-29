package com.lulu.workbench.android

import android.Manifest
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import com.lulu.workbench.android.agent.facade.WorkbenchRuntime
import com.lulu.workbench.android.bind.commands.BindCommands
import com.lulu.workbench.android.bind.state.BindIntent
import com.lulu.workbench.android.bind.state.BindStore
import com.lulu.workbench.android.bind.ui.QrScanPane
import com.lulu.workbench.android.chat.commands.ChatCommands
import com.lulu.workbench.android.chat.state.ChatStore
import com.lulu.workbench.android.chat.ui.ChatScreen
import com.lulu.workbench.android.settings.commands.SettingsCommands
import com.lulu.workbench.android.settings.state.SettingsIntent
import com.lulu.workbench.android.settings.state.SettingsStore
import com.lulu.workbench.android.settings.ui.SettingsScreen
import com.lulu.workbench.android.log.AndroidLogEnv
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.ui.theme.LuLuWorkbenchTheme

private enum class AppDest {
    Chat,
    Settings,
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val debug = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
        WbLog.start(AndroidLogEnv(this, filesDir, debug))
        WbLog.module(LogModule.APP).i("activity create")
        enableEdgeToEdge()
        setContent {
            val runtime = remember { WorkbenchRuntime.create(filesDir) }
            val mainHandler = remember { Handler(Looper.getMainLooper()) }
            val chatStore = remember {
                ChatStore(
                    ChatCommands(runtime.agent),
                    runOffMain = { block -> Thread { block() }.start() },
                    runOnMain = { block -> mainHandler.post { block() } },
                )
            }
            val settingsStore = remember { SettingsStore(SettingsCommands(runtime.llm)) }
            val bindStore = remember {
                BindStore(
                    BindCommands(runtime.wmcp, Build.MODEL),
                    runOffMain = { block -> Thread { block() }.start() },
                    runOnMain = { block -> mainHandler.post { block() } },
                )
            }
            var dest by remember { mutableStateOf(AppDest.Chat) }
            val cameraLauncher = rememberLauncherForActivityResult(
                ActivityResultContracts.RequestPermission(),
            ) { granted ->
                if (granted) bindStore.dispatch(BindIntent.StartScan)
                else bindStore.dispatch(BindIntent.CameraDenied)
            }
            LuLuWorkbenchTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    val pane = Modifier.padding(innerPadding)
                    when (dest) {
                        AppDest.Chat -> ChatScreen(
                            store = chatStore,
                            onOpenSettings = { dest = AppDest.Settings },
                            modifier = pane,
                        )
                        AppDest.Settings -> {
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
                                    onBack = { dest = AppDest.Chat },
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
                                            this@MainActivity,
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
    }
}
