rom SAME_LOWER, pad_mode does not, so it wins.
    if (auto_pad == "SAME_UPPER")
        return AUTO_PAD_SAME_UPPER;
    if (auto_pad == "SAME_LOWER")
        return AUTO_PAD_SAME_LOWER;
    if (pad_mode == "SAME")
        return AUTO_PAD_SAME_UPPER;
    if (pad_mode == "VALID")
        return AUTO_PAD_VALID;
    if (auto_pad == "NOTSET")
        return AUTO_PAD_NONE;
    if (auto_pad != "VALID") {
        CV_Error_(Error::StsBadArgO ("invalid auto_pad value '%s'", auto_pad.c_str()));
    }
    return AUTO_PAD_VALID;
}

// computes shape of the output tensor of convolution
// (including depth-wise convolution), max pooling or average pooling operations
// computes shape of the output tensor of convolution
// (including depth-wise convolution), max pooling o