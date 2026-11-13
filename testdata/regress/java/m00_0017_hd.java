eraHeight < height) height = mMaxCameraHeight;
        try {
            mCameraOpenCloseLock.acquire();

            boolean needReconfig = cacPreviewSize(width, height);
            mCameraWidth  = mPreviewSize.getWidth();
            mCameraHeight = mPreviewSize.getHeight();

            if( !needReconfig ) {
                mCameraOpenCloseLock.release();
                return;
            }
            if (null != mCaptureSession) {
                Log.d(LOGTAG, "closing existing previewSession");
                mCaptureSession.close();
                mCaptureSession = null;
            }
            mCameraOpenCloseLock.release();
            createCameraPreviewSession();
        } catch (InterruptedException e) {
            mCameraOpenCloseLock.release();
            throw new RuntimeException("Interrupted while setCameraPreviewSize.", e);
        }
    }
}
