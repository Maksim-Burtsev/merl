# `Const.new.m`, no parentheses on `new`, reaches the class's own method as `Const.new(1).m` does (#1096).
module Tea
  class Service
    def serve
      Teapot.new.spill
      #          ^ d: lib/tea/vessel.rb:8
      Kettle.new.descale
      #          ^ d: lib/tea/descale.rb:5
      Teapot.new.pour(2)
      #          ^ d: lib/tea/kettle.rb:40
    end
  end
end
