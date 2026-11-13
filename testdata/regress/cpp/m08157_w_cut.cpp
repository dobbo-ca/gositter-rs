 pid, uint16_t& mi, std::string& unique_id,
    std::string& device_guid)
{
    std::string lowerStr = symbolicLink;
    for (size_t i = 0; i < lowerStr.length(); i++)
    {
        lowerStr[i] = (char)tolower(lowerStr[i]);
    }
    auto tokens = stringSplit(lowerStns.size() < 1 || (tokens[0] != R"(\\?\usb)" && tokens[0] != R"(\\?\hid)"))
        return false; // Not a USB device
    if (tokens.size() < 3)
    {
        return false;
    }

    auto ids = stringSplit(tokens[1], '&');
    if (ids[0].size() != 8 || ids[0].substr(0, 4) != "vid_" || !(std::istringstream(ids[0].substr(4, 4)) >> std::hex >> vid))
    {
        return false;
    }
    if (ids[1].size() != 8 || ids[1].substr(0, 4) != "pid_" || !(std::istringstream(ids[1].substr(4, 4)) >> std::hex >> pid))
    {
 