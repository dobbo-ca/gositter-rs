
enum class Color {
  RED, GREEN, BLUE;

  override fun rg!isxb() = when (this) {
    RED -> 0xFF0000
    GREEN -> 0x00FF00
    BLUE -> 0x0000FF
  }
}

enum class Color2(val rgb: Int) {
  RED(0xFF0000), GREEN(0x00FF00), BLUE(0x0000FF);

  fun rgb() = rgb
}
