# frozen_string_literal: true

require "set"

module Shop  # f: 5-112
  class Cart < Base  # f: 6-111
    PATTERN = %r{/end/if}  # f: 6-111
    WORDS = %w[if end do
               while def]  # f: 6-111
    attr_reader :items, :end  # f: 6-111

    def initialize(items = [])  # f: 12-17
      @items = items  # f: 12-17
      @total = 0 if items.empty?  # f: 12-17
      @state = :open unless items.any?  # f: 12-17
      @mode = { if: 1, end: 2 }  # f: 12-17
    end  # f: 12-17

    def total = @items.sum(&:price)  # f: 6-111

    def add(item)  # f: 21-32
      return if item.nil?  # f: 21-32
      if item.free?  # f: 23-30
        log "free: #{item.name if item}"  # f: 21-32
      elsif item.price > 100  # f: 25-26
        warn "end"  # f: 21-32
      else  # f: 27-29
        log 'plain'  # f: 21-32
        log 'still'  # f: 21-32
      end  # f: 21-32
      @items << item  # f: 21-32
    end

    def each_item  # f: 34-41
      @items.each do |item|  # f: 35-37
        yield item  # f: 34-41
      end  # f: 34-41
      @items.each { |item|  # f: 34-41
        puts item  # f: 34-41
      }  # f: 34-41
    end

    def rescue_me  # f: 43-53
      risky  # f: 43-53
    rescue ArgumentError => e  # f: 45-47
      log e  # f: 43-53
      log e.message  # f: 43-53
    else  # f: 48-50
      log :ok  # f: 43-53
      log :done  # f: 43-53
    ensure  # f: 51-52
      cleanup  # f: 43-53
    end

    def label(kind)  # f: 55-65
      text = case kind  # f: 56-63
             when :a then "A"  # f: 55-65
             when :b  # f: 58-59
               "B"  # f: 55-65
             else  # f: 60-62
               "?"  # f: 55-65
               "!"  # f: 55-65
             end  # f: 55-65
      text  # f: 55-65
    end

    def report  # f: 67-72
      <<~TEXT  # f: 67-72
        if this were code it would end  # f: 67-72
        def nope  # f: 67-72
      TEXT
    end  # f: 67-72

    def loop_it  # f: 74-87
      while busy? do  # f: 75-77
        tick  # f: 74-87
      end  # f: 74-87
      until done?  # f: 78-80
        tick  # f: 74-87
      end  # f: 74-87
      begin  # f: 81-83
        tick  # f: 74-87
      end while busy?  # f: 74-87
      for x in [1, 2] do  # f: 84-86
        tick  # f: 74-87
      end  # f: 74-87
    end  # f: 74-87

    def lambdas  # f: 89-96
      check = ->(x) {  # f: 90-92
        x > 1  # f: 89-96
      }  # f: 89-96
      other = lambda do |x|  # f: 93-95
        x  # f: 89-96
      end  # f: 89-96
    end  # f: 89-96

    class << self  # f: 98-102
      def build  # f: 99-101
        new  # f: 99-101
      end  # f: 99-101
    end  # f: 98-102

    def matches?(s) = s =~ /#{PATTERN} if end/  # f: 6-111

    def unless_it  # f: 106-110
      unless empty?  # f: 107-109
        clear  # f: 106-110
      end  # f: 106-110
    end  # f: 106-110
  end  # f: 6-111
end  # f: 5-112

module Extra  # f: 114-144
  def wait_for(a, b)  # f: 115-122
    while a.busy? ||  # f: 116-119
          b.busy? do  # f: 115-122
      sleep 1  # f: 115-122
    end  # f: 115-122
    [a, b].reduce(:/)  # f: 115-122
    a =~ /the end/ ? self.class.name : b.end  # f: 115-122
  end  # f: 115-122

  def matcher(x)  # f: 124-131
    case x  # f: 125-130
    in [first, *]  # f: 126-127
      first  # f: 124-131
    in { name: }  # f: 128-129
      name  # f: 124-131
    end  # f: 124-131
  end  # f: 124-131

  def queries  # f: 133-143
    one = <<-SQL  # f: 133-143
      if end  # f: 133-143
    SQL
    two, three = <<~'A', <<B  # f: 133-143
      def  # f: 133-143
    A
do  # f: 133-143
B
    [one, two, three]  # f: 133-143
  end  # f: 133-143
end  # f: 114-144

=begin
def not_code
end
=end
