rams[i + 1], 0);
            if(value < 0) {
                CV_LOG_WARNING(nullptr, cv::format("The value(%d) for IMWRITE_EXR_DWA_COMPRESSION_LEVEL must be 0 or more. It is fallbacked to 0", value));
            }
#endif
        }
    }

    if( channels == 3 || channels == 4 )
    {
        header.channels().insert "R", Channel( type ) );
        header.channels().insert( "G", Channel( type ) );
        header.channels(