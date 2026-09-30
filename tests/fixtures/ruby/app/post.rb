class Post
  include Searchable

  attr_reader :title

  def initialize(title)
    @title = title
  end

  def title=(value)
    @title = value
  end

  def publish
    true
  end
end
