package com.lulu.spark.android.asr.tencent

import com.lulu.spark.android.asr.AsrApiException
import com.lulu.spark.android.asr.AsrAudioFormat
import com.lulu.spark.android.asr.AsrException
import com.lulu.spark.android.asr.AsrNotConfiguredException

import com.lulu.spark.android.network.HttpRequest
import com.lulu.spark.android.network.HttpResponse
import com.lulu.spark.android.network.NetworkClient
import com.lulu.spark.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AsrClientTest {
    @Test
    fun base64MatchesRfc4648() {
        assertEquals("dGVzdA==", encodeBase64("test".toByteArray()))
        assertEquals("YQ==", encodeBase64("a".toByteArray()))
    }

    @Test(expected = AsrNotConfiguredException::class)
    fun recognizeWithoutKeyFails() {
        AsrClientImpl(MemoryStorage(), RejectNetwork()).recognize("hi".toByteArray())
    }

    @Test
    fun saveKeepsSecretWhenBlankAndClearRemovesIt() {
        val storage = MemoryStorage()
        val asr = AsrClientImpl(storage, RejectNetwork())
        asr.saveConfig("  1250000000  ", "  AKIDABC  ", "  secret  ")
        val saved = asr.loadConfig()
        assertEquals("1250000000", saved.appId)
        assertEquals("AKIDABC", saved.secretId)
        assertTrue(saved.hasSecretKey)
        assertTrue(asr.isConfigured())
        asr.saveConfig("1250000000", "AKIDABC", "")
        assertTrue(asr.loadConfig().hasSecretKey)
        asr.clearConfig()
        assertFalse(asr.isConfigured())
        assertEquals("", asr.loadConfig().appId)
    }

    @Test
    fun recognizePostsSignedSentenceRequest() {
        val storage = MemoryStorage()
        val network = ScriptedNetwork(
            HttpResponse(
                status = 200,
                body = """{"Response":{"Result":"你好","RequestId":"req-1"}}""".encodeToByteArray(),
            ),
        )
        val asr = AsrClientImpl(storage, network, nowSeconds = { 1_700_000_000L })
        asr.saveConfig("125", "AKIDtestsecretid", "testsecretkey")
        val result = asr.recognize("test".toByteArray(), AsrAudioFormat.Wav)
        assertEquals("你好", result.text)
        assertEquals("req-1", result.requestId)
        val sent = network.last!!
        assertEquals("POST", sent.method)
        assertEquals(AsrUrl, sent.url)
        assertEquals(AsrAction, sent.headers["X-TC-Action"])
        assertEquals(AsrVersion, sent.headers["X-TC-Version"])
        assertEquals("1700000000", sent.headers["X-TC-Timestamp"])
        assertEquals(
            "TC3-HMAC-SHA256 Credential=AKIDtestsecretid/2023-11-14/asr/tc3_request, " +
                "SignedHeaders=content-type;host, " +
                "Signature=082e372a1d206aa438145ba5a3a38e4dad27aaa730b616087b33cd21de37533e",
            sent.headers["Authorization"],
        )
        assertEquals(
            """{"EngSerViceType":"16k_zh","SourceType":1,"VoiceFormat":"wav","Data":"dGVzdA==","DataLen":4}""",
            sent.body!!.decodeToString(),
        )
    }

    @Test(expected = AsrApiException::class)
    fun recognizeMapsTencentError() {
        val storage = MemoryStorage()
        val asr = AsrClientImpl(
            storage,
            ScriptedNetwork(
                HttpResponse(
                    status = 200,
                    body = """{"Response":{"Error":{"Code":"AuthFailure","Message":"bad"}}}"""
                        .encodeToByteArray(),
                ),
            ),
        )
        asr.saveConfig("125", "AKID", "key")
        asr.recognize("x".toByteArray())
    }

    @Test(expected = AsrException::class)
    fun recognizeRejectsEmptyAudio() {
        val asr = AsrClientImpl(MemoryStorage(), RejectNetwork())
        asr.saveConfig("125", "AKID", "key")
        asr.recognize(ByteArray(0))
    }
}

private class RejectNetwork : NetworkClient {
    override fun execute(request: HttpRequest): HttpResponse {
        throw AssertionError("network should not be called")
    }
}

private class ScriptedNetwork(
    private val response: HttpResponse,
) : NetworkClient {
    var last: HttpRequest? = null

    override fun execute(request: HttpRequest): HttpResponse {
        last = request
        return response
    }
}
