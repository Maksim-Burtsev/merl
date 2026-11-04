# #1340: a method parameter whose YARD `@param` names one class has that class, read directly or
# through an `@ivar` it is assigned to once. A tag naming several types, a tag apart from the
# `def`, two tags for one name, a parameter assigned again and a block parameter of the same name
# stay pickers.
class Memo
  def body
  end
end

class Draft
  def body
  end
end

class MemoMailer
  # @param [Memo] memo the memo to send
  # @param [Hash] options
  def initialize(memo, options = {})
    @memo = memo
    memo.body
    #    ^ d: app/documented_param.rb:6
    @options = options
  end

  def deliver
    @memo.body
    #     ^ d: app/documented_param.rb:6
  end

  # @param memo [Memo]
  def named_first(memo)
    memo.body
    #    ^ d: app/documented_param.rb:6
  end

  # @param [Memo, nil] memo
  def several(memo)
    memo.body
    #    ^ d: picker app/documented_param.rb:6, app/documented_param.rb:11
  end

  # @param [Array<Memo>] memo
  def wrapped(memo)
    memo.body
    #    ^ d: picker app/documented_param.rb:6, app/documented_param.rb:11
  end

  # @param [Memo] memo

  def apart(memo)
    memo.body
    #    ^ d: picker app/documented_param.rb:6, app/documented_param.rb:11
  end

  # @param [Memo] memo
  # @param [Draft] memo
  def twice(memo)
    memo.body
    #    ^ d: picker app/documented_param.rb:6, app/documented_param.rb:11
  end

  # @param [Memo] memo
  def reassigned(memo)
    memo = Draft.new if memo.nil?
    memo.body
    #    ^ d: picker app/documented_param.rb:6, app/documented_param.rb:11
  end

  # @param [Memo] memo
  def shadowed(memo, drafts)
    drafts.each { |memo| memo.body }
    #                         ^ d: picker app/documented_param.rb:6, app/documented_param.rb:11
  end

  # @param [Memo] memo
  def self.build(memo)
    memo.body
    #    ^ d: app/documented_param.rb:6
  end
end
