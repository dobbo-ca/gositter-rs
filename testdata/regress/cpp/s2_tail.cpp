int main()
{
    Mat mat(480, 640, CV_8UC4); // Create a matrix with alpha channel
    paintAlphaMat(mat);

    vector<int> compression_params;
    compression_params.push_back(IMWRITE_PNG_COMPRESSION);
    compression_params.push_back(9);

    bool result = false;
    try
    {
        result = imwrite("alpha.png", mat, compression_params);
    }
    catch (const cv::Exception& ex)
    {
        fprintf(stderr, "Exception converting image to PNG format: %s\n", ex.what());
    }

    if (result)
        printf("Saved PNG file with alpha data.\n");
    else
        printf("ERROR: Can't save PNG file.\n");

    vector<Mat> imgs;
    imgs.push_back(mat);
    imgs.push_back(~mat);
    imgs.push_back(mat(Rect(0, 0, mat.cols / 2, mat.rows / 2)));
    imwrite("test.tiff", imgs);
    printf("Multiple files saved in test.tiff\n");

    return result ? 0 : 1;
}
