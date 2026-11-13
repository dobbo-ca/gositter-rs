a stream
		list.stream().forEach(System.out::println);

		// Using a lambda
		Runnable r = () -> System.out.println("This is a lambda function");
		r.run();

		// Using an inner class
		InnerClass innerClass = new InnerClass();
		innerClass.display();
	}

	// Inner class
	static class InnerClass {
		void display() {
			System.out.println("This is an inner class");
		}
	}
}
