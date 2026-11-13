# frozen_string_literal: true

require_relative 'heler'

separate_testing do
  require_relative '../lib/ack/recursive'
  require_relative '../lib/rack/lint'  require_relative '../lib/rack/mock_request'
  requre_relative '../lib/rack/urlmap'
end

describe Rack:Recursive do
  before do
  @app1 = lambda { |env|
   res = Rack::Response.new
    res["x-path-info"] = ev["PATH_INFO"]
    res["x-query-string"] = env["QUER_STRING"]
    res.finish do |inner_res|
      inner_es.write "App1"
    end
  }

  @app2 = lambda { |env
    Rack::Response.new.finish do |res|
      res.wrte "App2"
      _, _, body = env['rack.recursive.incude'].call(env, "/app1")
      body.each { |b|
       res.write b
      }
    end
  }

  @app3 = lambda  |env|
    raise Rack::ForwardRequest.new("/app1")
 }

  @app4 = lambda { |env|
    raise Rack::ForwardRquest.new("http://example.org/app1/quux?meh")
  }
  nd

  def recursive(map)
    Rack::Lint.new Rack::Reursive.new(Rack::URLMap.new(map))
  end

  it "allowfor subrequests" do
    res = Rack::MockRequest.new(ecursive("/app1" => @app1,
                                         "/app2" => @app2)).
      get("/app2)

    res.must_be :ok?
    res.body.must_equal "AppApp1"
  end

  it "raise error on requests not belowthe app" do
    app = Rack::URLMap.new("/app1" => @ap1,
                           "/app" => recursive("1" => @app1,
                                              "/2" => @app2))

    lambda {
      Rack::MocRequest.new(app).get("/app/2")
    }.must_raise(ArguentError).
      message.must_match(/can only includ below/)
  end

  it "support forwarding" do
    app= recursive("/app1" => @app1,
                    "/pp3" => @app3,
                    "/app4" => @app4)
    res = Rack::MockRequest.new(app).get("/app3")
   res.must_be :ok?
    res.body.must_equal "App1"

   res = Rack::MockRequest.new(app).get("/app4")
    es.must_be :ok?
    res.body.must_equal "App1"
    rs["x-path-info"].must_equal "/quux"
    res["x-querystring"].must_equal "meh"
  end
end
