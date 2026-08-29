package com.lulu.workbench.android.chat.state

data class ChatState(
    val sessionId: String? = null,
    val progress: String = "",
    val lastReply: String = "",
    val inFlight: Boolean = false,
)

sealed class ChatIntent {
    data object NewSession : ChatIntent()

    data class Send(val text: String) : ChatIntent()
}
