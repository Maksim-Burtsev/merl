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
        #^ d: lib/tea/samovar.rb:37
        .temperature(90)
        #^ d: lib/tea/samovar.rb:42
        .build
        #^ d: lib/tea/samovar.rb:47
      cup.sip
      #   ^ d: lib/tea/samovar.rb:31
    end

    def quick
      Brewer.new.leaves(1).build.sip + Kit.make.leaves(2).build.size
      #      ^ d: none
      #          ^ d: lib/tea/samovar.rb:37
      #                    ^ d: lib/tea/samovar.rb:47
      #                          ^ d: lib/tea/samovar.rb:31
      #                                  ^ d: lib/tea/samovar.rb:91
      #                                       ^ d: lib/tea/samovar.rb:94
    end

    def shadow
      cup = Cup.new(1)
      cup = cup.next_cup
      #     ^ d: lib/tea/pouring.rb:44
      #         ^ d: lib/tea/samovar.rb:24
      cup.sip
      # ^ d: picker lib/tea/pouring.rb:44, lib/tea/pouring.rb:45
    end

    def strength
      2
    end

    def quirk
      strength = strength.to_i
      #          ^ d: lib/tea/pouring.rb:57
      [1].each { |c; strength| strength = c }
      #                        ^ d: lib/tea/pouring.rb:59
      strength
      # ^ d: lib/tea/pouring.rb:57
    end

    def later
      strength
      # ^ d: lib/tea/pouring.rb:52
    end

    def kin
      Samovar.new(1).decant(2) + Urn.new(1).pour(2) + Samovar.new(1).spill + Samovar.new(1).volume
      #       ^ d: lib/tea/vessel.rb:22
      #              ^ d: lib/tea/samovar.rb:85
      #                          ^ d: lib/tea/samovar.rb:88
      #                              ^ d: lib/tea/vessel.rb:22
      #                                     ^ d: lib/tea/samovar.rb:81
      #                                                              ^ d: lib/tea/vessel.rb:8
      #                                                                                     ^ d: lib/tea/vessel.rb:20
    end

    def taps
      kettle = Kettle.new(1)
      Kettle.new(1).tap { |k| k.cold } && kettle&.cold
      #                         ^ d: lib/tea/kettle.rb:24
      #                                             ^ d: lib/tea/kettle.rb:24
      ::Tea::Cup.new(1).sip + Leaf.new("g").color.size
      #      ^ d: lib/tea/samovar.rb:23
      #                 ^ d: lib/tea/samovar.rb:31
      #                                     ^ d: lib/tea/samovar.rb:89
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
    #                         ^ d: lib/tea/pouring.rb:104
  end
end

module Tea
  class Guess
    include Pourable

    def maybe(cup)
      cup = Cup.new(1) if cup.nil?
      cup.sip
      #   ^ d: picker lib/tea/samovar.rb:31, lib/tea/samovar.rb:65
      pot = Brewer.new
      pot = Infuser.new
      pot.leaves(1) + names { 1 }.dig(:a)
      #   ^ d: picker lib/tea/samovar.rb:37, lib/tea/samovar.rb:53
      #                           ^ d: picker lib/tea/samovar.rb:10
    end

    define_method(:pour) { |cups| super(cups) }
    #                             ^ d: none
  end
end
