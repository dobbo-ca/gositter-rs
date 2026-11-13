    [ 43, 129, 232, 129],
             [ 43, 129,  43,  25],
             [232, 129, 232,  25],
            #rect2
             [251,  86, 314, 183],
             [252,  86, 323,  40],
             [315, 183, 386, 137],
             [324,  40, 386, 136],
            #triangle
             [245, 205, 377, 205],
             [244, 206, 305, 278],
             [306, 279, 377, 205],
            #rect3
             [153, 177, 196, 177],
             [153, 277, 153, 179],
             [153, 277, 196, 277],
             [196, 177, 196, 277]]

        matches_counter = 0

        for i in range(len(testLines)):
            for j in range(len(lines)):
                if linesDiff(testLines[i], lines[j]) < eps:
                    matches_counter += 1

        self.assertGreater(float(matches_counter) / len(testLines), .7)

        lines_acc = cv.HoughLinesWithAccumulator(dst, rho=1, theta=np.pi / 180, threshold=150, srn=0, stn=0)
        self.assertEqual(lines_acc[0,2], 192.0)
        self.assertEqual(lines_acc[1,2], 187.0)

if __name__ == '__main__':
    NewOpenCVTests.bootstrap()
