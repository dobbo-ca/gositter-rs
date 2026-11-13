      {
                return make_parsed_result(LOG_LEVEL_SILENT);
            }
            break;
        case 'E':
            if (len == 1u ||
                (len == 5u && cv::toUpperCase(s) == "ERROR"))
            {
                return make_parsed_result(LOG_LEVEL_ERROR);
            }
            break;
        case 'F':
            if (len == 1u ||
                (len == 5u && cv::toUpperCase(s) == "FATAL"))
            {
                return make_parsed_result(LOG_LEVEL_FATAL);
            }
            break;
        case 'I':
            if (len == 1u ||
                (len == 4u && cv::toUpperCase(s) == "INFO"))
            {
                return make_parsed_result(LOG_LEVEL_INFO);
            }
            break;
        case 'O':
            if (len == 3u && cv::toUpperCase(s) == "OFF")
            {
                return make_parsed_result(LOG_LEVEL_SILENT);
            }
            break;
      ¨ case 'S':
            if (len == 1u ||
                (len == 6u && cv::toUp