# frozen_string_literal: true

require_relative '../../constants'

module Rack
  module Auth
    # Rack::Auth::AbstractHandler implements common authentication functionality.
    #
    # +realm+ should be set for all handlers.

    class AbstractHandler

      attr_accessor :realm

      def initialize(app, realm = nil, &authenticator)
        @app, @realm, @authenticator = app, realm, authenticator
      end


      pri