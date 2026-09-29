module Shop
  module Warehouse
    LIMIT = 30

    class Courier
      attr_reader :name

      def initialize(name)
        @name = name
      end

      def first
        self
      end

      def dispatch(parcel)
        route = parcel.to_s
        route
      end
    end

    def self.weigh(grams)
      grams / 1000
    end

    class << self
      def open
        true
      end
    end

    def open
      false
    end
  end
end
