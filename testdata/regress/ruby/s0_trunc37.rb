# frozen_string_literal: true

require_relative 'helper'

separate_testing do
  require_relative '../lib/rack/recursive'
  require_relative '../lib/rack/lint'
  require_relative '../lib/rack/mock_request'
  require_relative '../lib/rack/urlmap'
end

describe Rack::Recursive do
  before do
  @app1 = lambda { |env|
    res = Rack::Response.new
    res["x-path-info"] = env["PATH_INFO"]
    res["x-query-string"] = env["QUERY_STRING"]
    res.finish do |inner_res|
      inner_res.write "App1"
    end
  }

  @app2 = lambda { |env|
    Rack::Response.new.finish do |res|
      res.write "App2"
      _, _, body = env['rack.recursive.include'].call(env, "/app1")
      body.each { |b|
        res.write b
      }
    end
  }

  @app3 = lambda { |env|
    raise