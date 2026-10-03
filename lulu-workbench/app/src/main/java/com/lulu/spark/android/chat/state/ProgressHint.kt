package com.lulu.spark.android.chat.state

import com.lulu.spark.android.agent.loop.TurnProgress

fun progressHint(progress: TurnProgress): String =
    when (progress) {
        TurnProgress.CallingLlm -> "Requesting…"
        is TurnProgress.CallingTool -> "Calling ${progress.name}…"
        is TurnProgress.Finished -> ""
    }
