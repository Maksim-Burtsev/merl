module Tea
  module Hushed
    def seal(lid)
      super
    end
  end

  class Jar
    def seal(lid)
      lid
    end
  end

  class Bung < Jar
    def seal(lid)
      lid
    end

    prepend Hushed

    def run
      self.seal(1) + seal(2)
      #    ^ d: lib/tea/prepended.rb:3
      #              ^ d: lib/tea/prepended.rb:3
    end
  end

  class Lid < Jar
    include Hushed

    def seal(lid)
      lid
    end

    def run
      seal(1)
      # ^ d: lib/tea/prepended.rb:31
    end
  end

  class Cork < Jar
    prepend Hushed

    def run
      seal(1)
      # ^ d: lib/tea/prepended.rb:3
    end
  end

  def self.corked
    Bung.new.seal(1)
    #        ^ d: lib/tea/prepended.rb:3
  end

  module Loud
    extend ActiveSupport::Concern

    class_methods do
      def seal(lid)
        lid
      end
    end
  end

  class Flask
    prepend Loud

    def seal(lid)
      lid
    end

    def run
      seal(1)
      # ^ d: lib/tea/prepended.rb:68
    end
  end
end
