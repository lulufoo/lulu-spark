package com.lulu.workbench.android.network.okhttp

import com.lulu.workbench.android.network.HttpRequest
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test

class OkHttpNetworkClientTest {
    @Test
    fun executeReturnsStatusAndBody() {
        val server = MockWebServer()
        server.enqueue(MockResponse().setBody("pong").addHeader("mcp-session-id", "s1"))
        server.start()
        try {
            val client = OkHttpNetworkClient()
            val response = client.execute(
                HttpRequest(method = "GET", url = server.url("/").toString()),
            )
            assertEquals(200, response.status)
            assertArrayEquals("pong".encodeToByteArray(), response.body)
            assertEquals("s1", response.headers["mcp-session-id"])
        } finally {
            server.shutdown()
        }
    }

    @Test
    fun defaultClientAllowsSixtySecondRead() {
        val client = defaultHttpClient()
        assertEquals(60_000, client.readTimeoutMillis.toLong())
        assertEquals(60_000, client.writeTimeoutMillis.toLong())
        assertEquals(15_000, client.connectTimeoutMillis.toLong())
        assertEquals(0, client.callTimeoutMillis.toLong())
    }

    @Test(expected = UnsupportedOperationException::class)
    fun executeStreamIsReserved() {
        OkHttpNetworkClient().executeStream(
            HttpRequest(method = "GET", url = "http://127.0.0.1/"),
        ) {}
    }
}
