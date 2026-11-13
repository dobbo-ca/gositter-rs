/*
 * Copyright (C) 2020 Square, Inc.
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

import kotlinx.cinterop.get
import okio.Path.Companion.toPath
import okio.internal.toPath
import platform.posix.EEXIST
import platform.posix.errno

internal object PosixFileSystem : FileSystem() {
  internal val SELF_DIRECTORY_ENTRY = ".".toPath()
  internal val PARENT_DIRECTORY_ENTRY = "..".toPath()

  override fun canonicalize(path: Path) = variantCanonicalize(path)

  override fun metadataOrNull(path: Path) = variantMetadataOrNull(path)

  override fun list(dir: Path): List<Path> = list(dir, throwOnFa