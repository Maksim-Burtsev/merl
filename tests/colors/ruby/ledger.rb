# frozen_string_literal: true

require "json"
require_relative "entry"

# A ledger keeps entries in order.
module Books
  LIMIT = 1_000
  PATTERN = /[a-z]+\d/i

  Entry = Struct.new(:id, :amount, :note, keyword_init: true)

  class Error < StandardError; end

  class Ledger
    include Enumerable
    extend Forwardable
    attr_reader :owner, :entries
    attr_accessor :handler

    def_delegators :@entries, :size, :empty?

    @@count = 0

    def self.empty
      new("nobody")
    end

    def initialize(owner, opened: Time.now)
      raise ArgumentError, "owner is blank" if owner.to_s.strip.empty?

      @owner = owner
      @opened = opened
      @entries = []
      @@count += 1
    end

    def each(&block)
      @entries.each(&block)
    end

    def total
      @entries.sum(&:amount)
    end

    def add(amount, note = nil)
      entry = Entry.new(id: @entries.size, amount: amount, note: note)
      @entries << entry
      handler&.call(:moved, entry)
      entry
    end

    def sync(*ids, **opts)
      ids.each do |id|
        next unless (0..LIMIT).cover?(id)

        puts "synced #{id} of #{@entries.size}"
      end
      opts.fetch(:done, true)
    end

    def kind(entry)
      case entry.amount
      when ...0 then :debit
      when 0 then raise Error, "zero"
      else :credit
      end
    end

    def report
      lines = @entries.map { |e| format("%-10s %8.2f", e.note || "-", e.amount.abs) }
      raw = <<~TEXT
        total: #{total}
        count: #{@entries.size}
      TEXT
      lines.join("\n") + raw
    end

    def check(x)
      ch = ?a
      big = 0xFF_FF
      ratio = 1.5e3
      words = %w[a b c]
      syms = %i[x y]
      x.is_a?(String) && !x.empty? || ch == "\n" || big.positive? && ratio != 0 && words.any? && syms.any?
    end

    def to_s = "Ledger(owner=#{owner}, size=#{size})"

    def to_json(*args)
      { owner: @owner, entries: @entries.map(&:to_h), "count" => @@count }.to_json(*args)
    end

    private

    def stale?
      !@opened.nil? and @opened < Time.now - 3600
    end
  end
end

if __FILE__ == $PROGRAM_NAME
  ledger = Books::Ledger.empty.tap { |l| l.add(10.0, "start") }
  begin
    ledger.add(-2.5)
  rescue Books::Error => e
    warn e.message
  ensure
    puts ledger
  end
end
