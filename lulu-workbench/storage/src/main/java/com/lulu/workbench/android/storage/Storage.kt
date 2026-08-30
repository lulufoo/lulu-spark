package com.lulu.workbench.android.storage

import com.lulu.workbench.android.storage.file.FileStorage
import java.io.File

/** Path-and-key store. Does not know sessions. */
interface Storage {
    fun read(path: String): ByteArray?

    fun write(path: String, bytes: ByteArray)

    fun delete(path: String)

    fun list(prefix: String): List<String>

    fun getSecret(key: String): String?

    fun putSecret(key: String, value: String)

    fun deleteSecret(key: String)
}

class MemoryStorage : Storage {
    private val files = mutableMapOf<String, ByteArray>()
    private val secrets = mutableMapOf<String, String>()

    override fun read(path: String): ByteArray? = files[path]

    override fun write(path: String, bytes: ByteArray) {
        files[path] = bytes
    }

    override fun delete(path: String) {
        files.remove(path)
    }

    override fun list(prefix: String): List<String> {
        val root = prefix.trimEnd('/')
        return files.keys.filter { it == root || it.startsWith("$root/") }.sorted()
    }

    override fun getSecret(key: String): String? = secrets[key]

    override fun putSecret(key: String, value: String) {
        secrets[key] = value
    }

    override fun deleteSecret(key: String) {
        secrets.remove(key)
    }
}

object StorageFactory {
    fun create(root: File): Storage = FileStorage(root)

    fun createMemory(): Storage = MemoryStorage()
}
