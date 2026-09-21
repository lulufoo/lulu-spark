package com.lulu.workbench.android.chat

import android.os.Handler
import android.os.Looper
import androidx.lifecycle.ViewModel
import com.lulu.workbench.android.agent.facade.WorkbenchRuntime
import com.lulu.workbench.android.chat.commands.ChatCommands
import com.lulu.workbench.android.chat.state.ChatStore
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject

@HiltViewModel
class ChatViewModel @Inject constructor(
    commands: ChatCommands,
    runtime: WorkbenchRuntime,
) : ViewModel() {
    val store: ChatStore

    init {
        val keepAlive = runtime.wmcp.keepAlive()
        store = ChatStore(
            commands,
            keepAlive = keepAlive,
            runOffMain = { block -> Thread { block() }.start() },
            runOnMain = { block -> Handler(Looper.getMainLooper()).post(block) },
        )
        keepAlive.start()
    }
}
