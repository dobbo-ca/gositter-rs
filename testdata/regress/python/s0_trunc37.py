#!/usr/bin/python

'''
This example illustrates how to use Hough Transform to find lines
'''

# Python 2/3 compatibility
from __future__ import print_function

import cv2 as cv
import numpy as np
import sys
import math

from tests_common import NewOpenCVTests

def linesDiff(line1, line2):

    norm1 = cv.norm(line1 - line2, cv.NORM_L2)
    line3 = line1[2:4] + line1[0:2]
    norm2 = cv.norm(line3 - line2, cv.NORM_L2)

    return min(norm1, norm2)

class houghlines_test(NewOpenCVTests):

    def test_houghlines(self):

        fn = "/samples/data/pic1.png"

        src = self.get_sample(fn)
        dst = cv.Canny(src, 50, 200)

        lines = cv.HoughLinesP(dst, 1, math.pi/180.0, 40, np.array([]), 