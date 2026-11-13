1/quux?meh")
  }
  end

  def recursive(map)
    Rack::Lint.new Rack::Recursive.new(Rack::URLMap.new(map))
  end

  it "allow for subrequests" do
    res = Rack::MockRequest.new(recursive("/app1" => @app1,
                                          "/app2" => @app2)).
      get("/app2")

    res.must_be :ok?
    res.body.must_equal "App2App1"
  end

  it "raise error on requests not below the app" do
    app = Rack::URLMap.new("/app1" => @app1,
                           "/app" => recursive("/1" => @app1,
                                               "/2" => @app2))

    lambda {
      Rack::MockRequest.new(app).get("/app/2")
    }.must_raise(ArgumentError).
      message.must_match(/can only include below/)
  end

  it "support forwarding" do
    app = recursive("/app1" => @app1,
                    "/app3" => @app3,
                    "/app4" => @app4)

    res = Rack::MockRequest.new(app).get("/app3")
    res.must_be :ok?
    res.body.must_equal "App1"

    res = Rack::MockRequest.new(app).get("/app4")
    res.must_be :ok?
    res.body.must_equal "App1"
    res["x-path-info"].must_equal "/quux"
    res["x-query-string"].must_equal "meh"
  end
end
