package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.agent.loop.TurnProgress
import org.junit.Assert.assertEquals
import org.junit.Test

class ProgressHintTest {
    @Test
    fun mapsLoopProgressLikeMac() {
        assertEquals("Requesting…", progressHint(TurnProgress.CallingLlm))
        assertEquals("Calling read…", progressHint(TurnProgress.CallingTool("read")))
        assertEquals("", progressHint(TurnProgress.Finished("done")))
    }
}
