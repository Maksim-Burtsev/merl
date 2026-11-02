# #1340: what a record's `find` returns is an instance of the record, through an `@ivar` the class
# assigns once and through a method whose last value is it. A `find` of a class that is no record,
# or defines its own, an ivar assigned twice, and a parameter of the method's name type nothing;
# `@other` is not what the method `other` returns.
class MemberRecord < ActiveRecord::Base
  def promote!
  end
end

class Lookalike
  def self.find(id)
    id
  end

  def promote!
  end
end

class PromoteWorker
  def perform(id)
    @member = MemberRecord.find(id)
    @member.promote!
    #       ^ d: app/record_lookup.rb:6
    member.promote!
    #      ^ d: app/record_lookup.rb:6
    @other = Lookalike.find(id)
    @other.promote!
    #      ^ d: picker app/record_lookup.rb:6, app/record_lookup.rb:15
    @twice = MemberRecord.find(id)
    @twice.promote!
    #      ^ d: picker app/record_lookup.rb:6, app/record_lookup.rb:15
  end

  def again(member)
    @twice = Lookalike.new
    member.promote!
    #      ^ d: picker app/record_lookup.rb:6, app/record_lookup.rb:15
  end

  def self.lookup
    member.promote!
    #      ^ d: picker app/record_lookup.rb:6, app/record_lookup.rb:15
  end

  private

  def member
    @member_memo ||= MemberRecord.find(@id)
  end

  def other
    MemberRecord.find(@id)
  end
end

class SweepWorker
  @shared = MemberRecord.find(1)

  def self.prepare(id)
    @prepared = MemberRecord.find(id)
  end

  def sweep
    @shared.promote!
    #       ^ d: picker app/record_lookup.rb:6, app/record_lookup.rb:15
    @prepared.promote!
    #         ^ d: picker app/record_lookup.rb:6, app/record_lookup.rb:15
  end
end
