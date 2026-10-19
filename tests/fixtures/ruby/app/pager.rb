# A bare `next`, `redo`, `retry` or `break` is a keyword, never a serializer's `attribute :next` (#1094).
class PagerSerializer < ActiveModel::Serializer
  attribute :next
end

class Pager
  def pages(items)
    items.each do |item|
      next if item.nil?
#     ^ d: none
      redo if item.stale?
#     ^ d: none
      break if item.last?
#     ^ d: none
    end
  rescue Timeout::Error
    retry
#   ^ d: none
  end

  def follow(page)
    page.next
#        ^ d: picker app/pager.rb:3
  end
end
