package com.lulu.workbench.android.chat.ui

import com.lulu.workbench.android.chat.state.VoicePhase
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatHoldToTalkTest {
    @Test
    fun slideUpArmsCancel() {
        assertFalse(voiceCancelArmed(fingerY = 80f, originY = 80f, slopPx = 56f))
        assertFalse(voiceCancelArmed(fingerY = 40f, originY = 80f, slopPx = 56f))
        assertTrue(voiceCancelArmed(fingerY = 20f, originY = 80f, slopPx = 56f))
    }

    @Test
    fun holdDisabledWhenAsrMissing() {
        assertFalse(
            voiceHoldEnabled(
                inFlight = false,
                phase = VoicePhase.Idle,
                asrConfigured = false,
            ),
        )
        assertTrue(
            voiceHoldEnabled(
                inFlight = false,
                phase = VoicePhase.Idle,
                asrConfigured = true,
            ),
        )
    }

    @Test
    fun toastOnlyWhenSwitchingToVoiceWithoutKeys() {
        assertTrue(shouldToastVoiceSwitch(asrConfigured = false, toVoice = true))
        assertFalse(shouldToastVoiceSwitch(asrConfigured = true, toVoice = true))
        assertFalse(shouldToastVoiceSwitch(asrConfigured = false, toVoice = false))
    }
}
