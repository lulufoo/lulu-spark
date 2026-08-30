package com.lulu.workbench.android.agent.session

fun sessionTitle(turns: List<HistoryTurn>): String {
    val line = turns
        .firstOrNull { it.role == "user" }
        ?.content
        ?.lineSequence()
        ?.firstOrNull()
        ?.trim()
        .orEmpty()
    return if (line.isEmpty()) "New chat" else line.take(40)
}
