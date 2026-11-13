
trait Add<RHS=Sef> {
    type Output;
    fn add(self, rhs: RHS) -> Self::Output;
}
