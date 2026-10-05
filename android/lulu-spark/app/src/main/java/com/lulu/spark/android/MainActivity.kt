package com.lulu.spark.android

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.ui.Modifier
import com.lulu.spark.android.chat.ChatViewModel
import com.lulu.spark.android.chat.ui.ChatScreen
import com.lulu.spark.android.chat.ui.applyChatWindowIme
import com.lulu.spark.android.log.LogModule
import com.lulu.spark.android.log.WbLog
import com.lulu.spark.android.ui.theme.LuLuSparkTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    private val chatViewModel: ChatViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen()
        super.onCreate(savedInstanceState)
        WbLog.module(LogModule.APP).i("activity create")
        enableEdgeToEdge()
        applyChatWindowIme(window)
        setContent {
            LuLuSparkTheme {
                Scaffold(
                    modifier = Modifier.fillMaxSize(),
                    containerColor = MaterialTheme.colorScheme.background,
                    contentWindowInsets = WindowInsets(0, 0, 0, 0),
                ) { _ ->
                    ChatScreen(
                        viewModel = chatViewModel,
                        onOpenSettings = {
                            startActivity(Intent(this@MainActivity, SettingsActivity::class.java))
                        },
                        onOpenStaged = { id ->
                            startActivity(
                                StageActivity.fileIntent(
                                    this@MainActivity,
                                    chatViewModel.state.value.sessionId,
                                    id,
                                ),
                            )
                        },
                        onOpenStagedAll = {
                            startActivity(
                                StageActivity.listIntent(
                                    this@MainActivity,
                                    chatViewModel.state.value.sessionId,
                                ),
                            )
                        },
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            }
        }
    }
}
