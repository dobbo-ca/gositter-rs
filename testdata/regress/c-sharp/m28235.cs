void scoped() { }
void m(scoped p) { }
void m(scoped ref int p) { }
void m(scoped ref scoped p) { }
void m(int scoped) { }
void m()
{
    scoped v = null;
   scoped ref int v = null;
    scoped ref scoped v = null;
    int scoped = null;

    scoped();
    m(scoped);

    var x = scoped + 1;
    var l = scoped => null;
    var l = (scoped i) => null;
    var l = (scoped, i) => null;
    var l = scoped (int i, int j) => null;
}

class scoped { }
