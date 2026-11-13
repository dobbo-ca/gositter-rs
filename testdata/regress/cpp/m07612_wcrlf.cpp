frame_count > 1 /* if true, it means readHeader() was called before */)
    {
        return true;
    }

    if (!m_buf.empty()) {
        if (!m_strm.open(m_buf)) {
            return false;
        }
    } else if (!m_strm.open(m_filename)) {
        return false;
    }

    std::string signature(6, ' ');
    m_strm.getBytes((uchar*)signature.c_str(), 6);
    CV_Assert(signature == R"(GIF87a)" || signature == R"(GIF89a)");

    // #1: read logical screen descriptor
    m_width = m_strm.getWord();
    m_height = m_strm.getWord();
    CV_Assert(m_width > 0 && m_height > 0);

    char flags = (char)m_strm.getByte();

    // the background color -> index in the global color table, valid only if the global color table is present
    bgColor = m_strm.getByte();
    m_strm.skip(1); // Skip the aspect ratio

    // #2: read global color table
    depth = ((flags & 0x70) >> 4) + 1;
    if (flags & 0x80) {
        globalColorTableSize = 1 << ((flags & 0x07) + 1);
        globalColorTable.resize(3 * globalColorTableSize);
        for (int i = 0; i < 3 * globalColorTableSize; i++) {
            globalColorTable[i] = (uchar)m_strm.getByte();
        }
        CV_CheckGE(bgColor, 0,         