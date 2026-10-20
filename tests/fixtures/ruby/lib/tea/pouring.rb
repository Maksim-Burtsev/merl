require_relative "samovar"

module Tea
  using Polish

  class Pouring
    # def sip
    NOTES = %q(def build; end)

    def names(raw)
      text = raw.to_s.strip
      #               ^ d: none
      text.upcase + "tea".shine
      #    ^ d: none
      #                   ^ d: picker lib/tea/samovar.rb:72
      { a: 1 }.dig(:a) + File.basename(raw).chomp.size
      #        ^ d: none
      #                                     ^ d: none
    end

    def brew
      cup = Brewer.new
        .leaves(3)
        #^ d: picker lib/tea/samovar.rb:37, lib/tea/samovar.rb:53; want lib/tea/samovar.rb:37 (#1160)
        .temperature(90)
        #^ d: picker lib/tea/samovar.rb:42, lib/tea/samovar.rb:57; want lib/tea/samovar.rb:42 (#1160)
        .build
        #^ d: picker app/builder.rb:3, lib/tea/samovar.rb:47, lib/tea/samovar.rb:61; want lib/tea/samovar.rb:47 (#1160)
      cup.sip
      #   ^ d: picker lib/tea/samovar.rb:31, lib/tea/samovar.rb:65; want lib/tea/samovar.rb:31 (#1160)
    end

    def quick
      Brewer.new.leaves(1).build.sip + Kit.make.leaves(2).build.size
      #          ^ d: lib/tea/samovar.rb:37
      #                    ^ d: picker app/builder.rb:3, lib/tea/samovar.rb:47, lib/tea/samovar.rb:61; want lib/tea/samovar.rb:47 (#1160)
      #                          ^ d: picker lib/tea/samovar.rb:31, lib/tea/samovar.rb:65; want lib/tea/samovar.rb:31 (#1160)
      #                                  ^ d: lib/tea/samovar.rb:91
      #                                       ^ d: lib/tea/samovar.rb:94
    end

    def shadow
      cup = Cup.new(1)
      cup = cup.next_cup
      #     ^ d: lib/tea/pouring.rb:43
      #         ^ d: picker lib/tea/samovar.rb:24; want lib/tea/samovar.rb:24 (#1161)
      cup.sip
      # ^ d: picker lib/tea/pouring.rb:43, lib/tea/pouring.rb:44
    end

    def strength
      2
    end

    def quirk
      strength = strength.to_i
      #          ^ d: picker lib/tea/pouring.rb:51; want lib/tea/pouring.rb:56 (#1162)
      [1].each { |c; strength| strength = c }
      #                        ^ d: lib/tea/pouring.rb:58
      strength
      # ^ d: lib/tea/pouring.rb:56
    end

    def later
      strength
      # ^ d: lib/tea/pouring.rb:51
    end

    def kin
      Samovar.new(1).decant(2) + Urn.new(1).pour(2) + Samovar.new(1).spill + Samovar.new(1).volume
      #              ^ d: lib/tea/samovar.rb:85
      #                              ^ d: none; want lib/tea/samovar.rb:88 (#1161)
      #                                     ^ d: picker lib/tea/kettle.rb:40, lib/tea/samovar.rb:81, lib/tea/vessel.rb:4; want lib/tea/samovar.rb:81 (#1161)
      #                                                              ^ d: lib/tea/vessel.rb:8
      #                                                                                     ^ d: lib/tea/vessel.rb:20
    end

    def taps
      kettle = Kettle.new(1)
      Kettle.new(1).tap { |k| k.cold } && kettle&.cold
      #                         ^ d: picker lib/tea/kettle.rb:20, lib/tea/kettle.rb:24; want lib/tea/kettle.rb:24 (#1161)
      #                                             ^ d: picker lib/tea/kettle.rb:20, lib/tea/kettle.rb:24; want lib/tea/kettle.rb:24 (#1161)
      ::Tea::Cup.new(1).sip + Leaf.new("g").color.size
      #      ^ d: lib/tea/samovar.rb:23
      #                 ^ d: lib/tea/samovar.rb:31
      #                                     ^ d: none; want lib/tea/samovar.rb:89 (#1163)
    end

    def text
      label = "cup #{quick}"
      #              ^ d: lib/tea/pouring.rb:33
      label + <<~TEXT
        def quick
        #{brew}
        # ^ d: lib/tea/pouring.rb:21
      TEXT
    end
  end

  class Pot
    strength = 5
    define_method(:measure) { strength }
    #                         ^ d: lib/tea/pouring.rb:101
  end
end
