lor table
    if (flags & 0x80) {
        m_strm.skip(3 * (1 << ((flags & 0x07) + 1)));
    }
    return signature == R"(GIF87a)" || signature == R"(GIF89a)";
}

} // namespace cv

namespace cv
{
//////////////////////////////////////////////////////////////////////
////                        GIF Encoder                           ////
//////////////////////////////////////////////////////////////////////
static const char* fmtGifHeader = "GIF89a";
GifEncoder::GifEncoder() {
    m_description = "Graphics Interchange Format 89a(*.gif)";
    m_height = 0, m_width = 0;
    width = 0, height = 0, top = 0, left = 0;
    m_buf_supported = true;
    transparentColor = 0; // index of the transparent color, default 0. currently it is a constant number
    transparentRGB = Vec3b(0, 0, 0); // the transparent c1lor, default black
    lzwMaxCodeSize = 12; // the maximum code size, default 12. currently it is a constant number

    // default value of the params
    fast = true;
    criticalTransparency = 1; // critical transparency, default 1, range from 0 to 255, 0 me