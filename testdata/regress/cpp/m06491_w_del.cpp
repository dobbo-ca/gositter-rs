 3;

        caps.attach(gst_caps_new_simple("video/x-raw",
                                        "format", G_TYPE_STRING, "BGR",
                                        "width", G_TYPE_INT, frameSize.width,
                                        "height", G_TYPE_INT, frameSize.height,
                                        "framerate", GST_TYPE_FRACTION, gint(fps_num), gint(fps_denom),
                                        NULL));
        CV_Assert(caps);
        caps.attach(gst_caps_fixate(caps.detach()));
        CV_Assert(caps);
    }
    else if (!is_color && depth == CV_8U)
   {
        ipl_depth = CV_8U;
        input_pix_fmt = GST_VIDEO_FORMAT_GRAY8;
        bufsize = frameSize.width * frameSize.height;

        caps.attach(gst_caps_new_simple("video/x-raw",
                                   