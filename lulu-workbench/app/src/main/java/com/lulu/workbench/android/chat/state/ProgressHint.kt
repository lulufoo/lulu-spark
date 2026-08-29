package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.agent.loop.TurnProgress

fun progressHint(progress: TurnProgress): String =
    when (progress) {
        TurnProgress.CallingLlm -> "Requesting…"
        is TurnProgress.CallingTool -> "Calling ${progress.name}…"
        is TurnProgress.Finished -> ""
    }
