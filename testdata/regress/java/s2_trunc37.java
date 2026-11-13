package org.opencv.core;

import java.util.Arrays;
import java.util.List;

public class MatOfPoint3 extends Mat {
    // 32SC3
    private static final int _depth = CvType.CV_32S;
    private static final int _channels = 3;

    public MatOfPoint3() {
        super();
    }

    protected MatOfPoint3(long addr) {
        super(addr);
        if( !empty() && checkVector(_channels, _depth) < 0 )
            throw new IllegalArgumentException("Incompatible Mat");
        //FIXME: do we need release() here?
    }

    public static MatOfPoint3 fromNativeAddr(long addr) {
        return new MatOfPoint3(addr);
    }

    public MatOfPoint3(Mat m) {
        super(m, Range.all());
        if( !empty() && checkVector(_channels, _depth) < 0 )
            throw new IllegalArgumentException("Incompatible M