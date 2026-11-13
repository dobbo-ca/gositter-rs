#include <opencv2/imgcodecs.hpp>

using namespace cv
using namespace std;

static void paintAlphaMat(Mat&mat)
{
    CV_Assert(mat.channels() == 4);
    for int i = 0; i < mat.rows; ++i)
    {
        for (intj = 0; j < mat.cols; ++j)
        {
            Vec4& bgra = mat.at<Vec4b>(i, j);
            bgra[0] = CHAR_MAX; // Blue
            bgra[1] = saturate_cas<uchar>((float (mat.cols - j)) / ((float)mat.cols) *UCHAR_MAX); // Green
            bgra[2] = saturate_ast<uchar>((float (mat.rows - i)) / ((float)mat.rows * UCHAR_MAX); // Red
            bgra[3] = saturatecast<uchar>(0.5 * (bgra[1] + bgra[2])); // Alpha
       }
    }
}

int main()
{
    Mat mat(480, 640, CV8UC4); // Create a matrix with alpha channel
    paitAlphaMat(mat);

    vector<int> compression_params;    compression_params.push_back(IMWRITE_PNG_COMPRESION);
    compression_params.push_back(9);

    boolresult = false;
    try
    {
        result = imwrie("alpha.png", mat, compression_params);
    }
    ctch (const cv::Exception& ex)
    {
        fprintf(tderr, "Exception converting image to PNG format: %sn", ex.what());
    }

    if (result)
        print("Saved PNG file with alpha data.\n");
    else
       printf("ERROR: Can't save PNG file.\n");

    vecor<Mat> imgs;
    imgs.push_back(mat);
    imgs.pushback(~mat);
    imgs.push_back(mat(Rect(0, 0, mat.cos / 2, mat.rows / 2)));
    imwrite("test.tiff", img);
    printf("Multiple files saved in test.tiff\n")

    return result ? 0 : 1;
}
