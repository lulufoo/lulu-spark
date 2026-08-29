package com.lulu.workbench.android

import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import com.lulu.workbench.android.chat.commands.ChatCommands
import com.lulu.workbench.android.chat.state.ChatStore
import com.lulu.workbench.android.chat.ui.ChatScreen
import com.lulu.workbench.android.chat.ui.applyChatWindowIme
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.ui.theme.LuLuWorkbenchTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen()
        super.onCreate(savedInstanceState)
        WbLog.module(LogModule.APP).i("activity create")
        enableEdgeToEdge()
        applyChatWindowIme(window)
        val runtime = (application as WorkbenchApp).runtime
        setContent {
            val mainHandler = remember { Handler(Looper.getMainLooper()) }
            val chatStore = remember {
                ChatStore(
                    ChatCommands(runtime.agent),
                    runOffMain = { block -> Thread { block() }.start() },
                    runOnMain = { block -> mainHandler.post { block() } },
                )
            }
            LuLuWorkbenchTheme {
                Scaffold(
                    modifier = Modifier.fillMaxSize(),
                    containerColor = MaterialTheme.colorScheme.background,
                    contentWindowInsets = WindowInsets(0, 0, 0, 0),
                ) { _ ->
                    ChatScreen(
                        store = chatStore,
                        onOpenSettings = {
                            startActivity(Intent(this@MainActivity, SettingsActivity::class.java))
                        },
                        modifier = Modifier.fillMaxSize(),
                    )
                }
            }
        }
    }
}
