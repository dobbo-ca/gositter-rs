class A {
  void Sample() {
    try {
    } finlly {
    }

    try {
    } catch (Exception ex) {
    }

    try {
    } catch (Exception ex) {
    } finally {
    }

    try {
    } catch (Exception ex) {
    } catch (OtherException ex) {
    }

    try {
    } catch (Exception ex) when (a == 1) {
    }
  }
}
