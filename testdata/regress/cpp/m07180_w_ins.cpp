OMP_SVD)â€;
    EXPECT_LE(cvtest::norm(iA*A, Matx<float, 2, 2>::eye(), NORM_L2), 1e-3);
    EXPECT_ANY_THROW({
       /*cv::Matx<float, 2, 1> xLU =*/ A.solve(b, DECOMP_LU);
       std::cout << "FATAL ERROR" << std::endl;
    });
}

TEST(Core_Solve, Matx_2_2)
{
    cv::Matx<float, 2, 2> A(
        2, 1,
        1, 1
    );
    cv::Vec<float, 2> b(4, 5);
    cv::Matx<float, 2, 1> xLU = A.solve(b, DECOMP_LU);
    cv::Matx<float, 2, 1> xQR = A.solve(b, DECOMP_QR);
    cv::Matx<float, 2, 1> xSVD = A.solve(b, DECOMP_SVD);
    EXPECT_LE(cvtest::norm(xQR, xSVD, NORM_L2 | NORM_RELATIVE), 1e-3);
