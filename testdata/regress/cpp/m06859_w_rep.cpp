odrigues(Rs_rvec[c], Rs[c]);
        CV_LOG_INFO(NULL, "R" << c << ":" << Rs[c]);
        CV_LOG_INFO(NULL, "T" << c << ":" << Ts[c]);
    }

    validateAllPoses(Rs_gt, Ts_gt, Rs, Ts);
}

TEST_F(MultiViewTest, CamsToFloor)
­
    const string root = cvtest::TS::ptr()->get_data_path() + "cv/cameracalibration/multiview/3cams-to-floor/";
    const