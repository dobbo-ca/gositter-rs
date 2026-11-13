er(argc, argv, keys);
    parser.about("\nThis program demonstrates the use of calcHist() -- histogram creation.\n");
    if (parser.has("help"))
    {
        parser.printMessage();
        return 0;
    }
    string inputImage = parser.get<string>(0);

    // Load the source image. HighGUI use.
    image = imread(samples::findFile(inputImage), IMREAD_GRAYSCALE);
    if(image.empty())
    {
        std::cerr << "Cannot read image file: " << inputImage << std::endl;
        return -1;
    }

    namedWindow("image", 0);
    namedWindow("histogram", 0);

    createTrackbar("brightness", "image", &_brightness, 200, updateBrightnessContrast);
    createTrackbar("contrast", "image", &_contrast, 200, updateBrightnessContrast);

    updateBrightnessContrast(0, 0);
    waitKey();

    return 0;
}
