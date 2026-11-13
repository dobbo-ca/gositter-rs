et<CV_8U, CV_32F> > h(_src, _dst, dcn);

    if(!h.createKernel("Lab2BGR", ocl::imgproc::color_lab_oclsrc,
                       format("-D DCN=%d -D BIDX=%d%s", dcn, bidx, srgb ? " -D SRGB" : "")))
    {
        return false;
    }

    // Prepare additional arguments

    initLabTabs();

    static UMat ucoeffs, usRGBInvGammaTab;

    if (srgb && usRGBInvGammaTab.empty())
        Mat(1, GAMMA_TAB_SIZE*4, CV_32FC1, const_cast<float*>(sRGBInvGammaTab)).copyTo(usRGBInvGammaTab);

    float coeffs[9];
    softdouble whitePt[3];
    for(int i = 0; i < 3; i++)
        whitePt[i] = D65[i];

    for( int i = 0; i < 3; i++ )
    {
        coeffs[i+(bidx^2)*3] = (float)(XYZ2sRGB_D65[i  ]*whitePt[i]);
        coeffs[i+3]          = (float)(XYZ2sRGB_D65[i+3]*whitePt[i]);
        coeffs[i+bidx*3]     = (float)(XYZ2sRGB_D65[i+6]*whitePt[i]);
    }

    Mat(1, 9, CV_32FC1, coeffs).copyTo(ucoeffs);

    float lThresh = softfloat(8); // 0.008856f * 903.3f  = (6/29)^3*(29/3)^3 = 8
    float fThresh = softfloat(6)/softfloat(29); // 7.787f * 0.008856f + 16.0f / 116.0f = 6/29

    ocl::KernelArg coeffsarg = ocl::KernelArg::PtrReadOnly(ucoeffs);

    // Set additional arguments and run

    if(srgb)
        h.setArg(ocl::KernelArg::PtrReadOnly(usRGBInvGammaTab));

    h.setArg(coeffsarg);
    h.setArg(lThresh);
    h.setArg(fThresh);

    return h.run();
}


bool oclCv