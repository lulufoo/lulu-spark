package com.lulu.workbench.android.wmcp.keepalive

internal class ManualLinkClock : LinkKeepAliveClock {
    private val delayed = mutableListOf<Pair<Long, () -> Unit>>()
    var nowMs: Long = 0
        private set
    var executeCount: Int = 0
        private set

    override fun execute(task: () -> Unit) {
        executeCount += 1
        task()
    }

    override fun schedule(delayMs: Long, task: () -> Unit): CancelHandle {
        val item = (nowMs + delayMs) to task
        delayed.add(item)
        return CancelHandle { delayed.remove(item) }
    }

    fun pending(): Int = delayed.size

    fun advance(ms: Long) {
        nowMs += ms
        val due = delayed.filter { it.first <= nowMs }
        delayed.removeAll(due.toSet())
        due.forEach { it.second() }
    }
}
