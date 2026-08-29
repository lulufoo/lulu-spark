package com.lulu.workbench.android.chat.commands

interface VoiceRecorder {
    fun start()

    fun stop(): ByteArray
}

object IdleVoiceRecorder : VoiceRecorder {
    override fun start() {}

    override fun stop(): ByteArray = ByteArray(0)
}
