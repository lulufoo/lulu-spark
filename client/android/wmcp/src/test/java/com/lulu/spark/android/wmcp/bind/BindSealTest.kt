package com.lulu.spark.android.wmcp.bind

import org.bouncycastle.crypto.params.X25519PrivateKeyParameters
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.security.SecureRandom

class BindSealTest {
    @Test
    fun sealRoundTripsDeviceId() {
        val host = X25519PrivateKeyParameters(SecureRandom())
        val tempPub = host.generatePublicKey().encoded.toHex()
        val sealed = sealBindRequest(tempPub, "phone-1", "Pixel")
        assertTrue(sealed.size >= 32 + 12 + 16)
        val opened = openBindRequest(host.encoded, sealed)
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
