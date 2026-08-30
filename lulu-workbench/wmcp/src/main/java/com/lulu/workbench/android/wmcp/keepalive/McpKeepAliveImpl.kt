package com.lulu.workbench.android.wmcp.keepalive

import com.lulu.workbench.android.wmcp.McpKeepAlive
import com.lulu.workbench.android.wmcp.McpLinkListener
import com.lulu.workbench.android.wmcp.McpLinkState
import com.lulu.workbench.android.wmcp.mcp.McpWire
import com.lulu.workbench.android.wmcp.mcp.McpWireEvent
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

internal const val MCP_KEEP_ALIVE_RETRY_MS = 10_000L

internal fun interface CancelHandle {
    fun cancel()
}

internal interface LinkKeepAliveClock {
    fun execute(task: () -> Unit)

    fun schedule(delayMs: Long, task: () -> Unit): CancelHandle
}

internal class ExecutorLinkClock : LinkKeepAliveClock {
    private val worker = Executors.newSingleThreadExecutor { task ->
        Thread(task, "wmcp-keepalive").apply { isDaemon = true }
    }
    private val delayed = Executors.newSingleThreadScheduledExecutor { task ->
        Thread(task, "wmcp-keepalive-delay").apply { isDaemon = true }
    }

    override fun execute(task: () -> Unit) {
        worker.execute(task)
    }

    override fun schedule(delayMs: Long, task: () -> Unit): CancelHandle {
        val future = delayed.schedule(task, delayMs, TimeUnit.MILLISECONDS)
        return CancelHandle { future.cancel(false) }
    }
}

internal class McpKeepAliveImpl(
    private val wire: McpWire,
    private val clock: LinkKeepAliveClock,
    private val retryMs: Long = MCP_KEEP_ALIVE_RETRY_MS,
) : McpKeepAlive {
    private val listeners = CopyOnWriteArrayList<McpLinkListener>()
    private val lock = Any()
    private val started = AtomicBoolean(false)
    private val activating = AtomicBoolean(false)

    @Volatile
    private var current: McpLinkState =
        if (wire.isBound()) McpLinkState.Disconnected else McpLinkState.Unbound

    private var retry: CancelHandle? = null

    init {
        wire.addWireListener(::onWire)
    }

    override fun addListener(listener: McpLinkListener) {
        listeners.add(listener)
        listener.onMcpLink(current)
    }

    override fun removeListener(listener: McpLinkListener) {
        listeners.remove(listener)
    }

    override fun start() {
        if (!started.compareAndSet(false, true)) return
        if (!wire.isBound()) {
            setState(McpLinkState.Unbound)
            return
        }
        setState(McpLinkState.Disconnected)
        activateAsync()
    }

    override fun state(): McpLinkState = current

    private fun onWire(event: McpWireEvent) {
        when (event) {
            McpWireEvent.BindChanged -> onBindChanged()
            McpWireEvent.LinkUp -> onLinkUp()
            McpWireEvent.LinkDown -> onLinkDown()
        }
    }

    private fun onBindChanged() {
        if (!started.get()) return
        if (!wire.isBound()) {
            cancelRetry()
            setState(McpLinkState.Unbound)
            return
        }
        cancelRetry()
        setState(McpLinkState.Disconnected)
        activateAsync()
    }

    private fun onLinkUp() {
        if (!wire.isBound()) {
            cancelRetry()
            setState(McpLinkState.Unbound)
            return
        }
        setState(McpLinkState.Connected)
        if (started.get()) armTimer()
    }

    private fun onLinkDown() {
        if (!wire.isBound()) {
            cancelRetry()
            setState(McpLinkState.Unbound)
            return
        }
        setState(McpLinkState.Disconnected)
        if (started.get()) armTimer()
    }

    private fun activateAsync() {
        tickAsync(useProbe = false)
    }

    private fun tickAsync(useProbe: Boolean) {
        if (!activating.compareAndSet(false, true)) return
        clock.execute {
            try {
                if (!wire.isBound()) {
                    cancelRetry()
                    setState(McpLinkState.Unbound)
                    return@execute
                }
                if (useProbe && current == McpLinkState.Connected) {
                    wire.probe()
                } else {
                    wire.activate()
                }
                onLinkUp()
            } catch (_: Exception) {
                onLinkDown()
            } finally {
                activating.set(false)
            }
        }
    }

    private fun armTimer() {
        synchronized(lock) {
            retry?.cancel()
            retry = clock.schedule(retryMs) {
                tickAsync(useProbe = current == McpLinkState.Connected)
            }
        }
    }

    private fun cancelRetry() {
        synchronized(lock) {
            retry?.cancel()
            retry = null
        }
    }

    private fun setState(next: McpLinkState) {
        if (current == next) return
        current = next
        listeners.forEach { it.onMcpLink(next) }
    }
}
