init; }
}

record struct F {
  int Age { get; init; }
}

record class F {
  int Age { get; init; }
}

public record F<T> where T:struct {}

public record F<T> where T: new() {}

public record A : ISomething { }

[Test]
private record F<T1,T2> where T1 : I1, I2, new() where T2 : I2 { }

record Person(string FirstName, string LastName);

record Teacher(string FirstName, string LastName, string Subject) : Person(FirstName, LastName);

record Teacher(string FirstName, string LastName, string Subject) : Person(FirstName, LastName), Ns.I1, I2;

record Teacher() : Entity<Person>(), I1;

public record Person { };

public record struct Person2 { };

record NoBody;
