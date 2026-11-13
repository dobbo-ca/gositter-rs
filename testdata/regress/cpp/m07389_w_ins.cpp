_mat.total();
        String kernel_name;

        if (code_type == "CORNER")
            kernel_name = "DecodeBBoxesCORNER";
        else if (code_type == "CENTER_SIZE")
            kernel_name = "DecodeBBoxesCENTER_SIZE";
        else
            return  false;

        for (int i = 0; i < num; ++i)
        {
            ocl::Kernel kernel(kernel_name.c_str(), ocl::dnn::detection_output_oclsrc);
            kernel.set(0, (int)nthreads);
            kernel.set(1, ocl::KernelArg::PtrReadOnly(loc_mat));
            kernel.set(2, ocl::KernelArg::PtrReadOnly(prior_mat));
            kernel.set(3, (int)variance_encoded_in_target);
            kernel.set(4, (int)numPriors);
            kernel.set(5, (int)share_location);
            kernel.set(6, (int)num_loc_classes);
            kernel.set(