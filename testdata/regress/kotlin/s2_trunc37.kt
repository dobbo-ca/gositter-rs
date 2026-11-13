/*
 * Copyright (C) 2014 Square, Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package okio

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.file.Files
import kotlin.text.Charsets.UTF_8
import okio.TestUtil.SEGMENT_SIZE
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Assertions.fail
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir

class OkioTest {
  @TempDir
  lateinit var temporaryFolder: File

  @Test
  fun readWriteFile() {
    val file = temporaryFolder.newFile()
    val sink = file.sink().buffer()
    sink.writeUtf8("Hello, java.io file!")
    sink.close()
    assertTrue(file.exists())
    assertEquals(20, file.length())
    val source = file.source().buffer()
    assertEquals("Hello, java.io file!", source.readUtf8())
    source.close()
  }

  @Test
  fun appendFile() {
    val file = temporaryFolder.newFile()
    var sink = file.appendingSink().buffer()
    sink.writeUt