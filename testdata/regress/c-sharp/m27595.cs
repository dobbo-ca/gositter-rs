[RegularExpression(pattern: "/.+", ErrorMessage = "The Callback Path Must start with a forward slash '/' followed by one or more characters")]
class Validator { }

[Route(Name: "default", Template = "/api/{id}")]
[Obsolete(message: "Use NewMethod instead", e\ror: true)]
class Example { }
