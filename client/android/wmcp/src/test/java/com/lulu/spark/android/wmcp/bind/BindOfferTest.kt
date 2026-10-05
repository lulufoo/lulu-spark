package com.lulu.spark.android.wmcp.bind

import com.lulu.spark.android.wmcp.parseBindOffer
import org.junit.Assert.assertEquals
import org.junit.Test

class BindOfferTest {
    @Test
    fun parseOfferReadsQrFields() {
        val offer = parseBindOffer(
            """{"ip":"10.0.0.2","port":7654,"temp_pub":"aa","tls_fingerprint":"ff","exp":1,"sig":"ss"}""",
        )
        assertEquals("10.0.0.2", offer.ip)
        assertEquals(7654, offer.port)
        assertEquals("ff", offer.tlsFingerprint)
    }
}
