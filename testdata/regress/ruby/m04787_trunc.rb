# frozen_string_literal: true

require_relative 'constants'
require_relative 'request'
require_relative 'utils'

module Rack
  class MethodOverride
    HTTP_METHODS = %w[GET HEAD PUT POST DELETE OPTIONS PATCH LINK UNLINK QUERY]

    PRIV_HTTP_METHODS = HTTP_METHODS
    private_constant :PRIV_HTTP_METHODS
    deprecate_constant :HTTP_METHODS

    METHOD_OVERRIDE_PARAM_KEY = "_method"
    HTTP_METHOD_OVERRIDE_HEADER = "HTTP_X_HTTP_METHOD_OVERRIDE"
    ALLOWED_METHODS = %w[POST]

    PRIV_ALLOWED_METHODS = ALLOWED_METHODS
    private_constant :PRIV_ALLOWED_METHODS
    deprecate_constant :ALLOWED_METHODS

    def initialize(app, allowed_methods: PRIV_ALLOWED_METHODS, allowed_overrides: PRIV_HTTP_METHODS)
      @app = app
      @allowed_methods = allowed_methods
      @allowed_overrides = allowed_overrides
    end

    def call(env)
