# Two classes named `Base` (#793): `Shop::Cart < Base` reads `Shop::Base`, as Ruby looks the
# constant up in the lexical scopes first.
module Shop
  class Base
    def checkout = true

    def initialize
      @basket = []
    end
  end
end
