n OPENCV_DOWNLOAD_CACHE_DIR environment variable"<<std::endl;
        return findFile(filename);
    }
    else{
        std::string modelPath = utils::fs::join(getenv("OPENCV_DOWNLOAD_CACHE_DIR"), utils::fs::join(sha1, filename));
        if (utils::fs::exists(modelPath))
            return modelPath;
        modelPath = utils::fs::join(getenv("OPENCV_DOWNLOAD_CACHE_DIR"),file