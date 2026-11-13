;

    const char * const interpolationMap[3] = { "NEAREST", "LINEAR", "CUBIC" };
    ocl::ProgramSource program = op_type == OCL_OP_AFFINE ?
                ocl::imgproc::warp_affine_oclsrc : ocl::imgproc::warp_perspective_oclsrc;
    const char * const kernelName = op_type == OCL_OP_AFFINE ? "warpAffine" : "warpPerspective";

    int scalarcn = cn == 3 ? 4 : cn;
    bool is32f = interpolation == INTER_CUBIC || interpolation == INTER_LINEAR;
    int wdepth = interpolation == INTER_NEAREST ? depth : std::max(is32f ? CV_32F : CV_32S, depth);
    int sctype = CV_MAKETYPE(wdepth, scalarcn);

    ocl::Kernel k;
    String opts;
    if (interpolation == INTER_NEAREST)
    {
        opts = format("-D INTER_NEAREST -D T=%s%s -D CT=%s -D T1=%s -D ST=%s -D CN=%d -D ROWS_PER_WI=%d",
                      ocl::typeToStr(type),
                      doubleSupport ? " -D DOUBLE_SUPPORT" : "",
                      useDouble ? "double" : "float",
                      ocl::typeToStr(CV_MAT_DEPTH(type)),
                      ocl::typeToStr(sctype), cn, rowsPerWI);
    }
    else {
        char cvt[2][50];
        opts = format("-D INTER_%s -D T=%s -D T1=%s -D ST=%s -D WT=%s -D SRC_DEPTH=%d"
                      " -D CONVERT_TO_WT=%s -D CONVERT_TO_T=%s%s -D CT=%s -D CN=%d -D ROWS_PER_WI=%d",
                      interpolationMap[interpolation], ocl::typeToStr(type),
         