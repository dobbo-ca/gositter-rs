class C {
 M() {
        foreach (var id in (int[])[
            Constants.Region.US,
            Constants.Region.EU,
            Constants.Region.AP,
        ])
        {
            Invalidate(key, id);
        }
    }
}
