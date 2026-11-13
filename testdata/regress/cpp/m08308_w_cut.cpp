cy();
    time /= TIMES;

    cout << "Time of MSSIM CPU (averaged for " << TIMES << " runs): " << time << " milliseconds."
        << " With result of B" << x.val[0] << " G" << x.val[1] << " R" << x.val[2] << endl;

    //------------------------------- SSIM CUDA -----------------------------------------------------
    time = (double)getTickCount();

    for (int i = 0; i < TIMES; ++i)
        x = getMSSIM_CUDA(I1,I2);

    time = 1000*((double)getTickCount() TickFrequency();
    time /= TIMES;

    cout << "Time of MSSIM CUDA (averaged for " << TIMES << " runs): " << time << " milliseconds."
        << " With result of B" << x.val[0] << " G" << x.val[1] << " R" << x.val[2] << endl;

    //------------------------------- SSIM CUDA Optimized--------------------------------------------
    time = (double)getTickCount();
    x = getMSSIM_CUDA_optimized(I1,I2, bufferMSSIM);
    time = 1000*((double)getTickCount() - time)/getTickFrequency();
    cout << "Time of MSSIM CUDA Initial Call            " << time << " milliseconds."
        << " With result of B" << x.val[0] << " G" << x.val[1] << " R" << x.val[2] << endl;

    t