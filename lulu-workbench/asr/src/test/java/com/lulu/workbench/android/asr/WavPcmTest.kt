package com.lulu.workbench.android.asr

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WavPcmTest {
    @Test
    fun headerIsRiffWavePcm16kMono() {
        val wav = pcmToWav(byteArrayOf(1, 2, 3, 4))
        assertEquals("RIFF", wav.decodeToString(0, 4))
        assertEquals("WAVE", wav.decodeToString(8, 12))
        assertEquals(1, wav[20].toInt() and 0xFF)
        assertEquals(1, wav[22].toInt() and 0xFF)
        assertEquals(AsrPcmSampleRate, readInt32Le(wav, 24))
        assertEquals(4, readInt32Le(wav, 40))
        assertEquals(1.toByte(), wav[44])
        assertEquals(4.toByte(), wav[47])
    }

    @Test
    fun tooShortUsesHeaderPlusMinimumPcm() {
        assertTrue(isVoiceTooShort(pcmToWav(ByteArray(0))))
        assertTrue(isVoiceTooShort(pcmToWav(ByteArray(MinVoicePcmBytes - 1))))
        assertFalse(isVoiceTooShort(pcmToWav(ByteArray(MinVoicePcmBytes))))
    }

    @Test
    fun maxClipIs56SecondsOfPcm16kMono() {
        assertEquals(1_792_000, MaxVoicePcmBytes)
        assertEquals(MaxVoiceSeconds, MaxVoicePcmBytes / (AsrPcmSampleRate * 2))
    }

    private fun readInt32Le(bytes: ByteArray, at: Int): Int =
        (bytes[at].toInt() and 0xFF) or
            ((bytes[at + 1].toInt() and 0xFF) shl 8) or
            ((bytes[at + 2].toInt() and 0xFF) shl 16) or
            ((bytes[at + 3].toInt() and 0xFF) shl 24)
}
