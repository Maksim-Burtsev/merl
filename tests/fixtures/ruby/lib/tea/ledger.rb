# Inside a setter `def x=(v)` or `def []=(k, v)` the cursor is in an instance method, as in any
# other: its `self.class` is the class, and its locals stay in it (#1114).
module Tea
  class Ledger
    def self.entry_for(key)
      key
    end

    def tariff
      7
    end

    def []=(key, value)
      self.class.entry_for(key)
      #          ^ d: lib/tea/ledger.rb:5
    end

    def total=(value)
      tariff = value
      self.class.entry_for(tariff)
      #          ^ d: lib/tea/ledger.rb:5
    end

    def bill
      tariff
      # ^ d: lib/tea/ledger.rb:9
    end
  end
end
