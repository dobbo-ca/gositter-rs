 CUBLAS_STATUS_INTERNAL_ERROR: return "CUBLAS_STATUS_INTERNAL_ERROR";
                case CUBLAS_STATUS_NOT_SUPPORTED: return "CUBLAS_STATUS_NOT_SUPPORTED";
                case CUBLAS_STATUS_LICENSE_ERROR: return "CUBLAS_STATUS_LICENSE_ERROR";
                }
                return "UNKNOWN_CUBLAS_ERROR";
            };

            if (status != CUBLAS_STATUS_SUCCESS)
                throw cuBLASException(Error::GpuApiCallError, cublasGetErrorString(status), func, file, line);
        }
    }

    /** non-copyable cuBLAS smart handle
     *
     * UniqueHandle is a smart non-sharable wrapper for cuBLAS handle which ensures that the handle
     * is destroyed after use. The handle must always be associated with a non-default stream. Th