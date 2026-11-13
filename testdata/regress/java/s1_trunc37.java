package org.opencv.core;

//javadoc:Rect2d_
public class Rect2d {

    public double x, y, width, height;

    public Rect2d(double x, double y, double width, double height) {
        this.x = x;
        this.y = y;
        this.width = width;
        this.height = height;
    }

    public Rect2d() {
        this(0, 0, 0, 0);
    }

    public Rect2d(Point p1, Point p2) {
        x = (double) (p1.x < p2.x ? p1.x : p2.x);
        y = (double) (p1.y < p2.y ? p1.y : p2.y);
        width = (double) (p1.x > p2.x ? p1.x : p2.x) - x;
        height = (double) (p1.y > p2.y ? p1.y : p2.y) - y;
    }

    public Rect2d(Point p, Size s) {
        this((double) p.x, (double) p.y, (double) s.width, (double) s.height);
    }

    public Rect2d(double[] vals) {
        set(vals);
    }

    public void set(double[] vals) {
        if (vals != null) {
            x = vals.length > 0 ? (double) vals[0] : 0;
            y = vals.length > 1 ? (double) vals[1] : 0;
            width = vals.length > 2 ? (double) vals[