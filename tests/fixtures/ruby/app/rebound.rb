# #1340: a name typed as Ledger (`x = Ledger.new`, a YARD `@param`, a method, an `@ivar` set once)
# and bound again before the use, by a block, a `rescue`, a pattern, a multiple assignment or a
# second statement on a line, may hold a Receipt: `d` keeps the picker instead of jumping to Ledger.
class Ledger
  def reconcile
  end
end

class Receipt
  def reconcile
  end

  def lstrip
  end
end

class Rebound
  def ledger
    Ledger.new
  end

  def typed
    entry = Ledger.new
    entry.reconcile
    #     ^ d: app/rebound.rb:5
  end

  def brace(receipts)
    entry = Ledger.new
    receipts.each { |entry| entry.reconcile }
    #                             ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def block(receipts)
    entry = Ledger.new
    receipts.each_with_index do |n, entry|
      entry.reconcile
      #     ^ d: picker app/rebound.rb:5, app/rebound.rb:10
    end
  end

  def rescued
    entry = Ledger.new
    begin
      entry.reconcile
    rescue StandardError => entry
      entry.reconcile
      #     ^ d: picker app/rebound.rb:5, app/rebound.rb:10
    end
  end

  def pattern(given)
    entry = Ledger.new
    case given
    in {receipt: entry}
      entry.reconcile
      #     ^ d: picker app/rebound.rb:5, app/rebound.rb:10
    end
  end

  def multiple
    entry = Ledger.new
    n, entry = 1, Receipt.new
    entry.reconcile
    #     ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def second_statement
    entry = Ledger.new
    n = 1; entry = Receipt.new
    entry.reconcile
    #     ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def first_statement
    entry = Ledger.new
    entry = Receipt.new; entry.reconcile
    #                          ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def gate
    entry = Ledger.new
    def inner; entry.reconcile; end
    #                ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def method_shadowed(receipts)
    receipts.each { |ledger| ledger.reconcile }
    #                               ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def method_reassigned
    n = 1; ledger = Receipt.new; ledger.reconcile
    #                                   ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  # @param [Ledger] entry
  def documented(entry, receipts)
    if receipts
      entry = Receipt.new
    end
    entry.reconcile
    #     ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def core(receipts)
    label = "receipt"
    receipts.each { |label| label.lstrip }
    #                             ^ d: picker app/rebound.rb:13
  end

  def closed_on_the_line(receipts)
    entry = Ledger.new
    receipts.each { |entry| entry.lstrip }; entry.reconcile
    #                                             ^ d: app/rebound.rb:5
  end

  def closed_then_assigned(receipts)
    entry = Ledger.new
    receipts.each { |entry| entry.lstrip }; entry = Receipt.new; entry.reconcile
    #                                                                  ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def assigned_in_closed(receipts)
    entry = Ledger.new
    receipts.each { |n| entry = Receipt.new }; entry.reconcile
    #                                                ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def inside_the_block(receipts)
    entry = Ledger.new
    receipts.each { |entry| entry.reconcile }; entry.reconcile
    #                             ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def nested_blocks(receipts)
    entry = Ledger.new
    receipts.each { |entry| receipts.each { |n| n.lstrip } }; entry.reconcile
    #                                                               ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def closed_do(receipts)
    entry = Ledger.new
    receipts.each do |entry| entry.lstrip end; entry.reconcile
    #                                                ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def closed_lambda(receipts)
    entry = Ledger.new
    ->(entry) { entry.lstrip }; entry.reconcile
    #                                 ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def method_closed_on_the_line(receipts)
    receipts.each { |ledger| ledger.lstrip }; ledger.reconcile
    #                                                ^ d: app/rebound.rb:5
  end

  def either(receipts)
    entry = Ledger.new
    receipts.empty? || entry.reconcile || receipts.first
    #                        ^ d: app/rebound.rb:5
    entry.reconcile
    #     ^ d: app/rebound.rb:5
  end

  def hashed
    entry = Ledger.new
    row = { "owner" => entry.reconcile }
    #                        ^ d: app/rebound.rb:5
    entry.reconcile
    #     ^ d: app/rebound.rb:5
  end

  def controls(receipts)
    entry = Ledger.new
    # entry = Receipt.new
    label = "entry = Receipt.new"
    same = entry == receipts.first
    receipts.fetch(entry: 1)
    row = { entry: 1 }
    entry.label = Receipt.new
    receipts.each { |entry| entry.lstrip }
    entry.reconcile
    #     ^ d: app/rebound.rb:5
    entry = Receipt.new
  end
end

class Booked
  def initialize
    @entry = Ledger.new
  end

  def reconcile_it
    @entry.reconcile
    #      ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def swap
    @entry, n = Receipt.new, 1
  end
end

class Restored
  def initialize
    @entry = Ledger.new
  end

  def reconcile_it
    @entry.reconcile
    #      ^ d: picker app/rebound.rb:5, app/rebound.rb:10
  end

  def restore(saved)
    instance_variable_set(:@entry, saved)
  end
end

class Accessed
  attr_accessor :entry

  def initialize
    @entry = Ledger.new
  end

  def reconcile_it
    @entry.reconcile
    #      ^ d: app/rebound.rb:5
  end
end

top_entry = Ledger.new
def helper; end
def other_helper
end
top_entry.reconcile
#         ^ d: app/rebound.rb:5
