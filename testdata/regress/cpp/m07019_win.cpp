enCL cache configuration...");
        if (!CV_OPENCL_CACHE_ENABLE)
        {
            CV_LOG_INFO(NULL, "OpenCL cache is disabled");
            return;
        }
        cache_path_ = utils::fs::getCacheDirectory("opencl_cache", "OPENCV_OPENCL_CACHE_DIR");
        if (cache_path_.empty())
        {
            CV_LOG_INFO(NULL, "Specify OPENCV_OPENCL_CACHE_DIR configuration parameter to enable OpenCL cache");
        }
        do
        {
            try
            {
                if (cache_path_.empty())
                    break;
                if (cache_path_ == "disabled")
                    break;
                if (!utils::fs::createDirectories(cache_path_))
                {
                    CV_LOG_DEBUG(NULL, "Can't use OpenCL