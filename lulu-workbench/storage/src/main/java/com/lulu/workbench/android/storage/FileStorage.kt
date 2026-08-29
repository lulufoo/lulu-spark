package com.lulu.workbench.android.storage

import java.io.File
import java.nio.file.Path

/**
 * Private-directory files. Secrets are plaintext on purpose.
 * TODO: encrypt secrets (Android Keystore) in a later pass.
 */
class FileStorage(
    private val root: File,
) : Storage {
    init {
        root.mkdirs()
    }

    override fun read(path: String): ByteArray? {
        val file = resolve(path)
        if (!file.isFile) return null
        return file.readBytes()
    }

    override fun write(path: String, bytes: ByteArray) {
        val file = resolve(path)
        if (file.isDirectory) {
            file.deleteRecursively()
        }
        file.parentFile?.mkdirs()
        file.writeBytes(bytes)
    }

    override fun delete(path: String) {
        val file = resolve(path)
        if (file.exists()) file.delete()
    }

    override fun list(prefix: String): List<String> {
        val target = resolve(prefix)
        if (!target.exists()) return emptyList()
        if (target.isFile) return listOf(toStorePath(target))
        val rootPath = root.canonicalFile.toPath()
        return target.walkTopDown()
            .filter { it.isFile }
            .map { toStorePath(it, rootPath) }
            .sorted()
            .toList()
    }

    override fun getSecret(key: String): String? {
        val file = secretFile(key)
        if (!file.isFile) return null
        return file.readText()
    }

    override fun putSecret(key: String, value: String) {
        val file = secretFile(key)
        file.parentFile?.mkdirs()
        file.writeText(value)
    }

    override fun deleteSecret(key: String) {
        val file = secretFile(key)
        if (file.exists()) file.delete()
    }

    private fun secretFile(key: String): File {
        require(SECRET_KEY.matches(key)) { "invalid secret key" }
        return resolve("secrets/$key")
    }

    private fun resolve(path: String): File {
        val parts = path.split('/').filter { it.isNotEmpty() }
        require(parts.isNotEmpty()) { "path is empty" }
        require(parts.none { it == ".." || it == "." }) { "path must stay under the store root" }
        var current = root.canonicalFile
        for (part in parts) {
            current = File(current, part)
        }
        val resolved = current.canonicalFile
        val rootPath = root.canonicalFile
        require(resolved == rootPath || resolved.path.startsWith(rootPath.path + File.separator)) {
            "path escaped the store root"
        }
        return resolved
    }

    private fun toStorePath(
        file: File,
        rootPath: Path = root.canonicalFile.toPath(),
    ): String =
        rootPath.relativize(file.canonicalFile.toPath()).toString().replace(File.separatorChar, '/')
}

private val SECRET_KEY = Regex("[A-Za-z0-9_\\-]{1,64}")
