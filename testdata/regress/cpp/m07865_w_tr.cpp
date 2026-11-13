

CV_ImageWarpBaseTest::~CV_ImageWarpBaseTest()
{
}

String CV_ImageWarpBaseTest::interpolation_to_string(int inter) const
{
    bool inverse = (inter & WARP_INVERSE_MAP) != 0;
    inter &= ~WARP_INVERSE_MAP;
    String str;

    if (inter == INTER_NEAREST)
    