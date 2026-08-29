package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.asr.MinVoicePcmBytes
import com.lulu.workbench.android.asr.pcmToWav
import com.lulu.workbench.android.chat.commands.ChatCommands
import com.lulu.workbench.android.chat.commands.VoiceRecorder
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatVoiceTest {
    @Test
    fun pressWithoutKeysDoesNotRecordOrHint() {
        val recorder = ScriptedRecorder()
        val store = voiceStore(asrReady = false, recorder = recorder)
        assertFalse(store.state.asrConfigured)
        store.dispatch(ChatIntent.VoicePress)
        assertEquals(VoicePhase.Idle, store.state.voicePhase)
        assertEquals("", store.state.voiceHint)
        assertEquals(0, recorder.starts)
    }

    @Test
    fun refreshAsrUpdatesConfiguredFlag() {
        var ready = false
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_voice") },
                sendTurn = { _, _, _ -> },
                asrReady = { ready },
            ),
        )
        assertFalse(store.state.asrConfigured)
        ready = true
        store.dispatch(ChatIntent.RefreshAsr)
        assertTrue(store.state.asrConfigured)
    }

    @Test
    fun releaseSendsTranscribedText() {
        var sent = ""
        val store = voiceStore(
            recorder = ScriptedRecorder(pcmToWav(ByteArray(MinVoicePcmBytes))),
            transcribe = { "hello voice" },
            sendTurn = { _, text, onProgress ->
                sent = text
                onProgress(
                    com.lulu.workbench.android.agent.loop.TurnProgress.Finished("ok"),
                )
            },
        )
        store.dispatch(ChatIntent.VoicePress)
        assertEquals(VoicePhase.Recording, store.state.voicePhase)
        store.dispatch(ChatIntent.VoiceRelease(cancel = false))
        assertEquals("hello voice", sent)
        assertEquals(VoicePhase.Idle, store.state.voicePhase)
        assertEquals("hello voice", store.state.turns.first().content)
        assertFalse(store.state.inFlight)
    }

    @Test
    fun releaseCancelDoesNotSend() {
        var sends = 0
        val store = voiceStore(
            recorder = ScriptedRecorder(pcmToWav(ByteArray(MinVoicePcmBytes))),
            transcribe = { "nope" },
            sendTurn = { _, _, _ -> sends += 1 },
        )
        store.dispatch(ChatIntent.VoicePress)
        store.dispatch(ChatIntent.VoiceRelease(cancel = true))
        assertEquals(0, sends)
        assertEquals(VoicePhase.Idle, store.state.voicePhase)
        assertEquals("", store.state.voiceHint)
    }

    @Test
    fun shortClipSetsHint() {
        var sends = 0
        val store = voiceStore(
            recorder = ScriptedRecorder(pcmToWav(ByteArray(8))),
            transcribe = { "nope" },
            sendTurn = { _, _, _ -> sends += 1 },
        )
        store.dispatch(ChatIntent.VoicePress)
        store.dispatch(ChatIntent.VoiceRelease(cancel = false))
        assertEquals(0, sends)
        assertEquals("Too short", store.state.voiceHint)
    }

    @Test
    fun recognizeFailureStaysIdle() {
        val store = voiceStore(
            recorder = ScriptedRecorder(pcmToWav(ByteArray(MinVoicePcmBytes))),
            transcribe = { error("boom") },
        )
        store.dispatch(ChatIntent.VoicePress)
        store.dispatch(ChatIntent.VoiceRelease(cancel = false))
        assertEquals(VoicePhase.Idle, store.state.voicePhase)
        assertEquals("Recognition failed", store.state.voiceHint)
        assertFalse(store.state.inFlight)
        assertTrue(store.state.turns.isEmpty())
    }

    @Test
    fun pressIgnoredWhileInFlight() {
        val recorder = ScriptedRecorder()
        val store = voiceStore(
            recorder = recorder,
            sendTurn = { _, _, onProgress ->
                onProgress(com.lulu.workbench.android.agent.loop.TurnProgress.CallingLlm)
            },
        )
        store.dispatch(ChatIntent.Send("typed"))
        store.dispatch(ChatIntent.VoicePress)
        assertEquals(0, recorder.starts)
        assertEquals(VoicePhase.Idle, store.state.voicePhase)
    }

    @Test
    fun micDeniedSetsHint() {
        val store = voiceStore()
        store.dispatch(ChatIntent.MicDenied)
        assertEquals("Microphone permission denied", store.state.voiceHint)
    }

    private fun voiceStore(
        asrReady: Boolean = true,
        recorder: VoiceRecorder = ScriptedRecorder(),
        transcribe: (ByteArray) -> String = { "" },
        sendTurn: (SessionId, String, (com.lulu.workbench.android.agent.loop.TurnProgress) -> Unit) -> Unit =
            { _, _, _ -> },
    ): ChatStore =
        ChatStore(
            ChatCommands(
                create = { SessionId("sess_voice") },
                sendTurn = sendTurn,
                asrReady = { asrReady },
                transcribe = transcribe,
                recorder = recorder,
            ),
        )
}

private class ScriptedRecorder(
    private val wav: ByteArray = pcmToWav(ByteArray(MinVoicePcmBytes)),
) : VoiceRecorder {
    var starts = 0

    override fun start() {
        starts += 1
    }

    override fun stop(): ByteArray = wav
}
