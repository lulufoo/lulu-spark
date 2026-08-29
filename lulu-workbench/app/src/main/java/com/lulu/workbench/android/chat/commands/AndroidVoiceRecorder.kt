package com.lulu.workbench.android.chat.commands

import android.annotation.SuppressLint
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import com.lulu.workbench.android.asr.AsrPcmSampleRate
import com.lulu.workbench.android.asr.MaxVoicePcmBytes
import com.lulu.workbench.android.asr.pcmToWav
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import java.io.ByteArrayOutputStream
import kotlin.math.max

private const val ReadBytes = 4_096

class AndroidVoiceRecorder : VoiceRecorder {
    private val pcm = ByteArrayOutputStream()
    private var record: AudioRecord? = null
    private var reader: Thread? = null

    @SuppressLint("MissingPermission")
    override fun start() {
        discard()
        val min = AudioRecord.getMinBufferSize(
            AsrPcmSampleRate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
        )
        if (min <= 0) throw IllegalStateException("mic")
        val rec = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            AsrPcmSampleRate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
            max(min, ReadBytes),
        )
        if (rec.state != AudioRecord.STATE_INITIALIZED) {
            rec.release()
            throw IllegalStateException("mic")
        }
        rec.startRecording()
        record = rec
        reader = Thread({
            val buf = ByteArray(ReadBytes)
            while (true) {
                val n = rec.read(buf, 0, buf.size)
                if (n <= 0) break
                synchronized(pcm) {
                    if (pcm.size() >= MaxVoicePcmBytes) return@Thread
                    pcm.write(buf, 0, n)
                }
            }
        }, "wb-asr").also { it.start() }
    }

    override fun stop(): ByteArray {
        val rec = record
        record = null
        try {
            rec?.stop()
        } catch (error: Exception) {
            log.w("stop ${error.javaClass.simpleName}")
        }
        rec?.release()
        reader?.join(400)
        reader = null
        val raw = synchronized(pcm) { pcm.toByteArray() }
        pcm.reset()
        return pcmToWav(raw)
    }

    private fun discard() {
        if (record != null || pcm.size() > 0) stop()
    }
}

private val log = WbLog.module(LogModule.ASR)
