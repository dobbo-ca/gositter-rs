 and `path` APIs to Kotlin/JS, using reasonable default behaviors
 * if those symbols aren't available.
 *
 * This was originally implemented using a Kotlin/JS [JsModule], but that broke browser builds
 * because modules weren't available to browsers.
 *
 * https://nodejs.org/api/os.html
 * https://github.com/browserify/path-browserify/blob/master/index.js
 * https://github.com/CoderPuppy/os-browserify/blob/master/browser.js
 */

internal actual val PLATFORM_DIRECTORY_SEPARATOR: String
  get() = (path?.sep) as? String ?: "/"

private val os: dynamic
  get() {
    return try {
      js("require('os')")
    } catch (t: Throwable) {
      null
    }
  }

private val path: dynamic
  get() {
    return try {
      js("require('path')")
    } catch (t: Throwable) {
      null
    }
  }

internal val tmpdir: String
  get() = os?.tmpdir() as? String ?: "/tmp"
