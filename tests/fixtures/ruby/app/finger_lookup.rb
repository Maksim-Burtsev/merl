# #1340: a method's last value that is a call chain, memoised or not, types its caller's receiver,
# and so does a body whose `rescue` clauses only raise or give `nil`. A clause that gives another
# value, an `ensure`, a memo whose ivar is assigned elsewhere too (here in a reopening), and a
# chain through a method the rules cannot type stay pickers.
class Finger
  class Answer
    def href
    end
  end

  class Stray
    def href
    end
  end

  def perform
    Answer.new
  rescue ArgumentError
    raise ArgumentError, 'bad'
  end

  def fallback
    Answer.new
  rescue ArgumentError
    Stray.new
  end

  def guarded
    Answer.new
  ensure
    nil
  end
end

class FingerClient
  def link
    resource&.href
    #         ^ d: app/finger_lookup.rb:7
    direct.href
    #      ^ d: app/finger_lookup.rb:7
    loose.href
    #     ^ d: picker app/finger_lookup.rb:7, app/finger_lookup.rb:12
    shielded.href
    #        ^ d: picker app/finger_lookup.rb:7, app/finger_lookup.rb:12
    unknown.href
    #       ^ d: picker app/finger_lookup.rb:7, app/finger_lookup.rb:12
    swapped.href
    #       ^ d: picker app/finger_lookup.rb:7, app/finger_lookup.rb:12
    @swapped = Finger::Stray.new
  end

  private

  def resource
    @resource ||= Finger.new.perform
  rescue ArgumentError
    nil
  end

  def direct
    Finger.new.perform
  end

  def loose
    Finger.new.fallback
  end

  def shielded
    Finger.new.guarded
  end

  def unknown
    @unknown ||= helper.perform
  end
end

class FingerClient
  def swapped
    @swapped ||= Finger.new.perform
  end
end
