// This file is part of OpenCV project.
// It is subject to the license terms in the LICENSE file found in the top-level directory
// of this distribution and at http://opencv.org/license.html.
#include "perf_precomp.hpp"

namespace opencv_test {

CV_ENUM(ThreshType, THRESH_BINARY, THRESH_BINARY_INV, THRESH_TRUNC, THRESH_TOZERO, THRESH_TOZERO_INV)

typedef tuple<Size, MatType, ThreshType> Size_MatType_ThreshType_t;
typedef perf::TestBaseWithParam<Size_MatType_ThreshType_t> Size_MatType_ThreshType;

PERF_TEST_P(Size_MatType_ThreshType, threshold,
            testing::Combine(
                testing::Values(TYPICAL_MAT_SIZES),
                testing::Values(CV_8UC1, CV_16SC1, CV_32FC1, CV_64FC1),
                ThreshType::all()
                )
            )
{

    Size sz = get<0>(GetParam());
    int type = get<1>(GetParam());
    ThreshType threshType = get<2>(GetParam());

    Mat src(sz, type);
    Mat dst(sz, type);

    double thresh = theRNG().uniform(1, 254);
    double maxval = theRNG().uniform(1, 254);

    declare.in(src, WARMUP_RNG).out(dst);

    int runs = (sz.width <= 640) ? 40 : 1;
    TEST_CYCLE_MULTIRUN(r