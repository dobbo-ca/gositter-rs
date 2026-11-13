t = 0;
        }
    }

    public Rect2d clone() {
        return new Rect2d(x, y, width, height);
    }

    public Point tl() {
        return new Point(x, y);
    }

    public Point br() {
        return new Point(x + width, y + height);
    }

    public Size size() {
        return new Size(width, height);
    }

    public double area() {
        return width * height;
    }

    public boolean empty() {
        return width <= 0 || height <= 0;
    }

    public boolean contains(Point p) {
        return x <= p.x && p.x < x + width && y <= p.y && p.y < y + height;
    }

    @Override
    public int hashCode() {
        final int prime = 31;
        int result = 1;
        long temp;
        temp = Double.doubleToLongBits(height);
        result = prime * result + (int) (temp ^ (temp >>> 32));
        temp = Double.doubleToLongBits(width);
        result = prime * result + (int) (temp ^ (temp >>> 32));
        temp = Double.doubleToLongBits(x);
        result = prime * result + (int) (temp ^ (temp >>> 32));
        temp = Double.doubleToLongBits(y);
        result = prime * result + (int) (temp ^ (temp >>> 32));
        return result;
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof Rect2d)) return false;
        Rect2d it = (Rect2d) obj;
        return x == it.x && y == it.y && width == it.width && height == it.height;
    }

    @Override
    public String toString() {
        return "{" + x + ", " + y + ", " + width + "x" + height + "}";
    }
}
