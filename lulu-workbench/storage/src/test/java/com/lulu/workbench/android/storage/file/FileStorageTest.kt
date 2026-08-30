package com.lulu.workbench.android.storage.file

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.nio.file.Files

class FileStorageTest {
    @Test
    fun writeReadDeleteFileAndPlainSecret() {
        val root = Files.createTempDirectory("wb-store").toFile()
        val store = FileStorage(root)
        store.write("notes/a.txt", "hello".encodeToByteArray())
        assertArrayEquals("hello".encodeToByteArray(), store.read("notes/a.txt"))
        store.putSecret("llm_api_key", "secret")
        assertEquals("secret", store.getSecret("llm_api_key"))
        store.delete("notes/a.txt")
        store.deleteSecret("llm_api_key")
        assertNull(store.read("notes/a.txt"))
        assertNull(store.getSecret("llm_api_key"))
    }

    @Test
    fun listReturnsFilesUnderPrefix() {
        val root = Files.createTempDirectory("wb-store").toFile()
        val store = FileStorage(root)
        store.write("notes/a.txt", "a".encodeToByteArray())
        store.write("notes/b.txt", "b".encodeToByteArray())
        assertEquals(listOf("notes/a.txt", "notes/b.txt"), store.list("notes"))
        assertEquals(listOf("notes/a.txt"), store.list("notes/a.txt"))
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsParentTraversal() {
        val root = Files.createTempDirectory("wb-store").toFile()
        FileStorage(root).read("../escape")
    }
}
