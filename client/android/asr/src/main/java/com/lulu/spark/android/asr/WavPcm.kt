package com.lulu.spark.android.asr

const val AsrPcmSampleRate = 16_000
const val AsrWavHeaderBytes = 44
const val MinVoicePcmBytes = 3_200
const val MaxVoiceSeconds = 56
const val MaxVoicePcmBytes = AsrPcmSampleRate * 2 * MaxVoiceSeconds

fun pcmToWav(
    pcm: ByteArray,
    sampleRate: Int = AsrPcmSampleRate,
    channels: Int = 1,
    bitsPerSample: Int = 16,
): ByteArray {
    val byteRate = sampleRate * channels * bitsPerSample / 8
    val out = ByteArray(AsrWavHeaderBytes + pcm.size)
    "RIFF".encodeToByteArray().copyInto(out, 0)
    writeInt32Le(out, 4, 36 + pcm.size)
    "WAVE".encodeToByteArray().copyInto(out, 8)
    "fmt ".encodeToByteArray().copyInto(out, 12)
    writeInt32Le(out, 16, 16)
    writeInt16Le(out, 20, 1)
    writeInt16Le(out, 22, channels)
    writeInt32Le(out, 24, sampleRate)
    writeInt32Le(out, 28, byteRate)
    writeInt16Le(out, 32, channels * bitsPerSample / 8)
    writeInt16Le(out, 34, bitsPerSample)
    "data".encodeToByteArray().copyInto(out, 36)
    writeInt32Le(out, 40, pcm.size)
    pcm.copyInto(out, AsrWavHeaderBytes)
    return out
}

fun isVoiceTooShort(wav: ByteArray): Boolean =
    wav.size < AsrWavHeaderBytes + MinVoicePcmBytes

private fun writeInt16Le(target: ByteArray, at: Int, value: Int) {
    target[at] = value.toByte()
    target[at + 1] = (value shr 8).toByte()
}

private fun writeInt32Le(target: ByteArray, at: Int, value: Int) {
    target[at] = value.toByte()
    target[at + 1] = (value shr 8).toByte()
    target[at + 2] = (value shr 16).toByte()
    target[at + 3] = (value shr 24).toByte()
}
