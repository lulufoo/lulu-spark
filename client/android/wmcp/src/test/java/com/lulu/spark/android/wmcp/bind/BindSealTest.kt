package com.lulu.spark.android.wmcp.bind

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BindSealTest {
    @Test
    fun sealRoundTripsDeviceId() {
        val pair = generateBindRsa()
        val tempPub = tempPubHex(pair)
        val sealed = sealBindRequest(tempPub, "phone-1", "Pixel")
        assertTrue(sealed.size >= 256 + 12 + 16)
        val opened = openBindRequest(privatePkcs8(pair), sealed)
        assertEquals("1", opened.v)
        assertEquals("phone-1", opened.deviceId)
        assertEquals("Pixel", opened.deviceLabel)
    }

    @Test
    fun expiredOfferIsDetected() {
        assertTrue(bindOfferExpired(1, 2))
        assertFalse(bindOfferExpired(10, 10))
        assertFalse(bindOfferExpired(10, 9))
    }
}
