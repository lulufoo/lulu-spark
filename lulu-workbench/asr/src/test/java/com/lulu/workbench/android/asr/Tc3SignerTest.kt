package com.lulu.workbench.android.asr

import org.junit.Assert.assertEquals
import org.junit.Test

class Tc3SignerTest {
    @Test
    fun signsSentencePayloadWithFixedClock() {
        val signed = signTc3(
            Tc3SignRequest(
                secretId = "AKIDtestsecretid",
                secretKey = "testsecretkey",
                service = "asr",
                host = "asr.tencentcloudapi.com",
                payload =
                    """{"EngSerViceType":"16k_zh","SourceType":1,"VoiceFormat":"wav","Data":"dGVzdA==","DataLen":4}""",
                timestampSeconds = 1_700_000_000L,
            ),
        )
        assertEquals("2023-11-14", signed.date)
        assertEquals("1700000000", signed.timestamp)
        assertEquals(
            "TC3-HMAC-SHA256 Credential=AKIDtestsecretid/2023-11-14/asr/tc3_request, " +
                "SignedHeaders=content-type;host, " +
                "Signature=082e372a1d206aa438145ba5a3a38e4dad27aaa730b616087b33cd21de37533e",
            signed.authorization,
        )
    }
}
