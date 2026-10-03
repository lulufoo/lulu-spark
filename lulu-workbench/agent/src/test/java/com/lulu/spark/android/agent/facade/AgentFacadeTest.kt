package com.lulu.spark.android.agent.facade

import com.lulu.spark.android.agent.loop.TurnProgress
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertTrue
import org.junit.Test

class AgentFacadeTest {
    @Test
    fun twoSessionsGetDistinctLoopInstances() {
        val runtime = WorkbenchRuntime.createForTest()
        val first = runtime.agent.createSession()
        val second = runtime.agent.createSession()
        val left = runtime.agent.loop(first)
        val right = runtime.agent.loop(second)
        assertNotSame(left, right)
        assertTrue(left.sessionId != right.sessionId)
    }

    @Test
    fun sendEmitsProgressFromThatLoop() {
        val runtime = WorkbenchRuntime.createForTest()
        val id = runtime.agent.createSession()
        val loop = runtime.agent.loop(id)
        val seen = mutableListOf<TurnProgress>()
        loop.send("hi") { seen.add(it) }
        assertTrue(seen.any { it is TurnProgress.CallingLlm })
        val finished = seen.filterIsInstance<TurnProgress.Finished>().single()
        assertTrue(finished.reply.contains("not configured"))
    }
}
