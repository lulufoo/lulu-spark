package com.lulu.workbench.android

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.lulu.workbench.android.bind.BindViewModel
import com.lulu.workbench.android.bind.state.BindIntent
import com.lulu.workbench.android.bind.ui.QrScanPane
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.settings.SettingsViewModel
import com.lulu.workbench.android.settings.state.SettingsIntent
import com.lulu.workbench.android.settings.ui.SettingsScreen
import com.lulu.workbench.android.ui.theme.LuLuWorkbenchTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class SettingsActivity : ComponentActivity() {
    private val settingsViewModel: SettingsViewModel by viewModels()
    private val bindViewModel: BindViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WbLog.module(LogModule.APP).i("settings activity create")
        enableEdgeToEdge()
        setContent {
            val settingsState by settingsViewModel.state.collectAsStateWithLifecycle()
            val bindState by bindViewModel.state.collectAsStateWithLifecycle()
            val cameraLauncher = rememberLauncherForActivityResult(
                ActivityResultContracts.RequestPermission(),
            ) { granted ->
                if (granted) bindViewModel.dispatch(BindIntent.StartScan)
                else bindViewModel.dispatch(BindIntent.CameraDenied)
            }
            fun goBack() {
                when (settingsBackAction(bindState.scanning)) {
                    SettingsBackAction.CancelScan ->
                        bindViewModel.dispatch(BindIntent.CancelScan)
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
                    if (bindState.scanning) {
                        QrScanPane(
                            onQr = { qr -> bindViewModel.dispatch(BindIntent.Scanned(qr)) },
                            onCancel = { bindViewModel.dispatch(BindIntent.CancelScan) },
                            modifier = pane,
                        )
                    } else {
                        LaunchedEffect(Unit) {
                            bindViewModel.dispatch(BindIntent.Query)
                        }
                        SettingsScreen(
                            state = settingsState,
                            bindState = bindState,
                            onBack = { goBack() },
                            onSelect = { id ->
                                settingsViewModel.dispatch(SettingsIntent.Select(id))
                            },
                            onSave = { baseUrl, model, apiKey ->
                                settingsViewModel.dispatch(
                                    SettingsIntent.Save(baseUrl, model, apiKey),
                                )
                            },
                            onReset = { settingsViewModel.dispatch(SettingsIntent.Reset) },
                            onSaveAsr = { appId, secretId, secretKey ->
                                settingsViewModel.dispatch(
                                    SettingsIntent.SaveAsr(appId, secretId, secretKey),
                                )
                            },
                            onClearAsr = { settingsViewModel.dispatch(SettingsIntent.ClearAsr) },
                            onSaveWebSearch = { apiKey ->
                                settingsViewModel.dispatch(SettingsIntent.SaveWebSearch(apiKey))
                            },
                            onClearWebSearch = {
                                settingsViewModel.dispatch(SettingsIntent.ClearWebSearch)
                            },
                            onStartScan = {
                                val granted = ContextCompat.checkSelfPermission(
                                    this@SettingsActivity,
                                    Manifest.permission.CAMERA,
                                ) == PackageManager.PERMISSION_GRANTED
                                if (granted) bindViewModel.dispatch(BindIntent.StartScan)
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
