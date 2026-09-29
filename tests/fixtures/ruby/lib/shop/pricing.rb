=begin
A block comment that reads like code declares nothing:
class Tariff
def weigh(grams)
=end

module Shop
  RATE_CAP = 100

  class Tariff
    attr_reader :base

    def initialize(base)
      @base = base
    end

    def rate
      1
    end

    def describe
      "tariff"
    end
  end

  class Coupon
    attr_accessor :code
    attr_writer :owner

    def rate
      2
    end

    def describe
      "coupon"
    end

    def self.blank
      new
    end

    def expired?
      false
    end

    def expired!
      true
    end

    alias_method :label, :describe
    alias caption describe
  end

  def self.discount(total)
    [total, RATE_CAP].min - 1
  end

  BANNER = <<~TEXT
    def settle(total)
    class Basket
  TEXT

  NOTE = %q{
    def settle(total)
  }

  def self.settle(total)
    total
  end
end
