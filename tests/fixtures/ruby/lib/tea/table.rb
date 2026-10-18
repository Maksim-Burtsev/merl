require_relative "descale"

module Tea
  class Kettle
    def brew
      steep = steep() + 1
      #       ^ d: lib/tea/vessel.rb:30
      steep * steep()
      # ^ d: lib/tea/table.rb:6
      #       ^ d: lib/tea/vessel.rb:30
    end

    def boil
      heat = 100
      heat + self.heat
      # ^ d: lib/tea/table.rb:14
      #           ^ d: lib/tea/kettle.rb:13
    end

    def warm
      self.heat = heat + 1
      #    ^ d: lib/tea/kettle.rb:13
      #           ^ d: lib/tea/kettle.rb:13
      cold || self.cold
      # ^ d: lib/tea/kettle.rb:24
      #            ^ d: lib/tea/kettle.rb:24
    end

    def self.chill
      cold || hot || fill
      # ^ d: lib/tea/kettle.rb:20
      #       ^ d: lib/tea/kettle.rb:31
      #              ^ d: lib/tea/vessel.rb:14
    end

    def serve(cups)
      [cups].map { |heat| heat + 1 }
      #                   ^ d: lib/tea/table.rb:37
      cups
      # ^ d: lib/tea/table.rb:36
    end

    def pour_all(cups)
      [1].each do |cups|
        pour(cups)
        # ^ d: lib/tea/vessel.rb:4
        #    ^ d: lib/tea/table.rb:44
      end
      whistle && descale && volume
      # ^ d: lib/tea/kettle.rb:28
      #          ^ d: lib/tea/descale.rb:5
      #                     ^ d: lib/tea/vessel.rb:20
      RECIPE
      # ^ d: lib/tea/kettle.rb:4
    end
  end

  class Table
    def set(kettle)
      Kettle.cold
      #      ^ d: lib/tea/kettle.rb:20
      Kettle.hot
      #      ^ d: lib/tea/kettle.rb:31
      Kettle.fill
      #      ^ d: lib/tea/vessel.rb:14
      Vessel.empty
      #      ^ d: lib/tea/vessel.rb:26
      Teapot.new(1).pour(2) + Kettle.new(1).pour(2)
      #             ^ d: lib/tea/kettle.rb:40
      #                                     ^ d: lib/tea/vessel.rb:4
      Teapot.new(1).spill
      #             ^ d: lib/tea/vessel.rb:8
      Kettle.new(1).descale
      #             ^ d: lib/tea/descale.rb:5
      kettle.heat = 3
      #      ^ d: picker lib/tea/kettle.rb:13
      Kettle
      # ^ d: picker lib/tea/kettle.rb:10, lib/tea/descale.rb:4, lib/tea/table.rb:4
      Teapot
      # ^ d: lib/tea/kettle.rb:37
    end

    def total(cups, price)
      price.tap { |price| price } + price
      # ^ d: lib/tea/table.rb:83
      #                   ^ d: lib/tea/table.rb:84
      #                             ^ d: lib/tea/table.rb:83
      cups.each do |cups| cups end && cups
      # ^ d: lib/tea/table.rb:83
      #                   ^ d: lib/tea/table.rb:88
      #                               ^ d: lib/tea/table.rb:83
    end
  end
end
